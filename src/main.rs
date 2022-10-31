#[cfg(test)]
#[macro_use]
extern crate hex_literal;

pub mod api;
pub mod blockchain;
pub mod types;
pub mod miner;
pub mod network;

use blockchain::Blockchain;
use clap::clap_app;
use smol::channel;
use log::{error, info};
use api::Server as ApiServer;
use std::net;
use std::process;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time;
use std::collections::HashMap;
use crate::types::block::Block;
use crate::types::hash::{H256, Hashable};
use crate::types::transaction::{SignedTransaction};
use crate::types::transaction_generator::{TransactionGenerator};
use crate::types::address::Address;
use crate::network::message::Message;

use ring::signature::{self, Ed25519KeyPair, Signature, KeyPair, VerificationAlgorithm, EdDSAParameters};
use ring::{digest};
use crate::types::key_pair;

use std::time::{SystemTime};



fn main() {
    // parse command line arguments
    let matches = clap_app!(Bitcoin =>
     (version: "0.1")
     (about: "Bitcoin client")
     (@arg verbose: -v ... "Increases the verbosity of logging")
     (@arg peer_addr: --p2p [ADDR] default_value("127.0.0.1:6000") "Sets the IP address and the port of the P2P server")
     (@arg api_addr: --api [ADDR] default_value("127.0.0.1:7000") "Sets the IP address and the port of the API server")
     (@arg known_peer: -c --connect ... [PEER] "Sets the peers to connect to at start")
     (@arg p2p_workers: --("p2p-workers") [INT] default_value("4") "Sets the number of worker threads for P2P server")
    )
    .get_matches();

    // The time our node comes online
    // let node_start_time: SystemTime = SystemTime::now();
    // Create our own address
    let key = Arc::new(Mutex::new(key_pair::random()));
    let pub_key_guard = key.lock().unwrap();
    let pub_key = pub_key_guard.public_key();
    let h = digest::digest(&digest::SHA256, pub_key.as_ref());
    std::mem::drop(pub_key_guard);

    let hex_h = hex::encode(h).into_bytes();
    let public_addr = Address::from_public_key_bytes(&hex_h);

    println!("Our address {:?}", public_addr);

    // init logger
    let verbosity = matches.occurrences_of("verbose") as usize;
    stderrlog::new().verbosity(verbosity).init().unwrap();
    let blockchain = Blockchain::new();
    let blockchain = Arc::new(Mutex::new(blockchain));
    // parse p2p server address
    let p2p_addr = matches
        .value_of("peer_addr")
        .unwrap()
        .parse::<net::SocketAddr>()
        .unwrap_or_else(|e| {
            error!("Error parsing P2P server address: {}", e);
            process::exit(1);
        });

    // parse api server address
    let api_addr = matches
        .value_of("api_addr")
        .unwrap()
        .parse::<net::SocketAddr>()
        .unwrap_or_else(|e| {
            error!("Error parsing API server address: {}", e);
            process::exit(1);
        });

    // create channels between server and worker
    let (msg_tx, msg_rx) = channel::bounded(10000);

    // start the p2p server
    let (server_ctx, server) = network::server::new(p2p_addr, msg_tx).unwrap();
    server_ctx.start().unwrap();

    // start the worker
    let p2p_workers = matches
        .value_of("p2p_workers")
        .unwrap()
        .parse::<usize>()
        .unwrap_or_else(|e| {
            error!("Error parsing P2P workers: {}", e);
            process::exit(1);
        });
    // new blockchain
    let b_chain: Blockchain = Blockchain::new();
    let blockchain = Arc::new(Mutex::new(b_chain));
    let orphans_map:Arc<Mutex<HashMap<H256, Vec<Block>>>> = Arc::new(Mutex::new(HashMap::new()));
    let mempool: Arc<Mutex<HashMap<H256, SignedTransaction>>> = Arc::new(Mutex::new(HashMap::new()));
    let states: Arc<Mutex<HashMap<H256, HashMap<Address, (u64, u64)>>>> = Arc::new(Mutex::new(HashMap::new()));
    let peers: Arc<Mutex<Vec<Address>>> = Arc::new(Mutex::new(Vec::new()));

    // Add it to our own state map
    let blockchain_guard = blockchain.lock().unwrap();
    let mut states_guard = states.lock().unwrap();
    
    let mut ico_state: HashMap<Address, (u64, u64)> = HashMap::new();
    ico_state.insert(public_addr, (0, 50));
    states_guard.insert(blockchain_guard.tip(), ico_state);

    std::mem::drop(blockchain_guard);
    std::mem::drop(states_guard);
    
    
    let worker_ctx = network::worker::Worker::new(
        p2p_workers,
        msg_rx,
        &server,
        &blockchain,
        &orphans_map,
        &mempool,
        &states,
        &peers, 
        // &node_start_time, 
        &public_addr
    );
    worker_ctx.start();

    // start the miner
    let (miner_ctx, miner, finished_block_chan) = miner::new(&blockchain, &mempool, &states); // Added &blockchain, &mempool, and &states
    let miner_worker_ctx = miner::worker::Worker::new(&server, finished_block_chan, &blockchain); // Added &blockchain
    miner_ctx.start();
    miner_worker_ctx.start();

    // connect to known peers
    if let Some(known_peers) = matches.values_of("known_peer") {
        let known_peers: Vec<String> = known_peers.map(|x| x.to_owned()).collect();
        let server = server.clone();
        let our_pub_addr = public_addr.clone();
        thread::spawn(move || {
            for peer in known_peers {
                loop {
                    let addr = match peer.parse::<net::SocketAddr>() {
                        Ok(x) => x,
                        Err(e) => {
                            error!("Error parsing peer address {}: {}", &peer, e);
                            break;
                        }
                    };
                    match server.connect(addr) {
                        Ok(_) => {
                            info!("Connected to outgoing peer {}", &addr);
                            
                            // Request for state 
                            println!("Request state from {:?}", addr);
                            server.send(addr, Message::RequestState(our_pub_addr));

                            // Write to peer requesting for state - overwrite your state with peers state
                            // let msg = "LOL".to_string();
                            // server.send(addr, Message::Ping(msg));
                            // println!("{:?}", peer);

                            break;
                        }
                        Err(e) => {
                            error!(
                                "Error connecting to peer {}, retrying in one second: {}",
                                addr, e
                            );
                            thread::sleep(time::Duration::from_millis(1000));
                            continue;
                        }
                    }
                }
            }
        });
    }
    // let (tx_ctx, tx_handler) = TransactionGenerator::new(&server);
    let tx_generator: TransactionGenerator = TransactionGenerator::new(&server, &mempool, &key, &states, &blockchain, &public_addr, /*&node_start_time,*/ &peers); 


    // start the API server
    ApiServer::start(
        api_addr,
        &miner,
        &server,
        &blockchain,
        &tx_generator,
        &states
    );

    loop {
        std::thread::park();
    }
}
