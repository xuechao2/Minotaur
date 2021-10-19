use crate::crypto::hash::H160;
use serde::{Serialize,Deserialize};
use ring::signature::{Ed25519KeyPair, Signature, KeyPair, VerificationAlgorithm, EdDSAParameters};
use rand::Rng;
use crate::crypto::hash::{self, Hashable, H256,generate_random_hash};
use crate::crypto::key_pair;


#[derive(Serialize, Deserialize, Debug, Default,Hash, Eq, PartialEq,Clone)]
pub struct Transaction {
    pub recv: hash::H160,
    pub value: usize,
    pub nonce: usize,
}

#[derive(Serialize, Deserialize, Debug, Default,Hash, Eq, PartialEq,Clone)]
pub struct Sign {
    pub pubk: Vec<u8>,
    pub sig: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Default,Hash, Eq, PartialEq,Clone)]
pub struct SignedTransaction {
    pub transaction: Transaction,
    pub sign: Sign,
}

impl Hashable for Transaction {
    fn hash(&self) -> H256 {
        let serialized: Vec<u8> = bincode::serialize(&self).unwrap();
        let bytes: &[u8] = &serialized;
        ring::digest::digest(&ring::digest::SHA256, bytes).into()
    }
}

impl Hashable for SignedTransaction {
    fn hash(&self) -> H256 {
        let serialized: Vec<u8> = bincode::serialize(&self).unwrap();
        let bytes: &[u8] = &serialized;
        ring::digest::digest(&ring::digest::SHA256, bytes).into()
    }
}


/// Create digital signature of a transaction
pub fn sign(t: &Transaction, key: &Ed25519KeyPair) -> Signature {
    let serialized: Vec<u8> = bincode::serialize(&t).unwrap();
    key.sign(&serialized)
}

/// Verify digital signature of a transaction, using public key instead of secret key
pub fn verify(t: &Transaction, public_key: &<Ed25519KeyPair as KeyPair>::PublicKey, signature: &Signature) -> bool {
    let serialized: Vec<u8> = bincode::serialize(&t).unwrap();
    let bytes: &[u8] = &serialized;
    match VerificationAlgorithm::verify(&EdDSAParameters, public_key.as_ref().into(), bytes.into(), signature.as_ref().into()) {
        Ok(_) => true,
        Err(_e) => false,
    }
}

pub fn verify_signedtxn(t: &SignedTransaction) -> bool {
    let transaction = t.transaction.clone();
    let pubk = t.sign.pubk.clone();
    let sig = t.sign.sig.clone();
    let serialized: Vec<u8> = bincode::serialize(&transaction).unwrap();
    let bytes: &[u8] = &serialized;
    match VerificationAlgorithm::verify(&EdDSAParameters, pubk[..].into(), bytes.into(), sig[..].into()) {
        Ok(_) => true,
        Err(_e) => false,
    }
}

pub fn generate_random_transaction() -> Transaction {
    let mut rng = rand::thread_rng();
    Transaction {
    recv:  hash::generate_random_hash().into(),
    value: rng.gen(),
    nonce: rng.gen(),
    }
}

pub fn generate_random_signed_transaction() -> SignedTransaction {
    let transaction = generate_random_transaction();
    let pubk = key_pair::random();
    let sig = sign(&transaction, &pubk);
    let sign = Sign {
        pubk: pubk.public_key().as_ref().to_vec(),
        sig: sig.as_ref().to_vec(),
    };
    SignedTransaction {
        transaction,
        sign,
    }
}

pub fn generate_valid_transaction(recv: H160, value: usize, nonce: usize) -> Transaction {
    //let mut rng = rand::thread_rng();
    Transaction {
    recv:  recv,
    value: value,
    nonce: nonce,
    }
}

pub fn generate_valid_signed_transaction(recv: H160, value: usize, nonce: usize, pubk:&Ed25519KeyPair) -> SignedTransaction {
    let transaction = generate_valid_transaction(recv,value,nonce);
    //let pubk = key_pair::random();
    let sig = sign(&transaction, &pubk);
    let sign = Sign {
        pubk: pubk.public_key().as_ref().to_vec(),
        sig: sig.as_ref().to_vec(),
    };
    SignedTransaction {
        transaction,
        sign,
    }
}


#[cfg(any(test, test_utilities))]
mod tests {
    use super::*;
    use crate::crypto::key_pair;

    pub fn generate_random_transaction() -> Transaction {
        let mut rng = rand::thread_rng();
        Transaction {
            recv: hash::generate_random_hash().into(),
            value: rng.gen(),
            nonce: rng.gen(),
        }
    }

    #[test]
    fn sign_verify() {
        let t = generate_random_transaction();
        let key = key_pair::random();
        let signature = sign(&t, &key);
        assert!(verify(&t, &(key.public_key()), &signature));
    }

    #[test]
    fn sign_verify2() {
        let t = generate_random_signed_transaction();
        assert!(verify_signedtxn(&t));
    }
}
