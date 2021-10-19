use crate::state::{State,compute_key_hash,transaction_check};
use crate::transaction::verify_signedtxn;
use crate::transaction::SignedTransaction;
use std::collections::{HashMap, HashSet};
use super::message::Message;
use super::peer;
use crate::network::server::Handle as ServerHandle;
use crossbeam::channel;
use log::{debug, warn};
use crate::block::Block;
use crate::blockchain::{Blockchain,FlyClientProposal,FlyClientProof,FlyClientQuery};
use crate::crypto::hash::{Hashable, H160, H256};
use std::collections::VecDeque;
use std::time::{self, SystemTime, UNIX_EPOCH};
use serde::{Serialize,Deserialize};
use crate::crypto::merkle::{verify};
use tari_mmr::{MerkleMountainRange, MerkleProof, Hash};
use sha2::{Digest, Sha256};


use log::info;

use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Clone)]
pub struct Context {
    msg_chan: channel::Receiver<(Vec<u8>, peer::Handle)>,
    num_worker: usize,
    server: ServerHandle,
    //longestchain: Arc<Mutex<Vec<Block>>>,
    // buffer: Arc<Mutex<HashMap<H256,Block>>>,
    // all_blocks: Arc<Mutex<HashMap<H256,Block>>>,
    // delays: Arc<Mutex<Vec<u128>>>,
    // mempool: Arc<Mutex<Vec<SignedTransaction>>>,
    // all_txns: Arc<Mutex<HashMap<H256,SignedTransaction>>>,
    // state: Arc<Mutex<State>>,
}

pub fn new(
    num_worker: usize,
    msg_src: channel::Receiver<(Vec<u8>, peer::Handle)>,
    server: &ServerHandle,
    //longestchain: &Arc<Mutex<Vec<Block>>>,
    // buffer: &Arc<Mutex<HashMap<H256,Block>>>,
    // all_blocks: &Arc<Mutex<HashMap<H256,Block>>>,
    // time: &Arc<Mutex<Vec<u128>>>,
    // mempool: &Arc<Mutex<Vec<SignedTransaction>>>,
    // all_txns: &Arc<Mutex<HashMap<H256,SignedTransaction>>>,
    // state: &Arc<Mutex<State>>,
) -> Context {
    Context {
        msg_chan: msg_src,
        num_worker,
        server: server.clone(),
        //longestchain: Arc::clone(longestchain), 
        // blockchain: Arc::clone(blockchain), 
        // buffer: Arc::clone(buffer),
        // all_blocks: Arc::clone(all_blocks),
        // delays: Arc::clone(time),
        // mempool: Arc::clone(mempool),
        // all_txns: Arc::clone(all_txns),
        // state: Arc::clone(state),
    }
}

impl Context {
    pub fn start(self) {
        let num_worker = self.num_worker;
        for i in 0..num_worker {
            let mut cloned = self.clone();
            thread::spawn(move || {
                cloned.worker_loop();
                warn!("Worker thread {} exited", i);
            });
        }
    }

    fn worker_loop(&mut self) {
        loop {
            let msg = self.msg_chan.recv().unwrap();
            let (msg, peer) = msg;
            let msg: Message = bincode::deserialize(&msg).unwrap();
            match msg {
                Message::Ping(nonce) => {
                    debug!("Ping: {}", nonce);
                    peer.write(Message::Pong(nonce.to_string()));
                }
                Message::Pong(nonce) => {
                    debug!("Pong: {}", nonce);
                }
			    Message::FlyChain(proposal,sample_proof) => {
                    // todo: verify the chain
                    // info!("FlyChain");
                    // verify if the sample_proof is valid
                    if !sample_proof.verify(proposal.header.mmr_root) {
                        info!("Flyclient receives invalid chain")
                    } else {
                        info!("Flyclient receives valid chain with depth {:?}", proposal.chain_depth)
                    }
                }

                Message::FlyTxnProof(proposal, block_proof, txn_hash, txn_proof, txn_sample, txn_num, txn_root) => {
                    // info!("FlyTxnProof");
                    //verify if block proof is valid                  
                    if !block_proof.verify(proposal.header.mmr_root) {
                        info!("Flyclient receives invalid chain");
                        continue;
                    }

                    // verify if the txn proof is valid
                    if !verify(&txn_root, &txn_hash, &txn_proof, txn_sample, txn_num) {
                        info!("Fly client fails to verify txn {:?}: merkle proof verification failed", txn_hash);
                        continue;
                    }
                    info!("Fly client succeeds to verify txn {:?}", txn_hash);
                }

                _ => {}
            }
        }
    }
}

