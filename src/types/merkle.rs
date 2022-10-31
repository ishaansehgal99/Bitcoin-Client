use super::hash::{Hashable, H256};
use ring::{digest};

/// A Merkle tree.
#[derive(Debug, Default)]
pub struct MerkleTree {
    tree: Vec<H256>,
    len_leaves: usize
}

impl MerkleTree {
    pub fn new<T>(data: &[T]) -> Self where T: Hashable, {
        let mut new_tree = Vec::new();
        let mut d_len: usize = data.len();
        if d_len == 0 { return MerkleTree{tree: new_tree, len_leaves: 0}; }
        for x in data {
            new_tree.push(x.hash());
        }

        if d_len % 2 == 1 {
            new_tree.push(new_tree[d_len-1]);
            d_len += 1;
        }

        let mut idx = d_len / 2;
        let mult = 2;
        while idx >= 1 {
            let mut tmp_arr: Vec<H256> = Vec::new();
            for i in 0..idx {
                let hash1:H256 = new_tree[mult*i];
                let hash2:H256 = new_tree[(mult*i)+1];

                let mut ctx = digest::Context::new(&digest::SHA256);
                ctx.update(hash1.as_ref());
                ctx.update(hash2.as_ref());
                tmp_arr.push(H256::from(ctx.finish()));
            }
            if idx != 1 && idx % 2 == 1 {
                tmp_arr.push(tmp_arr[tmp_arr.len()-1]);
                idx += 1;
            }
            new_tree = [tmp_arr, new_tree].concat();
            idx /= 2;
        }
        return MerkleTree {tree: new_tree, len_leaves: d_len};
    }

    pub fn root(&self) -> H256 {
        if self.len_leaves == 0 {
            let mut ctx = digest::Context::new(&digest::SHA256);
            ctx.update("".as_ref());
            return H256::from(ctx.finish());
        } else { return self.tree[0]; }
    }

    /// Returns the Merkle Proof of data at index i
    pub fn proof(&self, index: usize) -> Vec<H256> {
        let mut proof: Vec<H256> = Vec::new();
        let mut idx = self.tree.len() - (self.len_leaves - index);
        while idx > 0 {
            let parent = (idx-1)/2;

            let left = parent*2 + 1;
            let right = parent*2 + 2;

            if self.tree[left] != self.tree[idx] {
                proof.push(self.tree[left]);
            } else {
                proof.push(self.tree[right]);
            }
            
            idx = parent;
        }
        proof
    }
}

/// Verify that the datum hash with a vector of proofs will produce the Merkle root. Also need the
/// index of datum and `leaf_size`, the total number of leaves.
pub fn verify(root: &H256, datum: &H256, proof: &[H256], index: usize, leaf_size: usize) -> bool {
    let mut cur_idx = index;
    let mut cur_leaves = leaf_size;
    let mut cur: H256 = H256::from(*datum);
    
    for x in proof {
        let mut ctx = digest::Context::new(&digest::SHA256);
        if cur_idx % 2 == 0 {
            ctx.update(cur.as_ref());
            ctx.update(x.as_ref());
        } else {
            ctx.update(x.as_ref());
            ctx.update(cur.as_ref());
        }
        cur = H256::from(ctx.finish());
        cur_idx = cur_idx/2 + cur_leaves;
        if (((cur_leaves+1)/2)%2 == 1) {
            cur_leaves = cur_leaves + 1;
        } 
    } 

    return cur == *root;
}
// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

#[cfg(test)]
mod tests {
    use crate::types::hash::H256;
    use super::*;

    macro_rules! gen_merkle_tree_data {
        () => {{
            vec![
                (hex!("0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d")).into(),
                (hex!("0101010101010101010101010101010101010101010101010101010101010202")).into()
            ]                                                                                     
        }};                                                                                       
    }                                                                                             

    #[test]
    fn merkle_root() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        let merkle_tree = MerkleTree::new(&input_data);
        let root = merkle_tree.root();
        assert_eq!(
            root,
            (hex!("6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920")).into()
        );
        // "b69566be6e1720872f73651d1851a0eae0060a132cf0f64a0ffaea248de6cba0" is the hash of
        // "0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d"
        // "965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f" is the hash of
        // "0101010101010101010101010101010101010101010101010101010101010202"
        // "6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920" is the hash of
        // the concatenation of these two hashes "b69..." and "965..."
        // notice that the order of these two matters
    }

    #[test]
    fn merkle_proof() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        let merkle_tree = MerkleTree::new(&input_data);
        let proof = merkle_tree.proof(0);
        assert_eq!(proof,
                   vec![hex!("965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f").into()]
        );
        // "965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f" is the hash of
        // "0101010101010101010101010101010101010101010101010101010101010202"
    }

    #[test]
    fn merkle_verifying() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        let merkle_tree = MerkleTree::new(&input_data);
        let proof = merkle_tree.proof(0);
        assert!(verify(&merkle_tree.root(), &input_data[0].hash(), &proof, 0, input_data.len()));
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST