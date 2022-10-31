use crate::types::address::Address;
use crate::types::hash::{H256, Hashable};
use crate::types::key_pair;
use serde::{Serialize,Deserialize};
use ring::signature::{self, Ed25519KeyPair, Signature, KeyPair, VerificationAlgorithm, EdDSAParameters};
use ring::{digest};
use rand::Rng;
use crate::network::message::Message;
use crate::network::peer;
use crate::network::server::Handle as ServerHandle;
use crate::time;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Transaction {
    pub sender: Address,
    pub receiver: Address,
    pub value: u64,
    // The nonce is the transaction counter of the sending address. 
    // It is the # of transactions sent by the sending address. 
    // It starts at 0.
    pub acc_nonce: u64 
}
impl Transaction {
    pub fn new(sender: Address, receiver: Address, value: u64, acc_nonce: u64) -> Self {
        Self {
            sender: sender,
            receiver: receiver,
            value: value,
            acc_nonce: acc_nonce
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct SignedTransaction {
    pub t: Transaction,
    pub sig: Vec<u8>,
    pub pub_key: Vec<u8>,
}

impl SignedTransaction {
    pub fn new(t: Transaction, sig: Vec<u8>, pub_key: Vec<u8>) -> Self {
        Self {
            t: t,
            sig: sig,
            pub_key: pub_key
        }
    }
}

impl Hashable for SignedTransaction {
    fn hash(&self) -> H256 {
        let s = bincode::serialize(&self).unwrap();
        ring::digest::digest(&ring::digest::SHA256, &s).into()
    }
}

/// Create digital signature of a transaction
pub fn sign(t: &Transaction, key: &Ed25519KeyPair) -> Signature {
    let message = bincode::serialize(&t).unwrap();
    let t_id = digest::digest(&digest::SHA256, digest::digest(&digest::SHA256, message.as_ref()).as_ref());
    let sig: Signature = key.sign(t_id.as_ref());
    return sig;
}

/// Verify digital signature of a transaction, using public key instead of secret key
pub fn st_verify(st: &SignedTransaction) -> bool {
    verify(&st.t, &st.pub_key, &st.sig)
} 

/// Verify digital signature of a transaction, using public key instead of secret key
pub fn verify(t: &Transaction, public_key: &[u8], signature: &[u8]) -> bool {
    let message = bincode::serialize(&t).unwrap();
    let t_id = digest::digest(&digest::SHA256, digest::digest(&digest::SHA256, message.as_ref()).as_ref());
    let pub_key = signature::UnparsedPublicKey::new(&signature::ED25519, public_key.as_ref());
    let ret = pub_key.verify(t_id.as_ref(), signature.as_ref()).is_ok();
    return ret;
}

#[cfg(any(test, test_utilities))]
pub fn generate_random_transaction() -> Transaction {
    let mut rng = rand::thread_rng();
    
    let val: u64 = rng.gen();   // Generate random value for transaction

    let key = key_pair::random();
    let pub_key = key.public_key();
    let h = digest::digest(&digest::SHA256, pub_key.as_ref());
    let hex_h = hex::encode(h).into_bytes();
    let addr1 = Address::from_public_key_bytes(&hex_h);
    /* 
    let key = key_pair::random();
    let pub_key = key.public_key();
    let h = digest::digest(&digest::SHA256, pub_key.as_ref());
    let hex_h = hex::encode(h).into_bytes();
    let addr2 = Address::from_public_key_bytes(&hex_h);
    */
    let t: Transaction = Transaction{sender: addr1, receiver: addr1, value: val, acc_nonce: 0u64};
    return t;
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::key_pair;
    use ring::signature::KeyPair;


    #[test]
    fn sign_verify() {
        let t = generate_random_transaction();
        let key = key_pair::random();
        let signature = sign(&t, &key);
        assert!(verify(&t, key.public_key().as_ref(), signature.as_ref()));
    }
    #[test]
    fn sign_verify_two() {
        let t = generate_random_transaction();
        let key = key_pair::random();
        let signature = sign(&t, &key);
        let key_2 = key_pair::random();
        let t_2 = generate_random_transaction();
        assert!(!verify(&t_2, key.public_key().as_ref(), signature.as_ref()));
        assert!(!verify(&t, key_2.public_key().as_ref(), signature.as_ref()));
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST