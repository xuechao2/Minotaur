use std::collections::HashMap;
use std::collections::HashSet;
use crate::crypto::hash::hash_multiply_by;
use crate::transaction::SignedTransaction;
use crate::transaction::generate_random_transaction;
use crate::block::generate_pos_block;
use crate::block::{Block, Header, Content};
use crate::crypto::merkle::MerkleTree;
use crate::crypto::hash::{H256,H160,Hashable,generate_random_hash};
use crate::transaction::Transaction;
use crate::network::server::Handle as ServerHandle;
use crate::blockchain::Blockchain;
use crate::network::message::Message;
use crate::state::{State,transaction_check,compute_key_hash};


use log::info;
use std::sync::{Arc, Mutex};

use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use std::time;
use std::time::{SystemTime, UNIX_EPOCH};
use std::thread;
use rand::Rng;

use vrf::openssl::{CipherSuite, ECVRF};
use vrf::VRF;   


enum ControlSignal {
    Start(u64), // the number controls the zeta of interval between block generation
    Exit,
}

enum OperatingState {
    Paused,
    Run(u64),
    ShutDown,
}

pub enum ContextUpdateSignal {
    NewPosBlock,
    AttackerParent(H256),
}
pub struct Context {
    /// Channel for receiving control signal
    blockchain: Arc<Mutex<Blockchain>>,
    control_chan: Receiver<ControlSignal>,
    operating_state: OperatingState,
    context_update_recv: Receiver<ContextUpdateSignal>,
    context_update_send: Sender<ContextUpdateSignal>,
    server: ServerHandle,
    //mempool: Arc<Mutex<Vec<SignedTransaction>>>,
    state: Arc<Mutex<State>>,
    all_blocks: Arc<Mutex<HashMap<H256,Block>>>,
    tranpool: Arc<Mutex<Vec<H256>>>,              //Pool of hash of transaction blocks that are not included yet
    vrf_secret_key: Vec<u8>,
    vrf_public_key: Vec<u8>,
    selfish_staker: bool,
    epoch_block_counts: HashMap<u128,(HashMap<Vec<u8>,usize>,f64)>,
    omega: f64,
    beta: f64,
    atttime: u128,
}

#[derive(Clone)]
pub struct Handle {
    /// Channel for sending signal to the staker thread
    control_chan: Sender<ControlSignal>,
}

pub fn new(
    blockchain: &Arc<Mutex<Blockchain>>,
    context_update_recv: Receiver<ContextUpdateSignal>,
    context_update_send: Sender<ContextUpdateSignal>,
    server: &ServerHandle,
    //mempool: &Arc<Mutex<Vec<SignedTransaction>>>,
    state: &Arc<Mutex<State>>,
    all_blocks: &Arc<Mutex<HashMap<H256,Block>>>,
    tranpool: &Arc<Mutex<Vec<H256>>>,
    vrf_secret_key: &Vec<u8>,
    vrf_public_key: &Vec<u8>,
    selfish_staker: bool,
    omega: f64,
    beta: f64,
    atttime: u128,
) -> (Context, Handle) {
    let (signal_chan_sender, signal_chan_receiver) = unbounded();

    let ctx = Context {
        blockchain: Arc::clone(blockchain),
        control_chan: signal_chan_receiver,
        operating_state: OperatingState::Paused,
        context_update_recv,
        context_update_send,
        server: server.clone(),
        //mempool: Arc::clone(mempool),
        state: Arc::clone(state),
        all_blocks: Arc::clone(all_blocks),
        tranpool: Arc::clone(tranpool),
        vrf_secret_key: vrf_secret_key.clone(),
        vrf_public_key: vrf_public_key.clone(),
        selfish_staker: selfish_staker,
        epoch_block_counts: Default::default(),
        omega,
        beta,
        atttime,
    };

    let handle = Handle {
        control_chan: signal_chan_sender,
    };

    (ctx, handle)
}

impl Handle {
    pub fn exit(&self) {
        self.control_chan.send(ControlSignal::Exit).unwrap();
    }

    pub fn start(&self, zeta: u64) {
        self.control_chan
            .send(ControlSignal::Start(zeta))
            .unwrap();
    }

}

impl Context {
    pub fn start(mut self) {
        thread::Builder::new()
            .name("staker".to_string())
            .spawn(move || {
                self.staker_loop();
            })
            .unwrap();
        info!("Staker initialized into paused mode");
    }

    fn handle_control_signal(&mut self, signal: ControlSignal) {
        match signal {
            ControlSignal::Exit => {
                info!("Staker shutting down");
                self.operating_state = OperatingState::ShutDown;
            }
            ControlSignal::Start(i) => {
                info!("Staker starting in continuous mode with zeta {}", i);
                self.operating_state = OperatingState::Run(i);
            }
        }
    }
    
    fn staker_loop(&mut self) {
        // include pow pos and virtual pos
        macro_rules! calc_difficulties {
            ($bc:expr, $ts:expr, $parent:expr) => {
                {
                let current_epoch = $bc.epoch($ts);
                if !self.epoch_block_counts.contains_key(&current_epoch) {
                    let count_pow_blocks = $bc.is_new_epoch_and_count_blocks($ts);
                    if let Some(v) = count_pow_blocks {
                        let set_to_count: HashMap<Vec<u8>, usize> = v.into_iter().map(|(k,v)|(k,v.len())).collect();
                        let self_count = set_to_count.get(&self.vrf_public_key).unwrap_or(&0);
                        let all_count: usize = {
                            let x = set_to_count.iter().map(|(_,v)|*v).sum();
                            if x>0 {
                                x
                            } else {
                                1
                            }
                        };
                        let fraction: f64 = *self_count as f64 / all_count as f64;
                        info!("[New Epoch] the count of blocks in previous epoch: {:?}",set_to_count);
                        self.epoch_block_counts.insert(current_epoch, (set_to_count, fraction));
                    } else if current_epoch == 0 {
                        // special handling
                        self.epoch_block_counts.insert(current_epoch, (Default::default(), 1f64));
                    } else {
                        panic!("not reach here")
                    }
                }
                let pow_difficulty = $bc.get_pow_difficulty($ts,$parent);
                let pos_difficulty = $bc.get_pos_difficulty();
                let (_, virtual_stake_fraction)= self.epoch_block_counts.get(&current_epoch).unwrap();
                // Virtual pos difficulty
                // self.beta is used to conveniently change stake power for experiments
                // if no requirement to change it, remove self.beta
                let virtual_pos = self.omega * virtual_stake_fraction + self.beta * (1f64-self.omega);
                let virtual_pos_difficulty = hash_multiply_by(&pos_difficulty, virtual_pos);
                (pow_difficulty, pos_difficulty, virtual_pos_difficulty)
                }
            }
        }
        let mut count = 0;
        let start: time::SystemTime = SystemTime::now();
        let mut vrf = ECVRF::from_suite(CipherSuite::SECP256K1_SHA256_TAI).unwrap();
        // // Inputs: Secret Key, Public Key (derived) & Message
        // let vrf_secret_key =
        //     hex::decode("c9afa9d845ba75166b5c215767b1d6934e50c3db36e89b127b8a622b120f6721").unwrap();   //TODO: use different vrf key pairs in different nodes
        // let vrf_public_key = vrf.derive_public_key(&vrf_secret_key).unwrap();
        let bc = self.blockchain.lock().unwrap();
        let mut parent = bc.tip();
        let mut parent_depth = bc.get_depth();
        drop(bc);
        // main mining loop
        loop {
            // check and react to control signals
            match self.operating_state {
                OperatingState::Paused => {
                    let signal = self.control_chan.recv().unwrap();
                    self.handle_control_signal(signal);
                    continue;
                }
                OperatingState::ShutDown => {
                    return;
                }
                _ => match self.control_chan.try_recv() {
                    Ok(signal) => {
                        self.handle_control_signal(signal);
                    }
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => panic!("Staker control channel detached"),
                },
            }
            if let OperatingState::ShutDown = self.operating_state {
                return;
            }

            // TODO: actual mining


            // parent = self.blockchain.lock().unwrap().tip();
            let mut ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros();
            let bc = self.blockchain.lock().unwrap();
            let (mut pow_difficulty, mut pos_difficulty, mut virtual_pos_difficulty) = calc_difficulties!(bc, ts, parent);
            drop(bc);
            //let parent_mmr = self.blockchain.lock().unwrap().get_mmr(&parent);
            let mut rng = rand::thread_rng();
            let mut data: Vec<SignedTransaction> = Default::default();
            // add txn_blks from tranpool to form a PoS block
            let txn_block_number = 32;
            let mut enough_txn_block = false;

            let mut transaction_ref: Vec<H256> = Vec::new();
            let mut rand: u128 = Default::default();  // TODO: update rand every epoch
            let ts_slice = ts.to_be_bytes();
            let rand_slice = rand.to_be_bytes();
            let message = [rand_slice,ts_slice].concat();
            // VRF proof and hash output
            let mut vrf_proof = vrf.prove(&self.vrf_secret_key, &message).unwrap();
            let mut vrf_hash = vrf.proof_to_hash(&vrf_proof).unwrap();



            let tran_snap = self.tranpool.lock().unwrap().clone();
            let tran_size = tran_snap.len(); 
            // info!("mem_size {}", mem_size);

            //if  mem_size >= txn_number && self.state.lock().unwrap().check_block(&parent) {
            if  tran_size >= txn_block_number { 
                let txn_blocks = tran_snap.to_vec();
                //let mut current_state = self.state.lock().unwrap().one_block_state(&parent).clone();
                let mut count_txn_block = 0;
                for txn_block in txn_blocks {
                    //if transaction_check(&mut current_state,&txn) {
                    transaction_ref.push(txn_block.clone());
                    count_txn_block = count_txn_block + 1;
                    if count_txn_block == txn_block_number {
                        enough_txn_block = true;
                        break;
                       // }
                    }
                }
            }

            if enough_txn_block || true {
                // info!("Start mining!");

                // update context
                {
                    let mut new_block: bool = false;
                    let mut attacker_update_parent = None;
                    for sig in self.context_update_recv.try_iter() {
                        match sig {
                            ContextUpdateSignal::NewPosBlock=> {
                                new_block = true;
                            }
                            ContextUpdateSignal::AttackerParent(h) => {
                                attacker_update_parent = Some(h);
                            }
                        }
                    }
                    if new_block {
                        ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros();
                        if self.atttime==0 || ts < self.atttime {
                            let bc = self.blockchain.lock().unwrap();
                            parent = bc.tip();
                            parent_depth = bc.get_depth();
                            let tmp = calc_difficulties!(bc, ts, parent);
                            drop(bc);
                            pow_difficulty = tmp.0;
                            pos_difficulty = tmp.1;
                            virtual_pos_difficulty = tmp.2;
                            rand = Default::default();  // TODO: update rand every epoch
                            let ts_slice = ts.to_be_bytes();
                            let rand_slice = rand.to_be_bytes();
                            let message = [rand_slice,ts_slice].concat();
                            // VRF proof and hash output
                            vrf_proof = vrf.prove(&self.vrf_secret_key, &message).unwrap();
                            vrf_hash = vrf.proof_to_hash(&vrf_proof).unwrap();
                        }
                    }
                    if let Some(h) = attacker_update_parent {
                        parent = h;
                        let bc = self.blockchain.lock().unwrap();
                        parent_depth = bc.find_one_depth(&parent).expect("cannot find parent in blockchain!");
                        let tmp = calc_difficulties!(bc, ts, parent);
                        drop(bc);
                        pow_difficulty = tmp.0;
                        pos_difficulty = tmp.1;
                        virtual_pos_difficulty = tmp.2;
                        rand = Default::default();  // TODO: update rand every epoch
                        let ts_slice = ts.to_be_bytes();
                        let rand_slice = rand.to_be_bytes();
                        let message = [rand_slice,ts_slice].concat();
                        // VRF proof and hash output
                        vrf_proof = vrf.prove(&self.vrf_secret_key, &message).unwrap();
                        vrf_hash = vrf.proof_to_hash(&vrf_proof).unwrap();
                    }
                }
                let blk = generate_pos_block(&data, &transaction_ref, &parent, rng.gen(), &pow_difficulty, &pos_difficulty, ts, &vrf_proof, &vrf_hash, 
                      &self.vrf_public_key, rand, self.selfish_staker);
                let vrf_hash_bytes: &[u8] = &vrf_hash;
                let vrf_hash_sha256: H256 = ring::digest::digest(&ring::digest::SHA256, vrf_hash_bytes).into();
                //info!("Vrf: {}",vrf_hash_sha256);
                if vrf_hash_sha256 <= virtual_pos_difficulty {    //TODO: change to PoS mining             
                    info!("Virtual diff: {}, PoS diff: {}",virtual_pos_difficulty,pos_difficulty);
                    let copy = blk.clone();
                    count += 1;
                    info!("Mined {} PoS blocks!", count);
                    info!("Timestamp of the block: {}", copy.header.timestamp);
                    let mut last_longest_chain: Vec<H256> = self.blockchain.lock().unwrap().all_blocks_in_longest_chain();

                    self.all_blocks.lock().unwrap().insert(blk.hash(), blk.clone());

                    if self.blockchain.lock().unwrap().insert_pos(&blk, self.selfish_staker) {
                        //self.state.lock().unwrap().update_block(&blk);
                        // longest chain changes
                        // update the longest chain
                        let mut longest_chain: Vec<H256> = self.blockchain.lock().unwrap().all_blocks_in_longest_chain();
                        // longest_chain.reverse();
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
                                let selfish = self.blockchain.lock().unwrap().find_one_block(&txn_block).unwrap().clone().selfish_block;
                                if !self.tranpool.lock().unwrap().contains(&txn_block) && (selfish || !self.selfish_staker) {
                                    self.tranpool.lock().unwrap().push(txn_block);
                                }
                            }
                        }
                        
                        // remove txn_blocks from the tranpool
                        for b in blocks {
                            let txn_blocks = b.content.transaction_ref;
                            self.tranpool.lock().unwrap().retain(|txn_block| !txn_blocks.contains(txn_block));
                        }

                        
                        //clean up mempool
                        // let mem_snap = self.mempool.lock().unwrap().clone();
                        // let mem_size = mem_snap.len();
                        // let txns = mem_snap.to_vec();
                        // let temp_tip = self.blockchain.lock().unwrap().tip().clone(); 
                        // if self.state.lock().unwrap().check_block(&temp_tip) {
                        //     let temp_state = self.state.lock().unwrap().one_block_state(&temp_tip).clone();
                        //     let mut invalid_txns = Vec::new();
                        //     for txn in txns {
                        //         let copy = txn.clone();
                        //         let pubk = copy.sign.pubk.clone();
                        //         let nonce = copy.transaction.nonce.clone();
                        //         let value = copy.transaction.value.clone();

                        //         let sender: H160 = compute_key_hash(pubk).into();
                        //         let (s_nonce, s_amount) = temp_state.get(&sender).unwrap().clone();
                        //         if s_nonce >= nonce {
                        //             invalid_txns.push(copy.clone());
                        //         }
                        //     }
                        //     self.mempool.lock().unwrap().retain(|txn| !invalid_txns.contains(txn));
                        // }
                        
                    } else {
                        // longest chain not change
                        //self.state.lock().unwrap().update_block(&blk);
                        // add txns back to the mempool
                        // let txns = blk.content.data.clone();
                        // self.mempool.lock().unwrap().extend(txns);

                        // add txn_blocks back to the tranpool
                        let txn_blocks = blk.content.transaction_ref.clone();
                            for txn_block in txn_blocks{
                                if !self.tranpool.lock().unwrap().contains(&txn_block) {
                                    self.tranpool.lock().unwrap().push(txn_block);
                                }
                            }
                    }

                    // copy.print_txns();
                    info!("Longest Blockchain Length: {}", self.blockchain.lock().unwrap().get_depth());
                    if self.selfish_staker {
                         info!("Longest Public Blockchain Length: {}", self.blockchain.lock().unwrap().get_pub_len());
                    }   
                    info!("Total Number of PoS Blocks in Blockchain: {}", self.blockchain.lock().unwrap().get_num_pos());
                    // info!("Total Number of Blocks: {}", self.all_blocks.lock().unwrap().len());
                    let last_block = self.blockchain.lock().unwrap().tip();                    
                    info!("Tranpool size: {}", self.tranpool.lock().unwrap().len());
                    // self.state.lock().unwrap().print_last_block_state(&last_block);
                    // self.blockchain.lock().unwrap().print_longest_chain();
                    ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros();
                    if !self.selfish_staker && (self.atttime == 0 || self.atttime > ts) {
                        self.server.broadcast(Message::NewBlockHashes(vec![blk.hash()]));
                        if self.blockchain.lock().unwrap().get_depth() % 100 == 0 {
                            info!("Chain quality: {}", self.blockchain.lock().unwrap().get_chain_quality());
                        }
                    } else {
                        self.context_update_send.send(ContextUpdateSignal::AttackerParent(blk.hash())).unwrap();
                        info!("[PrivateAttack] generate a block with parent height: {}", parent_depth)
                    }
                    if self.atttime == 0 || self.atttime > ts {
                        self.context_update_send.send(ContextUpdateSignal::NewPosBlock).unwrap();
                    }
                    //break;
                }
            }

            if let OperatingState::Run(i) = self.operating_state {
                if i != 0 {
                    let interval = time::Duration::from_micros(i as u64);
                    thread::sleep(interval);
                }
            }
            if count == 100000 {
                info!("pos_difficulty {}", self.blockchain.lock().unwrap().get_pos_difficulty());
                let time: u64 = SystemTime::now().duration_since(start).unwrap().as_secs();
                info!("{} seconds elapsed", time);
                let rate = 100000/time;
                info!("mining rate {} block/s", rate);
                break;
            }
        }
    }
}
