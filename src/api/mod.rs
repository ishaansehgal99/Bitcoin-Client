use serde::Serialize;
use crate::blockchain::Blockchain;
use crate::types::transaction_generator::{TransactionGenerator};
use crate::miner::Handle as MinerHandle;
// use crate::transaction_generator::Handle as TXHandle;
use crate::network::server::Handle as NetworkServerHandle;
use crate::network::message::Message;
use crate::types::transaction::SignedTransaction;
use crate::types::hash::{H256, Hashable};

use log::info;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use tiny_http::Header;
use tiny_http::Response;
use tiny_http::Server as HTTPServer;
use url::Url;
use crate::types::address::Address;

pub struct Server {
    handle: HTTPServer,
    miner: MinerHandle,
    network: NetworkServerHandle,
    blockchain: Arc<Mutex<Blockchain>>,
    tx_handler: TransactionGenerator,
    states: Arc<Mutex<HashMap<H256, HashMap<Address, (u64, u64)>>>>
}

#[derive(Serialize)]
struct ApiResponse {
    success: bool,
    message: String,
}

macro_rules! respond_result {
    ( $req:expr, $success:expr, $message:expr ) => {{
        let content_type = "Content-Type: application/json".parse::<Header>().unwrap();
        let payload = ApiResponse {
            success: $success,
            message: $message.to_string(),
        };
        let resp = Response::from_string(serde_json::to_string_pretty(&payload).unwrap())
            .with_header(content_type);
        $req.respond(resp).unwrap();
    }};
}
macro_rules! respond_json {
    ( $req:expr, $message:expr ) => {{
        let content_type = "Content-Type: application/json".parse::<Header>().unwrap();
        let resp = Response::from_string(serde_json::to_string(&$message).unwrap())
            .with_header(content_type);
        $req.respond(resp).unwrap();
    }};
}

impl Server {
    pub fn start(
        addr: std::net::SocketAddr,
        miner: &MinerHandle,
        network: &NetworkServerHandle,
        blockchain: &Arc<Mutex<Blockchain>>, 
        tx_handler: &TransactionGenerator,
        states: &Arc<Mutex<HashMap<H256, HashMap<Address, (u64, u64)>>>>
    ) {
        let handle = HTTPServer::http(&addr).unwrap();
        let server = Self {
            handle,
            miner: miner.clone(),
            network: network.clone(),
            blockchain: Arc::clone(blockchain),
            tx_handler: tx_handler.clone(),
            states: Arc::clone(states)
        };
        thread::spawn(move || {
            for req in server.handle.incoming_requests() {
                let miner = server.miner.clone();
                let network = server.network.clone();
                let blockchain = Arc::clone(&server.blockchain);
                let tx_handler = server.tx_handler.clone();
                let states = Arc::clone(&server.states);
                thread::spawn(move || {
                    // a valid url requires a base
                    let base_url = Url::parse(&format!("http://{}/", &addr)).unwrap();
                    let url = match base_url.join(req.url()) {
                        Ok(u) => u,
                        Err(e) => {
                            respond_result!(req, false, format!("error parsing url: {}", e));
                            return;
                        }
                    };
                    match url.path() {
                        "/miner/start" => {
                            let params = url.query_pairs();
                            let params: HashMap<_, _> = params.into_owned().collect();
                            let lambda = match params.get("lambda") {
                                Some(v) => v,
                                None => {
                                    respond_result!(req, false, "missing lambda");
                                    return;
                                }
                            };
                            let lambda = match lambda.parse::<u64>() {
                                Ok(v) => v,
                                Err(e) => {
                                    respond_result!(
                                        req,
                                        false,
                                        format!("error parsing lambda: {}", e)
                                    );
                                    return;
                                }
                            };
                            miner.start(lambda);
                            respond_result!(req, true, "ok");
                        }
                        "/blockchain/state" => {
                            let params = url.query_pairs();
                            let params: HashMap<_, _> = params.into_owned().collect();
                            let block = match params.get("block") {
                                Some(v) => v,
                                None => {
                                    respond_result!(req, false, "missing block");
                                    return;
                                }
                            };
                            let block = match block.parse::<u64>() {
                                Ok(v) => v,
                                Err(e) => {
                                    respond_result!(
                                        req,
                                        false,
                                        format!("error parsing block: {}", e)
                                    );
                                    return;
                                }
                            };
                            println!("Request Block #: {:?}", block); 
                            let blockchain = blockchain.lock().unwrap();
                            let v = blockchain.all_blocks_in_longest_chain();
                            let mut v_string: Vec<String> = Vec::new();

                            let block_usize = block as usize;
                            if block_usize < v.len() {
                                let state = &states.lock().unwrap()[&v[block_usize]];
                                for (address, (nonce, bal)) in state.iter() {
                                    let s_addr = format!("{}", address.to_string());
                                    let s_nonce = nonce.to_string();
                                    let s_bal = bal.to_string();
                                    let tuple: String = s_addr + ", " + &s_nonce + ", " + &s_bal;
                                    v_string.push(tuple);
                                }
                            } else {
                                println!("Out of range, no block # {:?}", block_usize);
                            }

                            respond_json!(req, v_string);
                        }
                        "/tx-generator/start" => {
                            let params = url.query_pairs();
                            let params: HashMap<_, _> = params.into_owned().collect();
                            let theta = match params.get("theta") {
                                Some(v) => v,
                                None => {
                                    respond_result!(req, false, "missing theta");
                                    return;
                                }
                            };
                            let theta = match theta.parse::<u64>() {
                                Ok(v) => v,
                                Err(e) => {
                                    respond_result!(
                                        req,
                                        false,
                                        format!("error parsing theta: {}", e)
                                    );
                                    return;
                                }
                            };

                            tx_handler.start(theta);
                            respond_result!(req, true, "ok");
                        }
                        "/network/ping" => {
                            network.broadcast(Message::Ping(String::from("Test ping")));
                            respond_result!(req, true, "ok");
                        }
                        "/blockchain/longest-chain" => {
                            let blockchain = blockchain.lock().unwrap();
                            let v = blockchain.all_blocks_in_longest_chain();
                            let /*mut*/ v_string: Vec<String> = v.into_iter().map(|h|h.to_string()).collect();
                            
                            // TODO: SHOULD THIS BE HERE? Commented
                            // let s: String = v_string.len().to_string();
                            // v_string.push(s);

                            respond_json!(req, v_string);
                        }
                        "/blockchain/longest-chain-tx" => {
                            // unimplemented!()
                            let blockchain = blockchain.lock().unwrap();
                            let v = blockchain.all_blocks_in_longest_chain();

                            let mut v_string: Vec<Vec<String>> = Vec::new();

                            for block in v {
                                let mut txs_vec: Vec<String> = Vec::new(); 
                                let txs: Vec<SignedTransaction> = blockchain.blocks[&block].content.data.clone();
                                
                                for tx in txs {
                                    let tx_hash = tx.hash().to_string();
                                    txs_vec.push(tx_hash);
                                }
                                v_string.push(txs_vec); 
                            }
                            
                            respond_json!(req, v_string);
                        }
                        "/blockchain/longest-chain-tx-count" => {
                            // unimplemented!()
                            respond_result!(req, false, "unimplemented!");
                        }
                        _ => {
                            let content_type =
                                "Content-Type: application/json".parse::<Header>().unwrap();
                            let payload = ApiResponse {
                                success: false,
                                message: "endpoint not found".to_string(),
                            };
                            let resp = Response::from_string(
                                serde_json::to_string_pretty(&payload).unwrap(),
                            )
                            .with_header(content_type)
                            .with_status_code(404);
                            req.respond(resp).unwrap();
                        }
                    }
                });
            }
        });
        info!("API server listening at {}", &addr);
    }
}
