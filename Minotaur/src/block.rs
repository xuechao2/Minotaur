use crate::state::compute_key_hash;
use crate::transaction::{Transaction, SignedTransaction, generate_random_transaction, generate_random_signed_transaction};
use serde::{Serialize, Deserialize};
use crate::crypto::hash::{self, H256, Hashable,generate_random_hash};
use rand::Rng;
use crate::crypto::merkle::MerkleTree;
use ring::signature::{Ed25519KeyPair, Signature, KeyPair, VerificationAlgorithm, EdDSAParameters};
use tari_mmr::{MerkleMountainRange, MerkleProof, Hash};
use sha2::{Digest, Sha256};
use log::info;


#[derive(Serialize, Deserialize, Debug,Hash, Eq, PartialEq,Clone)]
pub struct Block {
    pub header: Header,
    pub content: Content,
}

#[derive(Serialize, Deserialize, Debug,Hash, Eq, PartialEq,Clone)]
pub struct Header {
    pub parent: H256,
    nonce: u32,
    pub difficulty: H256,
    pub timestamp: u128,
    pub merkle_root: H256,
    pub mmr_root: Hash,
}

#[derive(Serialize, Deserialize, Debug,Hash, Eq, PartialEq,Clone)]
pub struct Content {
    pub data: Vec<SignedTransaction>,
}

impl Hashable for Header {
    fn hash(&self) -> H256 {
        let serialized: Vec<u8> = bincode::serialize(&self).unwrap();
        let bytes: &[u8] = &serialized;
        ring::digest::digest(&ring::digest::SHA256, bytes).into()
    }
}

impl Hashable for Content {
    fn hash(&self) -> H256 {
        let mt: MerkleTree = MerkleTree::new(&self.data);
        mt.root()
    }
}

impl Hashable for Block {
    fn hash(&self) -> H256 {
        self.header.hash()
    }
}

impl Block {
    pub fn print_txns(&self) {
        let txns = self.content.data.clone();
        info!("***** Print txns in block {:?} *****", self.hash());
        for txn in txns {
            let sender = compute_key_hash(txn.sign.pubk);
            let recv = txn.transaction.recv;
            info!("{:?} sends {:?} value {:?}", sender, recv, txn.transaction.value);
        }
        info!("*************************************");
    }

    pub fn clear_txns(&mut self) {
        self.content.data = Vec::new();
    }
}



pub fn generate_block(data: &Vec<SignedTransaction>, parent: &H256, nonce: u32, difficulty: &H256, 
                      timestamp: u128, parent_mmr: &MerkleMountainRange<Sha256, Vec<Hash>>) -> Block {
    let mt: MerkleTree = MerkleTree::new(data);
    let content = Content {
        data: data.to_vec(),
    };
    let header = Header {
        parent: *parent,
        nonce: nonce,
        difficulty: *difficulty, 
        timestamp: timestamp,
        merkle_root: mt.root(),
        mmr_root: parent_mmr.get_merkle_root().unwrap(),
    };
    Block {
        header,
        content,
   }
}

pub fn generate_genesis_block() -> Block {
    let content = Content {
        data: Default::default(),
    };
    let header = Header {
        parent: Default::default(),
        nonce: Default::default(),
        difficulty: <H256>::from([1; 32]), 
        timestamp: Default::default(),
        merkle_root: Default::default(),
        mmr_root: MerkleMountainRange::<Sha256, Vec<Hash>>::new(Vec::new()).get_merkle_root().unwrap(),
    };
    Block {
        header,
        content, 
   }
}

pub fn generate_random_block(parent: &H256, 
    parent_mmr: &MerkleMountainRange<Sha256, Vec<Hash>>) -> Block {
    let mut rng = rand::thread_rng();
    let mut data: Vec<SignedTransaction> = Vec::new();
    let t = generate_random_signed_transaction();
    data.push(t);
    let mt: MerkleTree = MerkleTree::new(&data);
    let content = Content {
        data: data,
    };
    let header = Header {
        parent: *parent,
        nonce: rng.gen(),
        difficulty: hash::generate_random_hash(), 
        timestamp: rng.gen(),
        merkle_root: mt.root(),
        mmr_root: parent_mmr.get_merkle_root().unwrap(),
    };
    Block {
        header,
        content, 
   }
}

#[cfg(any(test, test_utilities))]
pub mod test {
    use super::*;
    use crate::crypto::hash::H256;

    pub fn generate_random_block(parent: &H256, 
        parent_mmr: &MerkleMountainRange<Sha256, Vec<Hash>>) -> Block {
        let mut rng = rand::thread_rng();
        let mut data: Vec<SignedTransaction> = Vec::new();
        let t = generate_random_signed_transaction();
        data.push(t);
        let mt: MerkleTree = MerkleTree::new(&data);
        let content = Content {
            data: data,
        };
        let header = Header {
            parent: *parent,
            nonce: rng.gen(),
            difficulty: hash::generate_random_hash(), 
            timestamp: rng.gen(),
            merkle_root: mt.root(),
            mmr_root: parent_mmr.get_merkle_root().unwrap(),
        };
        Block {
            header,
            content, 
       }
    }
}
