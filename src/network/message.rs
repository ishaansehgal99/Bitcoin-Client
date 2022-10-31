use serde::{Serialize, Deserialize};

use crate::types::{hash::H256, block::Block, transaction::SignedTransaction};
// use std::time::{SystemTime};
use crate::types::address::Address;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    Ping(String),
    Pong(String),
    NewBlockHashes(Vec<H256>),
    GetBlocks(Vec<H256>),
    Blocks(Vec<Block>),
    NewTransactionHashes(Vec<H256>),
    GetTransactions(Vec<H256>),
    Transactions(Vec<SignedTransaction>),
    RequestState(Address),
    RespondState(HashMap<H256, HashMap<Address, (u64, u64)>>, Address),
    // InitialState(SystemTime, Address)
}
