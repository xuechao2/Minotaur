use std::collections::HashMap;
use crate::transaction::SignedTransaction;
use crate::transaction::generate_random_transaction;
use crate::block::generate_pow_block;
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

//use vrf::openssl::{CipherSuite, ECVRF};
//use vrf::VRF;   


enum ControlSignal {
    Start(u64), // the number controls the lambda of interval between block generation
    Exit,
}

enum OperatingState {
    Paused,
    Run(u64),
    ShutDown,
}

pub struct Context {
    /// Channel for receiving control signal
    blockchain: Arc<Mutex<Blockchain>>,
    control_chan: Receiver<ControlSignal>,
    operating_state: OperatingState,
    server: ServerHandle,
    mempool: Arc<Mutex<Vec<SignedTransaction>>>,
    state: Arc<Mutex<State>>,
    all_blocks: Arc<Mutex<HashMap<H256,Block>>>,
}

#[derive(Clone)]
pub struct Handle {
    /// Channel for sending signal to the miner thread
    control_chan: Sender<ControlSignal>,
}

pub fn new(
    blockchain: &Arc<Mutex<Blockchain>>,
    server: &ServerHandle,
    mempool: &Arc<Mutex<Vec<SignedTransaction>>>,
    state: &Arc<Mutex<State>>,
    all_blocks: &Arc<Mutex<HashMap<H256,Block>>>,
) -> (Context, Handle) {
    let (signal_chan_sender, signal_chan_receiver) = unbounded();

    let ctx = Context {
        blockchain: Arc::clone(blockchain),
        control_chan: signal_chan_receiver,
        operating_state: OperatingState::Paused,
        server: server.clone(),
        mempool: Arc::clone(mempool),
        state: Arc::clone(state),
        all_blocks: Arc::clone(all_blocks),
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

    pub fn start(&self, lambda: u64) {
        self.control_chan
            .send(ControlSignal::Start(lambda))
            .unwrap();
    }

}

impl Context {
    pub fn start(mut self) {
        thread::Builder::new()
            .name("miner".to_string())
            .spawn(move || {
                self.miner_loop();
            })
            .unwrap();
        info!("Miner initialized into paused mode");
    }

    fn handle_control_signal(&mut self, signal: ControlSignal) {
        match signal {
            ControlSignal::Exit => {
                info!("Miner shutting down");
                self.operating_state = OperatingState::ShutDown;
            }
            ControlSignal::Start(i) => {
                info!("Miner starting in continuous mode with lambda {}", i);
                self.operating_state = OperatingState::Run(i);
            }
        }
    }

    fn miner_loop(&mut self) {
        let mut count = 0;
        let start: time::SystemTime = SystemTime::now();
        // let mut vrf = ECVRF::from_suite(CipherSuite::SECP256K1_SHA256_TAI).unwrap();
        // Inputs: Secret Key, Public Key (derived) & Message
        // let vrf_secret_key =
        //     hex::decode("c9afa9d845ba75166b5c215767b1d6934e50c3db36e89b127b8a622b120f6721").unwrap();
        // let vrf_public_key = vrf.derive_public_key(&vrf_secret_key).unwrap();
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
                    Err(TryRecvError::Disconnected) => panic!("Miner control channel detached"),
                },
            }
            if let OperatingState::ShutDown = self.operating_state {
                return;
            }

            // TODO: actual mining


            let parent = self.blockchain.lock().unwrap().tip();
            let difficulty = self.blockchain.lock().unwrap().get_difficulty();
            let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros();
            // let parent_mmr = self.blockchain.lock().unwrap().get_mmr(&parent);
            let mut rng = rand::thread_rng();
            let mut data: Vec<SignedTransaction> = Vec::new();
            // add txns from mempool to from a block
            let txn_number = 256;
            let mut enough_txn = false;

            let mut transaction_ref = Default::default();
            let rand: u128 = Default::default();  // TODO: update rand every epoch
            // let ts_slice = ts.to_be_bytes();
            // let rand_slice = rand.to_be_bytes();
            // let message = [rand_slice,ts_slice].concat();
            // VRF proof and hash output
            //let vrf_proof = vrf.prove(&vrf_secret_key, &message).unwrap();
            //let vrf_hash = vrf.proof_to_hash(&vrf_proof).unwrap();
            let vrf_proof = Default::default();
            let vrf_hash = Default::default();
            let vrf_public_key:Vec<u8> = Default::default();



            let mem_snap = self.mempool.lock().unwrap().clone();
            let mem_size = mem_snap.len(); 
            // info!("mem_size {}", mem_size);

            //if  mem_size >= txn_number && self.state.lock().unwrap().check_block(&parent) {
            if  mem_size >= txn_number { 
                let txns = mem_snap.to_vec();
                //let mut current_state = self.state.lock().unwrap().one_block_state(&parent).clone();
                let mut count_txn = 0;
                for txn in txns {
                    //if transaction_check(&mut current_state,&txn) {
                    data.push(txn.clone());
                    count_txn = count_txn + 1;
                    if count_txn == txn_number {
                        enough_txn = true;
                        break;
                       // }
                    }
                }
            }

            while enough_txn {
                // info!("Start mining!");

                let blk = generate_pow_block(&data, &transaction_ref, &parent, rng.gen(), &difficulty, ts, &vrf_proof, &vrf_hash, &vrf_public_key, rand);
                if blk.hash() <= difficulty {
                    let copy = blk.clone();
                    count += 1;
                    info!("Mined {} block!", count);
                    let mut last_longest_chain: Vec<H256> = self.blockchain.lock().unwrap().all_blocks_in_longest_chain();

                    self.all_blocks.lock().unwrap().insert(blk.hash(), blk.clone());

                    if self.blockchain.lock().unwrap().insert(&blk) {
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
                        // let mut blocks = Vec::new();
                        // update the state
                    
                        // self.state.lock().unwrap().update_blocks(&blocks);
                        

                        // add txns back to the mempool
                        for blk_hash in last_longest_chain {
                            let block = self.blockchain.lock().unwrap().find_one_block(&blk_hash).unwrap();
                            let txns = block.content.data.clone();
                            self.mempool.lock().unwrap().extend(txns);
                        }

                        // remove txns from mempool
                        for blk_hash in longest_chain {
                            let block = self.blockchain.lock().unwrap().find_one_block(&blk_hash).unwrap();
                            let txns = block.content.data.clone();
                            self.mempool.lock().unwrap().retain(|txn| !txns.contains(txn));
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
                        //let txns = blk.content.data.clone();
                        //self.mempool.lock().unwrap().extend(txns);
                    }

                    // copy.print_txns();
                    info!("Longest Blockchain Length: {}", self.blockchain.lock().unwrap().get_depth());
                    info!("Total Number of Blocks in Blockchain: {}", self.blockchain.lock().unwrap().get_size());
                    // info!("Total Number of Blocks: {}", self.all_blocks.lock().unwrap().len());
                    let last_block = self.blockchain.lock().unwrap().tip();                    
                    info!("Mempool size: {}", self.mempool.lock().unwrap().len());
                    // self.state.lock().unwrap().print_last_block_state(&last_block);
                    self.blockchain.lock().unwrap().print_longest_chain();
                    self.server.broadcast(Message::NewBlockHashes(vec![blk.hash()]));
                    break;
                }
            }

            if let OperatingState::Run(i) = self.operating_state {
                if i != 0 {
                    let interval = time::Duration::from_micros(i as u64);
                    thread::sleep(interval);
                }
            }
            let time: u64 = SystemTime::now().duration_since(start).unwrap().as_secs();
            if time > 600 {
                //info!("difficulty {}", self.blockchain.lock().unwrap().get_difficulty());

                //info!("{} seconds elapsed", time);
                //let rate = 100000/time;
                //info!("mining rate {} block/s", rate);
                let longest_chain: Vec<H256> = self.blockchain.lock().unwrap().all_blocks_in_longest_chain();
                for blk_hash in longest_chain {
                    let ts = self.blockchain.lock().unwrap().find_one_block(&blk_hash).unwrap().header.timestamp;
                    println!("{}",ts)
                }

                break;
            }
        }
    }
}
