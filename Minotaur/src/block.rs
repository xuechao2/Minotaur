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
    pub block_type: bool,  // true for PoS, false for PoW
}

#[derive(Serialize, Deserialize, Debug,Hash, Eq, PartialEq,Clone)]
pub struct Header {
    pub parent: H256,
    pub nonce: u32,
    pub pow_difficulty: H256,
    pub pos_difficulty: H256,
    pub timestamp: u128,  // TODO: use current time
    pub merkle_root: H256,
    //pub mmr_root: Hash,  //ignore this for now
    pub vrf_proof: Vec<u8>,
    pub vrf_hash: Vec<u8>,
    pub vrf_pub_key: Vec<u8>,
    pub rand: u128,     // randomness for PoS leader election. TODO: update rand every epoch 
}

#[derive(Serialize, Deserialize, Debug,Hash, Eq, PartialEq,Clone)]
pub struct Content {
    pub data: Vec<SignedTransaction>,
    pub transaction_ref: Vec<H256>,
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



pub fn generate_pos_block(data: &Vec<SignedTransaction>, transaction_ref: &Vec<H256>, parent: &H256, nonce: u32, pow_difficulty: &H256, pos_difficulty: &H256,
                      timestamp: u128, vrf_proof: &Vec<u8>, vrf_hash: &Vec<u8>, 
                      vrf_pub_key: &[u8], rand: u128) -> Block {
    let mt: MerkleTree = MerkleTree::new(transaction_ref);
    let block_type = true; 
    let content = Content {
        data: data.to_vec(),
        transaction_ref: transaction_ref.to_vec()
    };
    let header = Header {
        parent: *parent,
        nonce: nonce,
        pow_difficulty: *pow_difficulty, 
        pos_difficulty: *pos_difficulty, 
        timestamp: timestamp,
        merkle_root: mt.root(),
        //mmr_root: parent_mmr.get_merkle_root().unwrap(),
        vrf_proof: vrf_proof.to_vec(),
        vrf_hash: vrf_hash.to_vec(),
        vrf_pub_key: vrf_pub_key.to_vec(),
        rand: rand,
    };
    Block {
        header,
        content,
        block_type
   }
}

pub fn generate_pow_block(data: &Vec<SignedTransaction>, transaction_ref: &Vec<H256>, parent: &H256, nonce: u32, pow_difficulty: &H256, pos_difficulty: &H256, 
                      timestamp: u128, vrf_proof: &Vec<u8>, vrf_hash: &Vec<u8>, 
                      vrf_pub_key: &[u8], rand: u128) -> Block {
    let mt: MerkleTree = MerkleTree::new(data);
    let block_type = false; 
    let content = Content {
        data: data.to_vec(),
        transaction_ref: transaction_ref.to_vec()
    };
    let header = Header {
        parent: *parent,
        nonce: nonce,
        pow_difficulty: *pow_difficulty, 
        pos_difficulty: *pos_difficulty,
        timestamp: timestamp,
        merkle_root: mt.root(),
        //mmr_root: parent_mmr.get_merkle_root().unwrap(),
        vrf_proof: vrf_proof.to_vec(),
        vrf_hash: vrf_hash.to_vec(),
        vrf_pub_key: vrf_pub_key.to_vec(),
        rand: rand,
    };
    Block {
        header,
        content,
        block_type
   }
}

pub fn generate_genesis_block(initial_time:u128) -> Block {
    let content = Content {
        data: Default::default(),
        transaction_ref: Default::default(),
    };
    let block_type = true;
    let header = Header {
        parent: Default::default(),
        nonce: Default::default(),
        //pow_difficulty: <H256>::from([1; 32]), 
        pow_difficulty: <H256>::from([
            0, 0, 25, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
        ]),
        pos_difficulty: <H256>::from([1; 32]), 
        timestamp: initial_time,
        merkle_root: Default::default(),
        // mmr_root: MerkleMountainRange::<Sha256, Vec<Hash>>::new(Vec::new()).get_merkle_root().unwrap(),
        vrf_proof: Default::default(),
        vrf_hash: Default::default(),
        vrf_pub_key: Default::default(),
        rand: Default::default(),
    };
    Block {
        header,
        content, 
        block_type
   }
}

// pub fn generate_random_block(parent: &H256, 
//     parent_mmr: &MerkleMountainRange<Sha256, Vec<Hash>>) -> Block {
//     let mut rng = rand::thread_rng();
//     let mut data: Vec<SignedTransaction> = Vec::new();
//     let t = generate_random_signed_transaction();
//     data.push(t);
//     let mt: MerkleTree = MerkleTree::new(&data);
//     let content = Content {
//         data: data,
//     };
//     let header = Header {
//         parent: *parent,
//         nonce: rng.gen(),
//         difficulty: hash::generate_random_hash(), 
//         timestamp: rng.gen(),
//         merkle_root: mt.root(),
//         mmr_root: parent_mmr.get_merkle_root().unwrap(),
//     };
//     Block {
//         header,
//         content, 
//    }
// }

// #[cfg(any(test, test_utilities))]
// pub mod test {
//     use super::*;
//     use crate::crypto::hash::H256;

//     pub fn generate_random_block(parent: &H256, 
//         parent_mmr: &MerkleMountainRange<Sha256, Vec<Hash>>) -> Block {
//         let mut rng = rand::thread_rng();
//         let mut data: Vec<SignedTransaction> = Vec::new();
//         let t = generate_random_signed_transaction();
//         data.push(t);
//         let mt: MerkleTree = MerkleTree::new(&data);
//         let content = Content {
//             data: data,
//         };
//         let header = Header {
//             parent: *parent,
//             nonce: rng.gen(),
//             difficulty: hash::generate_random_hash(), 
//             timestamp: rng.gen(),
//             merkle_root: mt.root(),
//             mmr_root: parent_mmr.get_merkle_root().unwrap(),
//         };
//         Block {
//             header,
//             content, 
//        }
//     }
// }
