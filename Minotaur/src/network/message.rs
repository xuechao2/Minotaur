use crate::transaction::SignedTransaction;
use serde::{Serialize, Deserialize};
use crate::crypto::hash::H256;
use crate::block::Block;
use tari_mmr::{MerkleMountainRange, MerkleProof, Hash};
use sha2::{Digest, Sha256};
use crate::blockchain::{FlyClientProposal,FlyClientProof,FlyClientQuery};

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
    // spv client
    SPVGetChain(),
    SPVChain(Vec<Block>),
    SPVVerifyTxn(H256, H256),
    SPVVerifyRandomTxn(),
    SPVTxnProof(H256, H256, H256, Vec<H256>, usize, usize),
    //fly client
    FlyGetChain(),
    FlyChain(FlyClientProposal,FlyClientProof),
    FlyVerifyRandomTxn(),
    FlyTxnProof(FlyClientProposal, FlyClientProof, H256, Vec<H256>, usize, usize,H256)
}
