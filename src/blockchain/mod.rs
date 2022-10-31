use crate::types::block::{Block, Header, Content};
use crate::types::hash::{H256, Hashable};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use ring::{digest};
use rand::Rng;

//Arc<Mutex<Blockchain>>

#[derive( Debug, Clone)]
pub struct Blockchain {
    pub blocks: HashMap<H256, Block>,
    pub tip: H256
}

impl Blockchain {
    pub fn get_genesis_block_hash(&self) -> H256 {
        let mut ctx = digest::Context::new(&digest::SHA256);
        ctx.update("genesis".as_ref());
        let rand_hash = H256::from(ctx.finish());

        // Rand Nonce
        let mut rng = rand::thread_rng();
        let rand_nonce: u32 = rng.gen();   // Generate random value for nonce

        let difficulty:[u8; 32] = [0u8,0u8,255u8,255u8,1u8,1u8,0u8,0u8,0u8,0u8,0u8,1u8,0u8,0u8,0u8,0u8,1u8,0u8,0u8,0u8,0u8,0u8,0u8,0u8,0u8,0u8,0u8,0u8,0u8,0u8,0u8,0u8];
        let header = Header {
            parent: rand_hash,
            nonce: 0u32,
            difficulty: difficulty.into(), // TODO: Set appropriate difficulty
            timestamp: 0,//SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_millis(),
            merkle_root: rand_hash
        };
        
        let block = Block{header: header, content: Content {data: Vec::new()}, height: 0};
        return block.hash();
    }

    /// Create a new blockchain, only containing the genesis block
    pub fn new() -> Self {
        let mut ctx = digest::Context::new(&digest::SHA256);
        ctx.update("genesis".as_ref());
        let rand_hash = H256::from(ctx.finish());

        // Rand Nonce
        let mut rng = rand::thread_rng();
        let rand_nonce: u32 = rng.gen();   // Generate random value for nonce

        // let first_part_difficulty: [u8; 16] = [0u8; 16];
        // let second_part_difficulty: [u8; 16] = [1u8; 16]; 
        let difficulty:[u8; 32] = [0u8,0u8,255u8,255u8,1u8,1u8,0u8,0u8,0u8,0u8,0u8,1u8,0u8,0u8,0u8,0u8,1u8,0u8,0u8,0u8,0u8,0u8,0u8,0u8,0u8,0u8,0u8,0u8,0u8,0u8,0u8,0u8];
        let header = Header {
            parent: rand_hash,
            nonce: 0u32,
            difficulty: difficulty.into(), // TODO: Set appropriate difficulty
            timestamp: 0,//SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_millis(),
            merkle_root: rand_hash
        };
         

        let block = Block{header: header, content: Content {data: Vec::new()}, height: 0};
        
        let mut blocks = HashMap::new();
        let block_hash = block.hash();
        blocks.insert(block_hash, block);
        
        Self {
            blocks: blocks,
            tip: block_hash
        }
    }

    /// Insert a block into blockchain
    pub fn insert(&mut self, block: &Block) {
        let mut b = block.clone();
        if self.blocks.contains_key(&block.header.parent) {
            b.height = self.blocks[&block.header.parent].height + 1;
            let b_height = b.height;
            let b_hash = b.hash(); 

            self.blocks.insert(b_hash, b);
            if b_height > self.blocks[&self.tip].height {
                self.tip = b_hash;
                println!("New longest chain, block added as tip: {:?}", b_hash);
            }

            println!("Inserting block with parent: {:?}", block.header.parent);
        }

        // let mut b = block.clone();
        // // Do we want to do something like 
        // // https://stackoverflow.com/questions/50435553/convert-u8-to-string
        // // to get parent of block's hash as string value?
        // println!("{:?} PARENT", block.header.parent, self.blocks); 
        // b.height = self.blocks[&block.header.parent].height + 1;
        // let b_hash = b.hash(); 
        // if b.height > self.blocks[&self.tip].height {
        //     // Objects cannot be keys
        //     // So are we hashing a reference to the object?
        //     // Or somehow the string value of the object?
        //     // I don't think we want to be hashing references b/c
        //     // their lifetime is not guaranteed to extend beyond
        //     // the scope of this block...or does it? How does 
        //     // garbage collection in rust work? Does it maintain 
        //     // reference counts to objects?
        //     self.blocks.insert(b_hash, b);
        //     self.tip = b_hash;
        //     println!("Inserting block with parent: {:?}", block.header.parent);
        // }
    }

    /// Get the last block's hash of the longest chain
    pub fn tip(&self) -> H256 {
        return self.tip;
    }

    /// Get all blocks' hashes of the longest chain, ordered from genesis to the tip
    pub fn all_blocks_in_longest_chain(&self) -> Vec<H256> {
        let mut blocks = Vec::new();
        let mut cur_block = self.tip;

        if self.blocks[&cur_block].height == 0 {
            blocks.push(cur_block);
            return blocks;
        }
        
        while (self.blocks[&cur_block].height > 0) {
            blocks.push(cur_block);
            cur_block = self.blocks[&cur_block].header.parent;
        }
        blocks.push(cur_block);

        blocks.reverse();
        blocks
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::block::generate_random_block;
    use crate::types::hash::Hashable;

    #[test]
    fn insert_one() {
        let mut blockchain = Blockchain::new();
        let genesis_hash = blockchain.tip();
        let block = generate_random_block(&genesis_hash);
        blockchain.insert(&block);
        assert_eq!(blockchain.tip(), block.hash());
    }

    use ntest::timeout;
    use super::*;
    #[test]
    #[timeout(60000)]
    fn sp2022autograder021() {
        let mut blockchain = Blockchain::new();
        let genesis_hash = blockchain.tip();
        let mut block = generate_random_block(&genesis_hash);
        blockchain.insert(&block);
        assert_eq!(blockchain.tip(), block.hash());
        for _ in 0..50 {
            let h = block.hash();
            block = generate_random_block(&h);
            blockchain.insert(&block);
            assert_eq!(blockchain.tip(), block.hash());
        }
    }
    #[test]
    #[timeout(60000)]
    fn sp2022autograder022() {
        let mut blockchain = Blockchain::new();
        let genesis_hash = blockchain.tip();
        let block_1 = generate_random_block(&genesis_hash);
        blockchain.insert(&block_1);
        assert_eq!(blockchain.tip(), block_1.hash());
        let block_2 = generate_random_block(&block_1.hash());
        blockchain.insert(&block_2);
        assert_eq!(blockchain.tip(), block_2.hash());
        let block_3 = generate_random_block(&block_2.hash());
        blockchain.insert(&block_3);
        assert_eq!(blockchain.tip(), block_3.hash());
        let fork_block_1 = generate_random_block(&genesis_hash);
        blockchain.insert(&fork_block_1);
        assert_eq!(blockchain.tip(), block_3.hash());
        let fork_block_2 = generate_random_block(&fork_block_1.hash());
        blockchain.insert(&fork_block_2);
        assert_eq!(blockchain.tip(), block_3.hash());
    }
    #[test]
    #[timeout(60000)]
    fn sp2022autograder023() {
        let mut blockchain = Blockchain::new();
        let genesis_hash = blockchain.tip();
        let block_1 = generate_random_block(&genesis_hash);
        blockchain.insert(&block_1);
        assert_eq!(blockchain.tip(), block_1.hash());
        let block_2 = generate_random_block(&block_1.hash());
        blockchain.insert(&block_2);
        assert_eq!(blockchain.tip(), block_2.hash());
        let fork_block_1 = generate_random_block(&genesis_hash);
        blockchain.insert(&fork_block_1);
        assert_eq!(blockchain.tip(), block_2.hash());
        let fork_block_2 = generate_random_block(&fork_block_1.hash());
        blockchain.insert(&fork_block_2);
        //assert_eq!(blockchain.tip(), block_2.hash());
        let fork_block_3 = generate_random_block(&fork_block_2.hash());
        blockchain.insert(&fork_block_3);
        assert_eq!(blockchain.tip(), fork_block_3.hash());
    }
    #[test]
    #[timeout(60000)]
    fn sp2022autograder024() {
        let mut blockchain = Blockchain::new();
        let genesis_hash = blockchain.tip();
        let block_1 = generate_random_block(&genesis_hash);
        blockchain.insert(&block_1);
        assert_eq!(blockchain.tip(), block_1.hash());
        let block_2 = generate_random_block(&block_1.hash());
        blockchain.insert(&block_2);
        assert_eq!(blockchain.tip(), block_2.hash());
        let block_3 = generate_random_block(&block_2.hash());
        blockchain.insert(&block_3);
        assert_eq!(blockchain.tip(), block_3.hash());
        let fork_block_1 = generate_random_block(&block_2.hash());
        blockchain.insert(&fork_block_1);
        assert_eq!(blockchain.tip(), block_3.hash());
        let fork_block_2 = generate_random_block(&fork_block_1.hash());
        blockchain.insert(&fork_block_2);
        assert_eq!(blockchain.tip(), fork_block_2.hash());
        let block_4 = generate_random_block(&block_3.hash());
        blockchain.insert(&block_4);
        assert_eq!(blockchain.tip(), fork_block_2.hash());
        let block_5 = generate_random_block(&block_4.hash());
        blockchain.insert(&block_5);
        assert_eq!(blockchain.tip(), block_5.hash());
    }
    #[test]
    #[timeout(60000)]
    fn sp2022autograder025() {
        let mut blockchain = Blockchain::new();
        let genesis_hash = blockchain.tip();
        let block_1 = generate_random_block(&genesis_hash);
        blockchain.insert(&block_1);
        assert_eq!(blockchain.tip(), block_1.hash());
        let block_2 = generate_random_block(&block_1.hash());
        blockchain.insert(&block_2);
        assert_eq!(blockchain.tip(), block_2.hash());
        let block_3 = generate_random_block(&block_2.hash());
        blockchain.insert(&block_3);
        assert_eq!(blockchain.tip(), block_3.hash());
        let fork_block_1 = generate_random_block(&block_2.hash());
        blockchain.insert(&fork_block_1);
        assert_eq!(blockchain.tip(), block_3.hash());
        let fork_block_2 = generate_random_block(&fork_block_1.hash());
        blockchain.insert(&fork_block_2);
        assert_eq!(blockchain.tip(), fork_block_2.hash());
        let another_block_1 = generate_random_block(&genesis_hash);
        blockchain.insert(&another_block_1);
        assert_eq!(blockchain.tip(), fork_block_2.hash());
        let another_block_2 = generate_random_block(&another_block_1.hash());
        blockchain.insert(&another_block_2);
        assert_eq!(blockchain.tip(), fork_block_2.hash());
        let another_block_3 = generate_random_block(&another_block_2.hash());
        blockchain.insert(&another_block_3);
        assert_eq!(blockchain.tip(), fork_block_2.hash());
        let another_block_4 = generate_random_block(&another_block_3.hash());
        blockchain.insert(&another_block_4);
        assert_eq!(blockchain.tip(), fork_block_2.hash());
        let another_block_5 = generate_random_block(&another_block_4.hash());
        blockchain.insert(&another_block_5);
        assert_eq!(blockchain.tip(), another_block_5.hash());
        let another_block_6 = generate_random_block(&another_block_5.hash());
        blockchain.insert(&another_block_6);
        assert_eq!(blockchain.tip(), another_block_6.hash());
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST