use serde::{Serialize, Deserialize};
use crate::types::hash::{H256, Hashable};
use crate::types::transaction::{SignedTransaction};
use ring::{digest};
use rand::Rng;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Header {
    pub parent: H256,
    pub nonce: u32,
    pub difficulty: H256,
    pub timestamp: u128,
    pub merkle_root: H256
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Content {
    pub data: Vec<SignedTransaction>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub header: Header,
    pub content: Content,
    pub height: u32
}

impl Hashable for Header {
    fn hash(&self) -> H256 {
        let s = bincode::serialize(&self).unwrap();
        digest::digest(&ring::digest::SHA256, &s).into()
    }
}

impl Hashable for Block {
    fn hash(&self) -> H256 {
        self.header.hash()
    }
}

impl Block {
    pub fn get_parent(&self) -> H256 {
        self.header.parent
    }

    pub fn get_difficulty(&self) -> H256 {
        self.header.difficulty
    }
}

#[cfg(any(test, test_utilities))]
pub fn generate_random_block(parent: &H256) -> Block {
    // Rand Nonce
    let mut rng = rand::thread_rng();
    let rand_nonce: u32 = rng.gen();   // Generate random value for nonce

    // Rand H256
    let mut ctx = digest::Context::new(&digest::SHA256);
    ctx.update("".as_ref()); // When testing all new blocks will have the same difficulty and merkle root.
    let rand_hash = H256::from(ctx.finish());
    
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_millis();

    let header = Header {
        parent: *parent,
        nonce: rand_nonce,
        difficulty: rand_hash,
        timestamp: timestamp,
        merkle_root: rand_hash
    };

    Block{header: header, content: Content {data: Vec::new()}, height: 0}
}