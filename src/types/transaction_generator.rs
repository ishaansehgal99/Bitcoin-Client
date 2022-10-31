use log::{debug, info};
use std::thread;
use crate::network::server::Handle as ServerHandle;
use crate::network::message::Message;
use crate::types::transaction::{SignedTransaction, Transaction, sign};
use crate::types::address::Address;
use crate::types::hash::{H256, Hashable};
use crate::types::key_pair;
use serde::{Serialize,Deserialize};
use ring::signature::{self, Ed25519KeyPair, Signature, KeyPair, VerificationAlgorithm, EdDSAParameters};
use ring::{digest};
use rand::Rng;
use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use std::time::{SystemTime};
use std::sync::{Arc, Mutex};
use crate::time;
use std::collections::HashMap;
use crate::blockchain::Blockchain;

// use std::fs::File;
// use std::fs::OpenOptions;
// use std::io::{BufRead, BufReader};

use rand::seq::SliceRandom;


#[derive(Clone)]
pub struct TransactionGenerator {
    server: ServerHandle, 
    mempool: Arc<Mutex<HashMap<H256, SignedTransaction>>>,
    key: Arc<Mutex<Ed25519KeyPair>>,
    states: Arc<Mutex<HashMap<H256 ,HashMap<Address, (u64, u64)>>>>,
    blockchain: Arc<Mutex<Blockchain>>,
    public_addr: Address,
    // node_start_time: SystemTime,
    peers: Arc<Mutex<Vec<Address>>>,
}

impl TransactionGenerator {

    pub fn new(
        server: &ServerHandle,
        mempool: &Arc<Mutex<HashMap<H256, SignedTransaction>>>,
        key: &Arc<Mutex<Ed25519KeyPair>>,
        states: &Arc<Mutex<HashMap<H256, HashMap<Address, (u64, u64)>>>>,
        blockchain: &Arc<Mutex<Blockchain>>,
        public_addr: &Address,
        // node_start_time: &SystemTime,
        peers: &Arc<Mutex<Vec<Address>>>,
    ) -> Self {
        Self {
            server: server.clone(),
            mempool: Arc::clone(mempool),
            key: Arc::clone(key),
            states: Arc::clone(states),
            blockchain: Arc::clone(blockchain),
            public_addr: public_addr.clone(),
            // node_start_time: node_start_time.clone(),
            peers: Arc::clone(peers),
        }
    }

    pub fn start(mut self, theta: u64) {
        thread::Builder::new()
        .name("transacation-generator".to_string())
        .spawn(move || {
            self.transaction_loop(3*theta);
        })
        .unwrap();
        info!("Transaction Generator initialized");
    }

    pub fn transaction_loop(&mut self, theta: u64) {
        loop {
            // let s_tx: SignedTransaction = ;//generate_random_signed_transaction(&self.key, &self.states, &self.blockchain);
            let state_contains_addr = self.states.lock().unwrap()[&self.blockchain.lock().unwrap().tip()].contains_key(&self.public_addr);
            // Only create transactions if we have something to provide
            if state_contains_addr {
                let s_tx = generate_random_signed_transaction(&self.key, &self.public_addr, &self.peers, &self.states, &self.blockchain);
                
                let mut mempool_guard = self.mempool.lock().unwrap();
                let tx_hash = s_tx.hash();
                mempool_guard.insert(tx_hash, s_tx);
                std::mem::drop(mempool_guard);

                self.server.broadcast(Message::NewTransactionHashes(vec![tx_hash]));
                // TODO Place this somewhere better
                // self.server.broadcast(Message::InitialState(self.node_start_time, self.public_addr));
            } else {
                println!("This node has balance=0, did not recieve ICO or balance from peers. Cannot create txs.");
            }
            
            thread::sleep(time::Duration::from_millis(theta));
            // let interval = Duration::from_micros(theta as u64);
            // thread::sleep(interval);
        }
    }
}
// Need to use static address to sign & send the transaction
pub fn generate_random_signed_transaction(
    key: &Arc<Mutex<Ed25519KeyPair>>, 
    public_addr: &Address, 
    peers: &Arc<Mutex<Vec<Address>>>, 
    states: &Arc<Mutex<HashMap<H256, HashMap<Address, (u64, u64)>>>>, 
    blockchain: &Arc<Mutex<Blockchain>>
) -> SignedTransaction {

    let mut rng = rand::thread_rng();
    
    let val: u64 = 1u64;//rng.gen();   // Generate random value for transaction

    // let key = key_pair::random();
    // let pub_key = key.public_key();
    // let h = digest::digest(&digest::SHA256, pub_key.as_ref());
    // let hex_h = hex::encode(h).into_bytes();
    // let addr1 = Address::from_public_key_bytes(&hex_h);
    
    let own_key_pair = key.lock().unwrap();
    let own_pub_key = own_key_pair.public_key();
    // let h = digest::digest(&digest::SHA256, own_pub_key.as_ref());
    // let hex_h = hex::encode(h).into_bytes();
    // let addr1 = Address::from_public_key_bytes(&hex_h);
    std::mem::drop(key);

    let rand_addr_2: Address;
    let peers_guard = peers.lock().unwrap();

    // // randomly choose address two from our peer vector
    if peers_guard.len() > 0 {
        let mut rng = rand::thread_rng();
        let rand_num = rng.gen_range(0..peers_guard.len());
        rand_addr_2 = peers_guard[rand_num];
        println!("ALL MY PEERS {:?}", peers_guard);
    }
    else {
        println!("Peers Vector is empty, len == 0\n");
        let key = key_pair::random();
        let pub_key = key.public_key();
        let h = digest::digest(&digest::SHA256, pub_key.as_ref());
        let hex_h = hex::encode(h).into_bytes();
        rand_addr_2 = Address::from_public_key_bytes(&hex_h);
    }

    std::mem::drop(peers_guard);

    println!("ADDRESSES {:?} {:?}", public_addr, rand_addr_2); 

    let blockchain_guard = blockchain.lock().unwrap();
    let states_guard = states.lock().unwrap();
    
    let (nonce, bal) = states_guard[&blockchain_guard.tip()][&public_addr];

    std::mem::drop(blockchain_guard);
    std::mem::drop(states_guard);
    println!("NONCE IN TX GENERATOR: {:?}", nonce);

    let t: Transaction = Transaction::new(public_addr.clone(), rand_addr_2, val, nonce + 1);

    let sig = sign(&t, &own_key_pair);
    let sig_vec: Vec<u8> = sig.as_ref().to_vec();
    let pub_key_vec: Vec<u8> = own_pub_key.as_ref().to_vec();
    
    let rand_st = SignedTransaction::new(t, sig_vec, pub_key_vec);

    return rand_st;
}