pub mod worker;

use log::info;

use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use std::sync::{Arc, Mutex};
use std::thread;
use rand::Rng;
use ring::{digest};

use crate::types::block::{Header, Content, Block};
use crate::types::transaction::{SignedTransaction};
use crate::types::hash::{H256, Hashable};
use crate::blockchain::Blockchain;
use crate::types::merkle::MerkleTree;
use std::collections::HashMap;
use crate::types::address::Address;

enum ControlSignal {
    Start(u64), // the number controls the lambda of interval between block generation
    Update, // update the block in mining, it may due to new blockchain tip or new transaction
    Exit,
}

enum OperatingState {
    Paused,
    Run(u64),
    ShutDown,
}

pub struct Context {
    /// Channel for receiving control signal
    control_chan: Receiver<ControlSignal>,
    operating_state: OperatingState,
    finished_block_chan: Sender<Block>,
    blockchain: Arc<Mutex<Blockchain>>,
    mempool: Arc<Mutex<HashMap<H256, SignedTransaction>>>,
    states: Arc<Mutex<HashMap<H256 ,HashMap<Address, (u64, u64)>>>>
}

#[derive(Clone)]
pub struct Handle {
    /// Channel for sending signal to the miner thread
    control_chan: Sender<ControlSignal>,
}

pub fn new(blockchain: &Arc<Mutex<Blockchain>>, mempool: &Arc<Mutex<HashMap<H256, SignedTransaction>>>, states: &Arc<Mutex<HashMap<H256 ,HashMap<Address, (u64, u64)>>>>) -> (Context, Handle, Receiver<Block>) {
    let (signal_chan_sender, signal_chan_receiver) = unbounded();
    let (finished_block_sender, finished_block_receiver) = unbounded();

    let ctx = Context {
        control_chan: signal_chan_receiver,
        operating_state: OperatingState::Paused,
        finished_block_chan: finished_block_sender,
        blockchain: Arc::clone(blockchain),
        mempool: Arc::clone(mempool),
        states: Arc::clone(states)
    };

    let handle = Handle {
        control_chan: signal_chan_sender,
    };

    (ctx, handle, finished_block_receiver)
}

#[cfg(any(test,test_utilities))]
fn test_new() -> (Context, Handle, Receiver<Block>) { // CORRECT????
    // new blockchain
    let b_chain: Blockchain = Blockchain::new();
    let blockchain = Arc::new(Mutex::new(b_chain));
    
    let mempool: Arc<Mutex<HashMap<H256, SignedTransaction>>> = Arc::new(Mutex::new(HashMap::new())); 
    let states: Arc<Mutex<HashMap<H256 ,HashMap<Address, (u64, u64)>>>> = Arc::new(Mutex::new(HashMap::new()));
    new(&blockchain, &mempool, &states)
}

impl Handle {
    pub fn exit(&self) {
        self.control_chan.send(ControlSignal::Exit).unwrap();
    }

    pub fn start(&self, lambda: u64) {
        self.control_chan
            .send(ControlSignal::Start(lambda))
            .unwrap();
    }

    pub fn update(&self) {
        self.control_chan.send(ControlSignal::Update).unwrap();
    }
}

impl Context {
    pub fn start(mut self) {
        thread::Builder::new()
            .name("miner".to_string())
            .spawn(move || {
                self.miner_loop();
            })
            .unwrap();
        info!("Miner initialized into paused mode");
    }

    fn miner_loop(&mut self) {
        // main mining loop
        let mut ctx = digest::Context::new(&digest::SHA256);
        ctx.update("genesis".as_ref());
        let rand_hash = H256::from(ctx.finish());
        let mut parent: H256 = rand_hash;

        if self.blockchain.lock().unwrap().blocks.len() >= 1 {
            parent = self.blockchain.lock().unwrap().tip();
        }
        println!("Initial Parent: {:?}", parent);
        
        loop {
            // check and react to control signals
            match self.operating_state {
                OperatingState::Paused => {
                    let signal = self.control_chan.recv().unwrap();
                    match signal {
                        ControlSignal::Exit => {
                            info!("Miner shutting down");
                            self.operating_state = OperatingState::ShutDown;
                        }
                        ControlSignal::Start(i) => {
                            info!("Miner starting in continuous mode with lambda {}", i);
                            self.operating_state = OperatingState::Run(i);
                        }
                        ControlSignal::Update => {
                            // in paused state, don't need to update
                        }
                    };
                    continue;
                }
                OperatingState::ShutDown => {
                    return;
                }
                _ => match self.control_chan.try_recv() {
                    Ok(signal) => {
                        match signal {
                            ControlSignal::Exit => {
                                info!("Miner shutting down");
                                self.operating_state = OperatingState::ShutDown;
                            }
                            ControlSignal::Start(i) => {
                                info!("Miner starting in continuous mode with lambda {}", i);
                                self.operating_state = OperatingState::Run(i);
                            }
                            ControlSignal::Update => {
                                let blockchain_guard = self.blockchain.lock().unwrap();
                                parent = blockchain_guard.tip();
                                std::mem::drop(blockchain_guard);
                            }
                        };
                    }
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => panic!("Miner control channel detached"),
                },
            }
            if let OperatingState::ShutDown = self.operating_state {
                return;
            }

            // TODO for student: actual mining, create a block
            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_millis();

            // Rand Nonce
            let mut rng = rand::thread_rng();
            let rand_nonce: u32 = rng.gen();   // Generate random value for nonce
            
            let tx_vec: Vec<SignedTransaction> = Vec::new();
            let merkle_tree = MerkleTree::new(&tx_vec);
            let root = merkle_tree.root();

            // let blockchain_guard = self.blockchain.lock().unwrap();
            //println!("Parent: {:?} BLOCKS ACQUIRED VIA LOCK: {:?}", parent, blockchain_guard.blocks);
            // let difficulty: H256 = blockchain_guard.blocks[&parent].header.difficulty;
            let difficulty:H256 = [0u8,0u8,255u8,255u8,1u8,1u8,0u8,0u8,0u8,0u8,0u8,1u8,0u8,0u8,0u8,0u8,1u8,0u8,0u8,0u8,0u8,0u8,0u8,0u8,0u8,0u8,0u8,0u8,0u8,0u8,0u8,0u8].into();
            // std::mem::drop(blockchain_guard);

            let header = Header {
                parent: parent,
                timestamp: timestamp,
                difficulty: difficulty,
                merkle_root: root,
                nonce: rand_nonce                
            };
            let mempool_guard = self.mempool.lock().unwrap();
            let num_transactions = std::cmp::min(mempool_guard.len(), 30);
            //println!("MEMPOOL LEN: {:?}", mempool_guard.len());
            let mut txs_data: Vec<SignedTransaction> = Vec::new(); 
            let mut idx = 0; 
            let mut curr_state = self.states.lock().unwrap()[&parent].clone();

            for k in mempool_guard.keys() {
                let elem = mempool_guard[k].clone();
                if idx < num_transactions {
                    if curr_state.contains_key(&elem.t.sender) {
                        let (s_nonce, bal) = curr_state[&elem.t.sender];
                        //println!("ACC_NONCE: {:?} & VAL: {:?}", elem.t.acc_nonce, elem.t.value);
                        if s_nonce + 1 == elem.t.acc_nonce && bal >= elem.t.value {
                            //println!("s_nonce + 1 == elem.t.acc_nonce && bal >= elem.t.value ");
                            curr_state.insert(
                                elem.t.sender, (s_nonce + 1, bal - elem.t.value)
                            );

                            if curr_state.contains_key(&elem.t.receiver) {
                                let (r_nonce, bal) = curr_state[&elem.t.receiver];
                                curr_state.insert(elem.t.receiver, (r_nonce + 1, bal + elem.t.value));
                            } else {
                                curr_state.insert(
                                    elem.t.receiver, (1, elem.t.value)
                                );
                            }
                            txs_data.push(elem);
                            idx += 1;
                        }
                    } else {
                        // Do nothing
                    }
                } else {
                    break;
                }
            }
            //println!("TXS DATA: {:?}", txs_data);
            std::mem::drop(mempool_guard);
            
            let block = Block{header: header, content: Content {data: txs_data}, height: 0}; // Content should be the transactions in the mempool.
            
            
            // TODO for student: if block mining finished, you can have something like: self.finished_block_chan.send(block.clone()).expect("Send finished block error");
            if block.hash() <= difficulty {
                //println!("PRINT STATE UPDATE\n");
                let mut blockchain_guard = self.blockchain.lock().unwrap();
                self.states.lock().unwrap().insert(block.hash(), curr_state);
                blockchain_guard.insert(&block);
                self.finished_block_chan.send(block.clone()).expect("Send finished block error");
                println!("Parent: {:?}. B-Hash: {:?}, TX-Data Len: {:?}", parent, block.hash(), block.content.data.len()); // PRINT PARENT
                parent = blockchain_guard.tip();
                std::mem::drop(blockchain_guard);
            }
            if let OperatingState::Run(i) = self.operating_state {
                if i != 0 {
                    let interval = Duration::from_micros(i as u64);
                    thread::sleep(interval);
                }
            }
        }
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

#[cfg(test)]
mod test {
    use ntest::timeout;
    use crate::types::hash::Hashable;

    #[test]
    #[timeout(60000)]
    fn miner_three_block() {
        let (miner_ctx, miner_handle, finished_block_chan) = super::test_new();
        miner_ctx.start();
        miner_handle.start(0);
        let mut block_prev = finished_block_chan.recv().unwrap();
        for _ in 0..2 {
            let block_next = finished_block_chan.recv().unwrap();
            assert_eq!(block_prev.hash(), block_next.get_parent());
            block_prev = block_next;
        }
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST