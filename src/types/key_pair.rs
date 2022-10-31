use ring::rand;
use ring::signature::Ed25519KeyPair;

/// Generate a random key pair.
pub fn random() -> Ed25519KeyPair {
    let rng = rand::SystemRandom::new();
    let pkcs8_bytes = Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
    Ed25519KeyPair::from_pkcs8(pkcs8_bytes.as_ref().into()).unwrap()
}

// pub fn rec_key_pair() -> Ed25519KeyPair {
//     let mut rng = rand::thread_rng();
//     let pkcs8_bytes = Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
//     Ed25519KeyPair::from_pkcs8(pkcs8_bytes.as_ref().into()).unwrap()

//     // let rng = rand::SystemRandom::new();
//     // let pkcs8_bytes = Ed25519KeyPair::from_seed_and_public_key("10.2.30.123", "").unwrap();
//     // return "c78b6d77d85a94490ee5ac63d46444735b0d6fa05289acef4e188006a2e256e5";
// }
