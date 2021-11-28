use crate::crypto::merkle::{MerkleTree, verify};
use crate::miner;
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
use rand::Rng;
use tari_mmr::{MerkleMountainRange, MerkleProof, Hash};
use sha2::{Digest, Sha256};


use log::info;

use std::sync::{Arc, Mutex};
use std::thread;

use vrf::openssl::{CipherSuite, ECVRF};
use vrf::VRF;   

#[derive(Clone)]
pub struct Context {
    msg_chan: channel::Receiver<(Vec<u8>, peer::Handle)>,
    num_worker: usize,
    server: ServerHandle,
    blockchain: Arc<Mutex<Blockchain>>,
    buffer: Arc<Mutex<HashMap<H256,Block>>>,
    all_blocks: Arc<Mutex<HashMap<H256,Block>>>,
    delays: Arc<Mutex<Vec<u128>>>,
    mempool: Arc<Mutex<Vec<SignedTransaction>>>,
    all_txns: Arc<Mutex<HashMap<H256,SignedTransaction>>>,
    state: Arc<Mutex<State>>,
    tranpool: Arc<Mutex<Vec<H256>>>,  
    context_update_send: channel::Sender<miner::ContextUpdateSignal>,
}

pub fn new(
    num_worker: usize,
    msg_src: channel::Receiver<(Vec<u8>, peer::Handle)>,
    server: &ServerHandle,
    blockchain: &Arc<Mutex<Blockchain>>,
    buffer: &Arc<Mutex<HashMap<H256,Block>>>,
    all_blocks: &Arc<Mutex<HashMap<H256,Block>>>,
    time: &Arc<Mutex<Vec<u128>>>,
    mempool: &Arc<Mutex<Vec<SignedTransaction>>>,
    all_txns: &Arc<Mutex<HashMap<H256,SignedTransaction>>>,
    state: &Arc<Mutex<State>>,
    tranpool: &Arc<Mutex<Vec<H256>>>,
    context_update_send: channel::Sender<miner::ContextUpdateSignal>,
) -> Context {
    Context {
        msg_chan: msg_src,
        num_worker,
        server: server.clone(),
        blockchain: Arc::clone(blockchain), 
        buffer: Arc::clone(buffer),
        all_blocks: Arc::clone(all_blocks),
        delays: Arc::clone(time),
        mempool: Arc::clone(mempool),
        all_txns: Arc::clone(all_txns),
        state: Arc::clone(state),
        tranpool: Arc::clone(tranpool),
        context_update_send,
    }
}

impl Context {
    pub fn start(self) {
        let num_worker = self.num_worker;
        for i in 0..num_worker {
            let cloned = self.clone();
            thread::spawn(move || {
                cloned.worker_loop();
                warn!("Worker thread {} exited", i);
            });
        }
    }

    fn worker_loop(&self) {
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
			    Message::NewBlockHashes(hashes) => {
                    //let tmp = hashes.clone();
                    let mut hashes_request = vec![];
                    // let all_blocks = self.blockchain.lock().unwrap().all_blocks_in_longest_chain();

				    for hash in hashes {
						if !self.all_blocks.lock().unwrap().contains_key(&hash) {
					    	hashes_request.push(hash);
						}
                    }

					if !hashes_request.is_empty() {
                        peer.write(Message::GetBlocks(hashes_request));
                        //self.server.broadcast(Message::NewBlockHashes(tmp));
                    }
                }
                Message::GetBlocks(hashes) => {
                    debug!("Receive GetBlocks hash {:?}!", hashes);
                    // let all_blocks = self.blockchain.lock().unwrap().all_blocks_in_longest_chain();
                    let mut blocks = vec![];
                    for hash in hashes {
                        if self.all_blocks.lock().unwrap().contains_key(&hash) {
                            let blk = self.all_blocks.lock().unwrap().get(&hash).expect("Message::GetBlocks error").clone();
                            debug!("find block hash {:?}!", blk.hash());
                            blocks.push(blk);
                        }
                    }
                    if !blocks.is_empty() {
                        peer.write(Message::Blocks(blocks));
                    }
                }

                

                Message::Blocks(blks) => {
                    let mut queue: VecDeque<Block> = VecDeque::new();
                    let mut hashes_send = vec![];

                    let mut vrf = ECVRF::from_suite(CipherSuite::SECP256K1_SHA256_TAI).unwrap();

                    for blk in blks {
                        //verify txns inside blks
                        // let mut flag = false;
                        // for txn in &blk.content.data {
                        //     if !verify_signedtxn(&txn) {
                        //         flag = true;
                        //         break;
                        //     }
                        // }
                        // if flag {
                        //     break;
                        // }
                        let copy = blk.clone();
                        self.all_blocks.lock().unwrap().insert(copy.hash(), copy);

                        // let serialized: Vec<u8> = bincode::serialize(&blk).unwrap();
                        // info!("block size {}", serialized.len());

                        let time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros()-blk.header.timestamp;
                        debug!("now {}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros());
                        debug!("ts {}", blk.header.timestamp);
                        debug!("delay {}", time);
                        self.delays.lock().unwrap().push(time);
                    	hashes_send.push(blk.hash());
                        queue.push_back(blk);
                    }
                    self.server.broadcast(Message::NewBlockHashes(hashes_send));
                    let mut hashes_request = vec![];
                    while !queue.is_empty() {
                        let blk = queue.pop_front().unwrap();
                        let parent = blk.header.parent;
                        let blk_type = blk.block_type;
                        if blk_type {
                            let ts_slice = blk.header.timestamp.to_be_bytes();
                            let rand_slice = blk.header.rand.to_be_bytes();
                            let message = [rand_slice,ts_slice].concat();
                            let vrf_pk: &[u8] = &blk.header.vrf_pub_key;
                            let vrf_beta = vrf.verify(&blk.header.vrf_pub_key, &blk.header.vrf_proof, &message);

                            match vrf_beta {
                                Ok(vrf_beta) => {
                                    let vrf_hash_bytes: &[u8] = &blk.header.vrf_hash;
                                    let vrf_hash_sha256: H256 = ring::digest::digest(&ring::digest::SHA256, vrf_hash_bytes).into();
                                    if vrf_hash_sha256 <= blk.header.pos_difficulty && blk.header.pos_difficulty == self.blockchain.lock().unwrap().get_pos_difficulty() 
                                    && blk.header.vrf_hash == vrf_beta  {
                                        //if self.blockchain.lock().unwrap().contains_hash(&parent) && self.state.lock().unwrap().check_block(&parent) { //blockchain has the parent
                                            //let mut current_state = self.state.lock().unwrap().one_block_state(&parent).clone();
                                        if self.blockchain.lock().unwrap().contains_hash(&parent) {
                                            let txn_blocks = blk.content.transaction_ref.clone();

                                            let mut last_longest_chain: Vec<H256> = self.blockchain.lock().unwrap().all_blocks_in_longest_chain();
                                            if self.blockchain.lock().unwrap().insert_pos(&blk) {
                                                //self.state.lock().unwrap().update_block(&blk);
                                                // longest chain changes
                                                // update the longest chain
                                                let mut longest_chain: Vec<H256> = self.blockchain.lock().unwrap().all_blocks_in_longest_chain();
                                                // remove the common prefix
                                                while last_longest_chain.len()>0 && longest_chain.len()>0 && last_longest_chain[0]==longest_chain[0] {
                                                    last_longest_chain.remove(0);
                                                    longest_chain.remove(0);
                                                }
                                                let mut blocks = Vec::new();
                                                // update the state
                                                for blk_hash in longest_chain {
                                                    let block = self.blockchain.lock().unwrap().find_one_block(&blk_hash).unwrap();
                                                    blocks.push(block);
                                                }
                                                // self.state.lock().unwrap().update_blocks(&blocks);
                                                

                                                // add txn_blocks back to the tranpool
                                                for blk_hash in last_longest_chain {
                                                    let block = self.blockchain.lock().unwrap().find_one_block(&blk_hash).unwrap();
                                                    let txn_blocks = block.content.transaction_ref.clone();
                                                    for txn_block in txn_blocks{
                                                        if !self.tranpool.lock().unwrap().contains(&txn_block) {
                                                            self.tranpool.lock().unwrap().push(txn_block);
                                                        }
                                                    }
                                                }

                                                // remove txn_blocks from the tranpool
                                                for blk in blocks {
                                                    let txn_blocks = blk.content.transaction_ref;
                                                    self.tranpool.lock().unwrap().retain(|txn_block| !txn_blocks.contains(txn_block));
                                                }

                                                
                                            }
                                        } else if self.buffer.lock().unwrap().contains_key(&parent) { // buffer has the parent
                                            let parent_blk = self.buffer.lock().unwrap().get(&parent).unwrap().clone();
                                            self.buffer.lock().unwrap().remove(&parent);
                                            queue.push_back(parent_blk);
                                            queue.push_back(blk);
                                        } else {
                                            let mut flag = false;
                                            for i in 0..queue.len() {
                                                let block = queue.get(i).unwrap();
                                                if block.header.hash() == parent {
                                                    flag = true;
                                                    break;
                                                }
                                            }
                                            // if queue contains the parent
                                            if flag {
                                                queue.push_back(blk);
                                            } else {
                                                if !hashes_request.contains(&blk.header.parent) {
                                                    hashes_request.push(blk.header.parent);
                                                }
                                                self.buffer.lock().unwrap().insert(blk.hash(), blk);
                                            }
                                        }
                                    }
                                    // tell the miner to update the context
                                    self.context_update_send.send(miner::ContextUpdateSignal::NewBlock).unwrap();
                                }
                                Err(e) => {
                                    println!("VRF proof is not valid: {}", e);
                                }
                            }
                        } else {
                            if blk.hash() <= blk.header.pow_difficulty && blk.header.pow_difficulty == self.blockchain.lock().unwrap().get_pow_difficulty(blk.header.timestamp) {
                                if self.blockchain.lock().unwrap().contains_hash(&parent) {
                                    self.blockchain.lock().unwrap().insert_pow(&blk);
                                    let txns = blk.content.data.clone();
                                    let hash = blk.hash().clone();
                                    self.mempool.lock().unwrap().retain(|txn| !txns.contains(txn));
                                    if !self.tranpool.lock().unwrap().contains(&hash){
                                        self.tranpool.lock().unwrap().push(hash);
                                    }

                                } else if self.buffer.lock().unwrap().contains_key(&parent) { // buffer has the parent
                                    let parent_blk = self.buffer.lock().unwrap().get(&parent).unwrap().clone();
                                    self.buffer.lock().unwrap().remove(&parent);
                                    queue.push_back(parent_blk);
                                    queue.push_back(blk);
                                } else {
                                    let mut flag = false;
                                    for i in 0..queue.len() {
                                        let block = queue.get(i).unwrap();
                                        if block.header.hash() == parent {
                                            flag = true;
                                            break;
                                        }
                                    }
                                    // if queue contains the parent
                                    if flag {
                                        queue.push_back(blk);
                                    } else {
                                        if !hashes_request.contains(&blk.header.parent) {
                                            hashes_request.push(blk.header.parent);
                                        }
                                        self.buffer.lock().unwrap().insert(blk.hash(), blk);
                                    }
                                }
                                // tell the miner to update the context
                                self.context_update_send.send(miner::ContextUpdateSignal::NewBlock).unwrap();
                            }
                        }

                    }
                        

                    if !hashes_request.is_empty() {
                        peer.write(Message::GetBlocks(hashes_request));
                    }

                    let mut total_delay = 0;
                    let tmp: Vec<u128> = self.delays.lock().unwrap().clone();
                    let size = tmp.len() as u128;
                    for delay in tmp {
                        total_delay += delay;
                    }

                    debug!("Buffer size {}", self.buffer.lock().unwrap().len());
                    debug!("Blockchain size {}", self.blockchain.lock().unwrap().get_depth());

                    info!("Longest Blockchain Length: {}", self.blockchain.lock().unwrap().get_depth());
                    info!("Total Number of PoW Blocks in Blockchain: {}", self.blockchain.lock().unwrap().get_num_pow());
                    info!("Total Number of PoS Blocks in Blockchain: {}", self.blockchain.lock().unwrap().get_num_pos());
                    // info!("Total Number of Blocks: {}", self.all_blocks.lock().unwrap().len());

                    let last_block = self.blockchain.lock().unwrap().tip();                    
                    info!("Mempool size: {}", self.mempool.lock().unwrap().len());
                    info!("tranpool size: {}", self.tranpool.lock().unwrap().len());
                    // self.state.lock().unwrap().print_last_block_state(&last_block);
                    // debug!("Total Block Delay:{}", total_delay);
                    // info!("Avg Block Delay:{}", total_delay/size);
                    // self.blockchain.lock().unwrap().print_longest_chain();
                }



                Message::NewTransactionHashes(hashes) => {
                    let mut hashes_request = vec![];
                    // info!("Receive new tx hash");

				    for hash in hashes {
						if !self.all_txns.lock().unwrap().contains_key(&hash) {
					    	hashes_request.push(hash);
						}
                    }

					if !hashes_request.is_empty() {
                        peer.write(Message::GetTransactions(hashes_request));
                    }
                }

                Message::GetTransactions(hashes) => {
                    debug!("Receive GetTransactions hash {:?}!", hashes);
                    let mut txns = vec![];
                    for hash in hashes {
                        if self.all_txns.lock().unwrap().contains_key(&hash) {
                            let txn = self.all_txns.lock().unwrap().get(&hash).expect("Message::GetTransactions Error").clone();
                            txns.push(txn);
                            debug!("find txn hash {:?}!", hash);
                        }
                    }

                    if !txns.is_empty() {
                        peer.write(Message::Transactions(txns));
                    }
                }

                Message::Transactions(txns) => {
                    let mut hashes_send = vec![];
                    for txn in txns {
                        let copy = txn.clone();
                        self.all_txns.lock().unwrap().insert(txn.hash(), txn);
                        hashes_send.push(copy.clone().hash());
                        if!self.mempool.lock().unwrap().contains(&copy) {
                            self.mempool.lock().unwrap().push(copy.clone());
                        }

                    }

                    // let temp_tip = self.blockchain.lock().unwrap().tip().clone(); 
                    // if self.state.lock().unwrap().check_block(&temp_tip) {
                    //     //let temp_state = self.state.lock().unwrap().one_block_state(&temp_tip).clone();
                    //     for txn in txns {
                    //         //if verify_signedtxn(&txn) {
                    //         if true {
                    //             let copy = txn.clone();
                    //             // let pubk = copy.sign.pubk.clone();
                    //             // let nonce = copy.transaction.nonce.clone();
                    //             // let value = copy.transaction.value.clone();

                    //             // let sender: H160 = compute_key_hash(pubk).into();
                    //             // let (s_nonce, s_amount) = temp_state.get(&sender).unwrap().clone();
                    //             // if s_nonce < nonce {
                    //             //     self.mempool.lock().unwrap().push(copy.clone());
                    //             // }
                    //             self.all_txns.lock().unwrap().insert(txn.hash(), txn);
                    //             // info!("Mempool size: {}", self.mempool.lock().unwrap().len());
                    //             hashes_send.push(copy.clone().hash());
                    //         }
                    //     }
                    //     self.server.broadcast(Message::NewTransactionHashes(hashes_send));
                    // }

                }


                Message::SPVGetChain() => {
                    debug!("Receive SPVGetChain");
                    let longest_chain = self.blockchain.lock().unwrap().get_longest_chain();

                    if !longest_chain.is_empty() {
                        peer.write(Message::SPVChain(longest_chain));
                    }
                }

                Message::SPVVerifyTxn(block_hash, txn_hash) => {
                    debug!("Receive SPVVerifyTxn");
                    let longest_chain_hash: Vec<H256> = self.blockchain.lock().unwrap().all_blocks_in_longest_chain();
                    let mut contains_block = false;
                    for hash in longest_chain_hash {
                        if block_hash == hash {
                            contains_block = true;
                            break;
                        }
                    }
                    if !contains_block {
                        info!("-------------Block {:?} not found in longest chain when SPVVerifyTxn-------------", block_hash);
                    } else {
                        let block = self.blockchain.lock().unwrap().find_one_block(&block_hash).unwrap();
                        let txns = block.content.data.clone();
                        let mut contains_txn = false;
                        for i in 0..txns.len() {
                            if txn_hash == txns[i].hash() {
                                contains_txn = true;
                                let txn_num = txns.len();

                                let mt: MerkleTree = MerkleTree::new(&txns);
                                let proof = mt.proof(i);
                                let root = block.header.merkle_root;

                                peer.write(Message::SPVTxnProof(block_hash, root, txn_hash, proof, i, txn_num));
                                break;
                            }
                        }
                        if !contains_txn {
                            info!("-------------Txn {:?} not found in longest chain when SPVVerifyTxn-------------", txn_hash);
                        }
                    }
                    
                }

                Message::SPVVerifyRandomTxn() => {
                    debug!("Receive SPVVerifyRandomTxn");
                    let longest_chain_hash: Vec<H256> = self.blockchain.lock().unwrap().all_blocks_in_longest_chain();

                    if longest_chain_hash.len() <= 11 {
                        info!("-------------Chain not long enough yet-------------");
                    } else {
                        let last_stable: usize = longest_chain_hash.len()-10;
                        let mut rng = rand::thread_rng();
                        let random_block_index: usize = rng.gen_range(1, last_stable);

                        let block = self.blockchain.lock().unwrap().find_one_block(&longest_chain_hash[random_block_index]).unwrap();

                        let txns = block.content.data.clone();
                        let txn_num = txns.len();
                        let random_txn_index: usize = rng.gen_range(0, txn_num);
                        let txn = txns[random_txn_index].clone();
                        let txn_hash = txn.hash();

                        let mt: MerkleTree = MerkleTree::new(&txns);

                        let proof = mt.proof(random_txn_index);

                        let block_hash = block.hash();
                        let root = block.header.merkle_root;

                        // info!("-------------verification result:{}-------------", verify(&root, &txn_hash, &proof, random_txn_index, txn_num));

                        peer.write(Message::SPVTxnProof(block_hash, root, txn_hash, proof, random_txn_index, txn_num));
                    }
                }

                Message::FlyGetChain() => {
                    debug!("Receive FlyGetChain");
                    let longest_chain_hash: Vec<H256> = self.blockchain.lock().unwrap().all_blocks_in_longest_chain();
                    if longest_chain_hash.len() <= 3 {
                        info!("-------------Chain not long enough yet-------------");
                    } else {
                        let proposal: FlyClientProposal = FlyClientProposal::new(&self.blockchain.lock().unwrap());
                        let mut rng = rand::thread_rng();
                        let sample: usize = rng.gen_range(0, proposal.chain_depth-2);
                        let query: FlyClientQuery = FlyClientQuery::new(proposal.chain_depth, vec![sample]);
                        let proof: FlyClientProof = FlyClientProof::new(&self.blockchain.lock().unwrap(), sample, query.query_depth);
                        peer.write(Message::FlyChain(proposal,proof));
                    }
                }

                Message::FlyVerifyRandomTxn() => {
                    debug!("Receive FlyVerifyRandomTxn");
                    let longest_chain_hash: Vec<H256> = self.blockchain.lock().unwrap().all_blocks_in_longest_chain();

                    if longest_chain_hash.len() <= 3 {
                        info!("-------------Chain not long enough yet-------------");
                    } else {
                        let proposal: FlyClientProposal = FlyClientProposal::new(&self.blockchain.lock().unwrap());
                        let mut rng = rand::thread_rng();
                        let block_sample: usize = rng.gen_range(0, proposal.chain_depth-2);
                        let query: FlyClientQuery = FlyClientQuery::new(proposal.chain_depth, vec![block_sample]);
                        let block_proof: FlyClientProof = FlyClientProof::new(&self.blockchain.lock().unwrap(), block_sample, query.query_depth);

                        let block = self.blockchain.lock().unwrap().find_one_block(&longest_chain_hash[block_sample+1]).unwrap();
                        let txns = block.content.data.clone();
                        let txn_num = txns.len();
                        let txn_sample: usize = rng.gen_range(0, txn_num);
                        let txn = txns[txn_sample].clone();
                        let txn_hash = txn.hash();

                        let mt: MerkleTree = MerkleTree::new(&txns);

                        let txn_proof = mt.proof(txn_sample);

                        //let block_hash = block.hash();
                        let txn_root = block.header.merkle_root;

                        // info!("-------------verification result:{}-------------", verify(&root, &txn_hash, &proof, random_txn_index, txn_num));

                        peer.write(Message::FlyTxnProof(proposal, block_proof, txn_hash, txn_proof, txn_sample, txn_num, txn_root));
                    }
                }
                
                
                _ => {}
            }
        }
    }
}
