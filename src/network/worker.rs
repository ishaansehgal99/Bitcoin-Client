use super::message::Message;
use super::peer;
use super::server::Handle as ServerHandle;
use crate::types::hash::{H256, Hashable};
use crate::types::transaction::{Transaction, SignedTransaction, st_verify};
use std::sync::{Arc, Mutex};
use crate::blockchain::Blockchain;
use crate::types::block::Block;
use std::collections::HashMap;
use crate::types::address::Address;
extern crate queues;
use queues::*;
use ring::{digest};
use std::time::{SystemTime, UNIX_EPOCH, Duration};


use log::{debug, warn, error};

use std::thread;

#[cfg(any(test,test_utilities))]
use super::peer::TestReceiver as PeerTestReceiver;
#[cfg(any(test,test_utilities))]
use super::server::TestReceiver as ServerTestReceiver;

#[derive(Clone)]
pub struct State {
    account_addr: Address,
    account_nonce: u64,
    balance: u64
}

#[derive(Clone)]
pub struct Worker {
    msg_chan: smol::channel::Receiver<(Vec<u8>, peer::Handle)>,
    num_worker: usize,
    server: ServerHandle,
    blockchain: Arc<Mutex<Blockchain>>,
    orphans_map: Arc<Mutex<HashMap<H256, Vec<Block>>>>,
    mempool: Arc<Mutex<HashMap<H256, SignedTransaction>>>,
    states: Arc<Mutex<HashMap<H256 ,HashMap<Address, (u64, u64)>>>>, // given block hash, returns state
    peers: Arc<Mutex<Vec<Address>>>, 
    // node_start_time: SystemTime, 
    public_addr: Address
}


impl Worker {
    pub fn new(
        num_worker: usize,
        msg_src: smol::channel::Receiver<(Vec<u8>, peer::Handle)>,
        server: &ServerHandle,
        blockchain: &Arc<Mutex<Blockchain>>, 
        orphans_map: &Arc<Mutex<HashMap<H256, Vec<Block>>>>,
        mempool: &Arc<Mutex<HashMap<H256, SignedTransaction>>>,
        states: &Arc<Mutex<HashMap<H256 ,HashMap<Address, (u64, u64)>>>>,
        peers: &Arc<Mutex<Vec<Address>>>, 
        // node_start_time: &SystemTime, 
        public_addr: &Address
    ) -> Self {
        Self {
            msg_chan: msg_src,
            num_worker,
            server: server.clone(),
            blockchain: Arc::clone(blockchain),
            orphans_map: Arc::clone(orphans_map),
            mempool: Arc::clone(mempool),
            states: Arc::clone(states),
            peers: Arc::clone(peers), 
            // node_start_time: node_start_time.clone(), 
            public_addr: public_addr.clone()
        }
    }

    pub fn start(self) {
        let num_worker = self.num_worker;
        for i in 0..num_worker {
            let cloned = self.clone();
            thread::spawn(move || {
                cloned.worker_loop();
                warn!("Worker thread {} exited", i);
            });
        }
    }

    fn worker_loop(&self) {
        loop {
            let result = smol::block_on(self.msg_chan.recv());
            if let Err(e) = result {
                error!("network worker terminated {}", e);
                break;
            }
            let msg = result.unwrap();
            let (msg, mut peer) = msg;
            let msg: Message = bincode::deserialize(&msg).unwrap();
            match msg {
                Message::Ping(nonce) => {
                    debug!("Ping: {}", nonce);
                    peer.write(Message::Pong(nonce.to_string()));
                }
                Message::Pong(nonce) => {
                    debug!("Pong: {}", nonce);
                }
                // Check if new hashes are already in our blockchain
                // If they are not send out a get blocks message with the block hashes needed
                Message::NewBlockHashes(hashes) => {
                    println!("NEW BLOCK HASH MESSAGE RECEIEVED"); 
                    let mut missing_hashes: Vec<H256> = Vec::new();
                    let blockchain_guard = self.blockchain.lock().unwrap();
                    for hash in hashes {
                        if blockchain_guard.blocks.contains_key(&hash) {
                            continue;
                        } else {
                            missing_hashes.push(hash); 
                        }
                    }
                    std::mem::drop(blockchain_guard);
                    println!("{:?} missing block hashes, length: {:?}", missing_hashes, missing_hashes.len()); 
                    if missing_hashes.len() > 0 {
                        peer.write(Message::GetBlocks(missing_hashes));
                    }
                }
                // If the hashes are in blockchain, you can get these blocks 
                // And reply them by Blocks message.
                Message::GetBlocks(hashes) => {
                    println!("GET BLOCKS MESSAGE RECEIEVED"); 
                    let mut blocks: Vec<Block> = Vec::new();
                    let blockchain_guard = self.blockchain.lock().unwrap();
                    for hash in hashes {
                        if blockchain_guard.blocks.contains_key(&hash) {
                            blocks.push(blockchain_guard.blocks[&hash].clone());
                        }
                    }
                    std::mem::drop(blockchain_guard);
                    if blocks.len() > 0 {
                        peer.write(Message::Blocks(blocks));
                    }
                    //println!("SENT BLOCKS: {:?}", blocks);
                }
                // Insert the blocks into blockchain if not already in it. 
                // Also if the blocks are new to this node, 
                // you need to make a broadcast of a NewBlockHashes message. 
                // NewBlockHashes message should contain hashes of blocks newly received.
                Message::Blocks(blocks) => {
                    println!("BLOCKS MESSAGE RECEIEVED"); 
                    let mut blockchain_guard = self.blockchain.lock().unwrap();
                    let mut new_block_hashes: Vec<H256> = Vec::new();
                    let mut missing_parents: Vec<H256> = Vec::new();

                    for block in blocks {
                        if blockchain_guard.blocks.contains_key(&block.hash()) {
                            continue;
                        } else {
                            if blockchain_guard.blocks.contains_key(&block.header.parent) {
                                // Any time we insert a block that is in our chain, 
                                // recurisvely check in our orphans map, 
                                // if any blocks parents are the block that got inserted
                                let mut queue: Queue<Block> = queue![block]; 
                                while queue.size() > 0 {
                                    let curr_block: Block = queue.remove().unwrap();

                                    if curr_block.hash() <= blockchain_guard.blocks[&curr_block.header.parent].header.difficulty 
                                    && curr_block.header.difficulty == blockchain_guard.blocks[&curr_block.header.parent].header.difficulty {
                                        // Check all blocks transactions are not in our mempool 
                                        let mut failed_block = false;
                                        let mut valid_txs = Vec::new();
                                        
                                        let curr_block_data: Vec<SignedTransaction> = curr_block.content.data.clone(); 
                                        let mut curr_state = self.states.lock().unwrap()[&curr_block.header.parent].clone();

                                        for tx in curr_block_data {
                                            if st_verify(&tx) {
                                                let h = digest::digest(&digest::SHA256, tx.pub_key.as_ref());
                                                let hex_h = hex::encode(h).into_bytes();
                                                
                                                if tx.t.sender == Address::from_public_key_bytes(&hex_h) {
                                                    // let mut curr_state = self.states.lock().unwrap()[&curr_block.header.parent].clone();
                                                    if curr_state.contains_key(&tx.t.sender) {
                                                        let (s_nonce, bal) = curr_state[&tx.t.sender];
                                                        if (s_nonce+1 == tx.t.acc_nonce) && (bal >= tx.t.value) { // Spending check
                                                            curr_state.insert(
                                                                tx.t.sender, (s_nonce + 1, bal - tx.t.value)
                                                            );

                                                            if curr_state.contains_key(&tx.t.receiver) {
                                                                let (r_nonce, bal) = curr_state[&tx.t.receiver];
                                                                curr_state.insert(tx.t.receiver, (r_nonce + 1, bal + tx.t.value));
                                                            } else {

                                                                curr_state.insert(
                                                                    tx.t.receiver, (1, tx.t.value)
                                                                );
                                                            }
                                                            valid_txs.push(tx.clone());
                                                        } else {
                                                            failed_block = true; 
                                                            break;
                                                        }
                                                        
                                                    } else {
                                                        failed_block = true; 
                                                        break;
                                                    }
                                                } else {
                                                    failed_block = true; 
                                                    break;
                                                } 
                                                
                                            } else {
                                                failed_block = true; 
                                                break;
                                            }
                                        }

                                        if !failed_block {
                                            println!("INSERTING BLOCK {:?}, TXS: {:?}", curr_block.hash(), curr_block.content.data.len());
                                            blockchain_guard.insert(&curr_block);
                                            println!("PRINT STATE UPDATING BLOCKS\n");
                                            self.states.lock().unwrap().insert(curr_block.hash(), curr_state);
                                             // Remove transactions from our mempool
                                            for tx in valid_txs {
                                                let mut mempool_guard = self.mempool.lock().unwrap();
                                                if mempool_guard.contains_key(&tx.hash()) {
                                                    mempool_guard.remove(&tx.hash());
                                                } else {
                                                    //println!("TRANSACTION HASH NOT IN THE MEMPOOL: {:?}", &tx.hash());
                                                }
                                                std::mem::drop(mempool_guard);
                                            }
                                        } else {
                                            continue;
                                        }
                                        
                                        new_block_hashes.push(curr_block.hash());
                                    
                                        let mut orphans_map_guard = self.orphans_map.lock().unwrap();
                                        if orphans_map_guard.contains_key(&curr_block.hash()) {
                                            let orphans_vec: Vec<Block> = orphans_map_guard[&curr_block.hash()].clone(); 
                                            for elem in orphans_vec {
                                                let new_block: Block = elem;
                                                println!("ADDING ORPHAN, ORPHAN TO BE ADDED TO BC: {:?}", new_block);
                                                queue.add(new_block);
                                            }
                                            // TODO: Remove block from orphans map
                                            orphans_map_guard.remove(&curr_block.hash());
                                            std::mem::drop(orphans_map_guard);
                                        }  
                                    } 
                                }
                                
                                
                            } else {
                                // Parent -> Child orphans map
                                    // If parent not in map 
                                    // Add new block to map with parent as key, block as value
                                let mut orphans_map_guard = self.orphans_map.lock().unwrap();

                                let block_parent = block.header.parent;
                                
                                if orphans_map_guard.contains_key(&block.header.parent){
                                    let mut orphan_vec: Vec<Block> = orphans_map_guard[&block_parent].clone();
                                    orphan_vec.push(block); 
                                    orphans_map_guard.insert(block_parent, orphan_vec);
                                } else {
                                    let mut vec: Vec<Block> = Vec::new(); 
                                    vec.push(block);
                                    orphans_map_guard.insert(block_parent, vec);
                                }   
                               
                                // Getblocks? 
                                missing_parents.push(block_parent);
                                std::mem::drop(orphans_map_guard);

                            }
                        }
                    }

                    peer.write(Message::GetBlocks(missing_parents));

                    // println!("{:?} ENTIRE NODES BLOCKCHAIN", blockchain_guard.blocks.keys());
                    std::mem::drop(blockchain_guard);
                    if new_block_hashes.len() > 0 {
                        println!("MISSING BLOCK HASHES: {:?}", new_block_hashes.len());
                        self.server.broadcast(Message::NewBlockHashes(new_block_hashes));
                    }
                }
                Message::NewTransactionHashes(tx_hashes) => {
                    println!("NEW TXS HASH MESSAGE RECEIEVED");
                    println!("DEBUG STATEMENT");
                    let mut missing_hashes: Vec<H256> = Vec::new();
                    let mempool_guard = self.mempool.lock().unwrap();
                    println!("DEBUG: Taken mempool mutex in NEWTXHASH MSG");
                    for hash in tx_hashes {
                        if mempool_guard.contains_key(&hash) {
                            continue;
                        } else {
                            missing_hashes.push(hash); 
                        }
                    }
                    std::mem::drop(mempool_guard);
                    println!("{:?} missing tx hashes, length: {:?}", missing_hashes, missing_hashes.len()); 
                    if missing_hashes.len() > 0 {
                        peer.write(Message::GetTransactions(missing_hashes));
                    }
                }
                Message::GetTransactions(tx_hashes) => {
                    println!("GET TXS MESSAGE RECEIEVED"); 
                    let mut signed_txs: Vec<SignedTransaction> = Vec::new();
                    let mempool_guard = self.mempool.lock().unwrap();
                    for hash in tx_hashes {
                        if mempool_guard.contains_key(&hash) {
                            signed_txs.push(mempool_guard[&hash].clone());
                        }
                    }
                    std::mem::drop(mempool_guard);
                    if signed_txs.len() > 0 {
                        peer.write(Message::Transactions(signed_txs));
                    }
                }
                Message::Transactions(txs) => {
                    println!("TX MESSAGE RECEIEVED");
                    let mut new_tx_hashes: Vec<H256> = Vec::new();
                    for tx in txs {
                        // Verify that the tx is signed correctly
                        if st_verify(&tx) {
                            
                            let blockchain_guard = self.blockchain.lock().unwrap();
                            let h = digest::digest(&digest::SHA256, tx.pub_key.as_ref());
                            let hex_h = hex::encode(h).into_bytes();

                            if tx.t.sender == Address::from_public_key_bytes(&hex_h) { // Check if the public key matches the owner's address of the withdrawing account
                                let curr_state = self.states.lock().unwrap()[&blockchain_guard.tip()].clone();
                                println!("curr_state: {:?}", curr_state);
                                if curr_state.contains_key(&tx.t.sender) {
                                    println!("CURR STATE CONTAINS SENDER ADDR");
                                    let (s_nonce, bal) = curr_state[&tx.t.sender];
                                    if (s_nonce+1 >= tx.t.acc_nonce) && (bal >= tx.t.value) { // Check if the balance is enough and the suggested account nonce is equal to one plus the account nonce                                        
                                        let mut mempool_guard = self.mempool.lock().unwrap();
                                        let tx_hash = tx.hash();
                                        mempool_guard.insert(tx_hash, tx);
                                        new_tx_hashes.push(tx_hash);
                                        std::mem::drop(mempool_guard);
                                    }
                                }
                            }
                        }
                    }
                    if new_tx_hashes.len() > 0 {
                        self.server.broadcast(Message::NewTransactionHashes(new_tx_hashes));
                    }                  
                }
                // A new node coming online has asked the node it connects to for its state
                Message::RequestState(peer_addr) => {
                    // We add the requesting node's Address to our peers vec
                    self.peers.lock().unwrap().push(peer_addr);
                    // We send the node our state and our node's Address
                    let our_state = self.states.lock().unwrap().clone();
                    println!("Sending State");
                    peer.write(Message::RespondState(our_state, self.public_addr));
                }
                // Node responds to new node's request for state
                Message::RespondState(requested_state, peer_addr) => {
                    // Add the responding nodes Address to our peers vec
                    self.peers.lock().unwrap().push(peer_addr);

                    // Set our state equal to the requested state
                    let mut states_guard = self.states.lock().unwrap();
                    states_guard.clear();

                    // Copy of requested_state
                    let mut copy_requested_state:HashMap<H256, HashMap<Address, (u64, u64)>> = HashMap::new();
                    for (key, value) in requested_state.into_iter() {
                        let val_c = value.clone();
                        states_guard.insert(key, value);
                        copy_requested_state.insert(key, val_c);
                    }
                    std::mem::drop(states_guard);
                    println!("Receiving State");
                    println!("Current Peers {:?}", self.peers.lock().unwrap()); 
                    // let c_requested_state = requested_state.clone();
                    self.server.sendToEveryoneButMeAndWhereThisMsgCameFrom(peer, Message::RespondState(copy_requested_state, peer_addr));
                }
                // Message::InitialState(time, addr) => {
                //     println!("Initial State Message RECEIEVED");
                //     println!("{:?}, {:?}, {:?}", addr, time, self.node_start_time); 
                    

                //     let received_time = time.duration_since(UNIX_EPOCH).expect("Time went backwards");
                //     let our_time = self.node_start_time.duration_since(UNIX_EPOCH).expect("Time went backwards"); 

                //     let recieved_time_ns = received_time.as_nanos();
                //     let our_time_ns = our_time.as_nanos();

                //     println!("{:?}", recieved_time_ns < our_time_ns); 
                   
                //     let blockchain_guard = self.blockchain.lock().unwrap();
                //     let mut peers_guard = self.peers.lock().unwrap();
                    
                //     if !peers_guard.contains(&addr) {
                //         peers_guard.push(addr); 
                //         if recieved_time_ns < our_time_ns {
                //             let mut curr_guard = self.states.lock().unwrap(); 
                //             let mut curr_state = curr_guard[&blockchain_guard.get_genesis_block_hash()].clone(); 
                //             curr_state.clear(); 
                //             curr_state.insert(addr, (0, 50));
                //             curr_guard.insert(blockchain_guard.get_genesis_block_hash(), curr_state);
                //             std::mem::drop(curr_guard);
                //         } else {
                //             // This address may not be associated with this timestamp and so could cause errors
                //             // let mut our_addr: Address = [0u8,0u8,255u8,255u8,1u8,1u8,0u8,0u8,0u8,0u8,0u8,1u8,0u8,0u8,0u8,0u8,1u8,0u8,0u8,0u8].into();
                //             // for a in self.states.lock().unwrap()[&blockchain_guard.get_genesis_block_hash()].keys() {
                //             //     our_addr = *a;
                //             // }
                //             peer.write(Message::InitialState(self.node_start_time, self.public_addr));
                //         }
                //     }

                //     std::mem::drop(peers_guard);
                //     std::mem::drop(blockchain_guard);
                // }
                _ => unimplemented!(),
            }
        }
    }
}

#[cfg(any(test,test_utilities))]
struct TestMsgSender {
    s: smol::channel::Sender<(Vec<u8>, peer::Handle)>
}
#[cfg(any(test,test_utilities))]
impl TestMsgSender {
    fn new() -> (TestMsgSender, smol::channel::Receiver<(Vec<u8>, peer::Handle)>) {
        let (s,r) = smol::channel::unbounded();
        (TestMsgSender {s}, r)
    }

    fn send(&self, msg: Message) -> PeerTestReceiver {
        let bytes = bincode::serialize(&msg).unwrap();
        let (handle, r) = peer::Handle::test_handle();
        smol::block_on(self.s.send((bytes, handle))).unwrap();
        r
    }
}
#[cfg(any(test,test_utilities))]
/// returns two structs used by tests, and an ordered vector of hashes of all blocks in the blockchain
fn generate_test_worker_and_start() -> (TestMsgSender, ServerTestReceiver, Vec<H256>) {
    let (server, server_receiver) = ServerHandle::new_for_test();
    let (test_msg_sender, msg_chan) = TestMsgSender::new();
    // new blockchain
    let b_chain: Blockchain = Blockchain::new();
    let blockchain = Arc::new(Mutex::new(b_chain));
    let orphans_map:Arc<Mutex<HashMap<H256, Vec<Block>>>> = Arc::new(Mutex::new(HashMap::new()));
    let mempool: Arc<Mutex<HashMap<H256, SignedTransaction>>> = Arc::new(Mutex::new(HashMap::new()));
    let states: Arc<Mutex<HashMap<H256 ,HashMap<Address, (u64, u64)>>>> = Arc::new(Mutex::new(HashMap::new()));
     let peers: Arc<Mutex<Vec<Address>>> = Arc::new(Mutex::new(Vec::new()));
    // let node_start_time: SystemTime = SystemTime::now();
     let public_addr: Address = [0u8,0u8,255u8,255u8,1u8,1u8,0u8,0u8,0u8,0u8,0u8,1u8,0u8,0u8,0u8,0u8,1u8,0u8,0u8,0u8].into();

    let worker = Worker::new(1, msg_chan, &server, &blockchain, &orphans_map, &mempool, &states, &peers,/*, &node_start_time*/ &public_addr);
    worker.start(); 
    let vec = blockchain.lock().unwrap().all_blocks_in_longest_chain();
    (test_msg_sender, server_receiver, vec)
}


// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

#[cfg(test)]
mod test {
    use ntest::timeout;
    use crate::types::block::generate_random_block;
    use crate::types::hash::Hashable;

    use super::super::message::Message;
    use super::generate_test_worker_and_start;

    #[test]
    #[timeout(60000)]
    fn reply_new_block_hashes() {
        let (test_msg_sender, _server_receiver, v) = generate_test_worker_and_start();
        let random_block = generate_random_block(v.last().unwrap());
        let mut peer_receiver = test_msg_sender.send(Message::NewBlockHashes(vec![random_block.hash()]));
        let reply = peer_receiver.recv();
        if let Message::GetBlocks(v) = reply {
            assert_eq!(v, vec![random_block.hash()]);
        } else {
            panic!();
        }
    }
    #[test]
    #[timeout(60000)]
    fn reply_get_blocks() {
        let (test_msg_sender, _server_receiver, v) = generate_test_worker_and_start();
        let h = v.last().unwrap().clone();
        let mut peer_receiver = test_msg_sender.send(Message::GetBlocks(vec![h.clone()]));
        let reply = peer_receiver.recv();
        if let Message::Blocks(v) = reply {
            assert_eq!(1, v.len());
            assert_eq!(h, v[0].hash())
        } else {
            panic!();
        }
    }
    #[test]
    #[timeout(60000)]
    fn reply_blocks() {
        let (test_msg_sender, server_receiver, v) = generate_test_worker_and_start();
        let random_block = generate_random_block(v.last().unwrap());
        let mut _peer_receiver = test_msg_sender.send(Message::Blocks(vec![random_block.clone()]));
        let reply = server_receiver.recv().unwrap();
        if let Message::NewBlockHashes(v) = reply {
            assert_eq!(v, vec![random_block.hash()]);
        } else {
            panic!();
        }
    }

    #[test]
    #[timeout(60000)]
    fn reply_a_fuck_ton() {
        let (test_msg_sender_1, server_receiver_1, v_1) = generate_test_worker_and_start();
        let (test_msg_sender_2, server_receiver_2, v_2) = generate_test_worker_and_start();

        // Worker #1 Generates two blocks in a chain
        let random_block_1 = generate_random_block(v_1.last().unwrap());
        let random_block_2 = generate_random_block(&random_block_1.hash());

        let random_block_1_hash = random_block_1.hash();
        
        let mut peer_receiver = test_msg_sender_2.send(Message::NewBlockHashes(vec![random_block_1_hash]));
        let mut reply = peer_receiver.recv();
        if let Message::GetBlocks(v) = reply {
            assert_eq!(v, vec![random_block_1_hash]);
        } else {
            panic!();
        }

        peer_receiver = test_msg_sender_2.send(Message::Blocks(vec![random_block_1]));
        reply = server_receiver_1.recv().unwrap();
        if let Message::NewBlockHashes(something) = reply {
            assert_eq!(something, vec![random_block_1_hash]);
        } else {
            panic!();
        }

        peer_receiver = test_msg_sender_2.send(Message::NewBlockHashes(vec![random_block_1_hash, random_block_2.hash()]));
        let reply = peer_receiver.recv();
        if let Message::GetBlocks(v) = reply {
            assert_eq!(v, vec![random_block_2.hash()]);
        } else {
            panic!();
        }
        

        


        // Send one of the blocks to worker #2
        // let h = v_1.last().unwrap().clone();
        // let mut peer_receiver = test_msg_sender_2.send(Message::GetBlocks(vec![h.clone()]));
        // let reply = peer_receiver.recv();
        // if let Message::Blocks(something) = reply {
        //     assert_eq!(1, something.len());
        //     assert_eq!(h, something[0].hash())
        // } else {
        //     panic!();
        // }




        // Worker #1 sends out two new blocks 
        // let mut peer_receiver = test_msg_sender_1
        //     .send(Message::NewBlockHashes(vec![random_block_1.hash(), random_block_2.hash()]));
        

        // Worker #2 receives two new blocks
        // let worker_2_recieve = server_receiver_2.recv();
        // let mut rand = Vec::new();
        // if let Message::NewBlockHashes(rand) = worker_2_recieve {
        //     assert_eq!(something, vec![random_block_1.hash(), random_block_2.hash()]);
        // } else {
        //     panic!();
        // }
        
        // Worker #1 receieves a get blocks message
        // let reply = peer_receiver.recv();

        // if let Message::GetBlocks(something) = reply {
        //     assert_eq!(something, vec![random_block_1.hash(), random_block_2.hash()]);
        // } else {
        //     panic!();
        // }
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST