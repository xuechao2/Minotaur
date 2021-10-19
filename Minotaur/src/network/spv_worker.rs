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
use crate::blockchain::Blockchain;
use crate::crypto::hash::{Hashable, H160, H256};
use std::collections::VecDeque;
use std::time::{self, SystemTime, UNIX_EPOCH};
use serde::{Serialize,Deserialize};
use crate::crypto::merkle::{verify};


use log::info;

use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Clone)]
pub struct Context {
    msg_chan: channel::Receiver<(Vec<u8>, peer::Handle)>,
    num_worker: usize,
    server: ServerHandle,
    longestchain: Arc<Mutex<Vec<Block>>>,
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
    longestchain: &Arc<Mutex<Vec<Block>>>,
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
        longestchain: Arc::clone(longestchain), 
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
			    Message::SPVChain(chain) => {
                    // todo: verify the chain
                    // info!("SPVChain");
                    if self.longestchain.lock().unwrap().len() < chain.len() {
                        self.longestchain = Arc::new(std::sync::Mutex::new(chain));
                    }
                }

                Message::SPVTxnProof(block_hash, root, txn_hash, proof, index, leaf_size) => {
                    // info!("SPVTxnProof");
                    //verify if longest chain contains the block
                    let longestchain:Vec<Block> = self.longestchain.lock().unwrap().clone();
                    let mut contains_block = false;
                    for block in longestchain {
                        if block.hash() == block_hash {
                            contains_block = true;
                            break;
                        }
                    }
                    if !contains_block {
                        info!("SPV fails to verify txn {:?}: block {:?} not in longest chain", txn_hash, block_hash);
                        continue;
                    }
                    // verify if the proof is valid
                    if !verify(&root, &txn_hash, &proof, index, leaf_size) {
                        info!("SPV fails to verify txn {:?}: merkle proof verification failed", txn_hash);
                        continue;
                    }
                    info!("SPV succeed to verify txn {:?}", txn_hash);
                }

                _ => {}
            }
        }
    }
}

