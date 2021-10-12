use std::cmp::min;
use crate::transaction::generate_random_transaction;
//use crate::block::generate_block;
use crate::block::{Block, Header, Content};
use crate::crypto::merkle::MerkleTree;
use crate::crypto::hash::{H256,Hashable,generate_random_hash};
use crate::transaction::{Transaction,SignedTransaction,generate_valid_signed_transaction};
use crate::network::server::Handle as ServerHandle;
use crate::blockchain::Blockchain;
use crate::network::message::Message;
use std::collections::{HashMap, HashSet};
//use crate::state::{State,compute_key_hash,transaction_check};
use crate::crypto::hash::H160;
use ring::signature::Ed25519KeyPair;
use ring::signature::KeyPair;
use crate::state::{State,transaction_check,compute_key_hash,create_ico_keys};


use log::info;
use std::sync::{Arc, Mutex};

use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use std::time;
use std::time::{SystemTime, UNIX_EPOCH};
use std::thread;
use rand::Rng;
use std::io::BufReader;
use std::io::BufRead;
use std::io;
use std::fs;


enum ControlSignal {
    Start(u64), // the number controls the theta of interval between tx generation
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
    all_txns: Arc<Mutex<HashMap<H256,SignedTransaction>>>,
    state: Arc<Mutex<State>>,
    //key_pairs: Vec<Ed25519KeyPair>,
    accounts: Vec<H160>,
}

#[derive(Clone)]
pub struct Handle {
    /// Channel for sending signal to the txgenerator thread
    control_chan: Sender<ControlSignal>,
}

pub fn new(
    blockchain: &Arc<Mutex<Blockchain>>,
    server: &ServerHandle,
    mempool: &Arc<Mutex<Vec<SignedTransaction>>>,
    all_txns: &Arc<Mutex<HashMap<H256,SignedTransaction>>>,
    state: &Arc<Mutex<State>>,
    //key_pairs: &Vec<Ed25519KeyPair>,
    accounts: &Vec<H160>,
) -> (Context, Handle) {
    let (signal_chan_sender, signal_chan_receiver) = unbounded();

    let ctx = Context {
        blockchain: Arc::clone(blockchain),
        control_chan: signal_chan_receiver,
        operating_state: OperatingState::Paused,
        server: server.clone(),
        mempool: Arc::clone(mempool),
        all_txns: Arc::clone(all_txns),
        state: Arc::clone(state),
        //key_pairs: key_pairs.clone(),
        accounts: accounts.clone(),
    };

    let handle = Handle {
        control_chan: signal_chan_sender,
    };

    (ctx, handle)
}

impl Handle {
    pub fn exit(&self) {
        self.control_chan.send(ControlSignal::Exit).expect("txgenerator exit");
    }

    pub fn start(&self, theta: u64) {
        self.control_chan
            .send(ControlSignal::Start(theta))
            .expect("txgenerator start");
    }

}

impl Context {
    pub fn start(mut self) {
        thread::Builder::new()
            .name("txgenerator".to_string())
            .spawn(move || {
                self.generator_loop();
            })
            .expect("txgenerator 1");
        info!("Txgenerator initialized into paused mode");
    }

    fn handle_control_signal(&mut self, signal: ControlSignal) {
        match signal {
            ControlSignal::Exit => {
                info!("Txgenerator shutting down");
                self.operating_state = OperatingState::ShutDown;
            }
            ControlSignal::Start(i) => {
                info!("Txgenerator starting in continuous mode with theta {}", i);
                self.operating_state = OperatingState::Run(i);
            }
        }
    }



    fn generator_loop(&mut self) {
        let account_number = self.accounts.len();
        let start: time::SystemTime = SystemTime::now();
        let keypairs = create_ico_keys(account_number);
        // main mining loop
        loop {
            // check and react to control signals
            match self.operating_state {
                OperatingState::Paused => {
                    let signal = self.control_chan.recv().expect("txgenerator 2");
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
                    Err(TryRecvError::Disconnected) => panic!("Txgenerator control channel detached"),
                },
            }
            if let OperatingState::ShutDown = self.operating_state {
                return;
            }

            //let tx = generate_random_signed_transaction();
            //generate valid tx
            let mut rng = rand::thread_rng();
            let parent = self.blockchain.lock().expect("txgenerator error 1").tip();
            //info!("1:{}",parent);
            if self.state.lock().unwrap().check_block(&parent) {
                let current_state = self.state.lock().expect("txgenerator error 2").one_block_state(&parent).clone();
                //info!("2");
                let sender_index:usize = rng.gen_range(0,account_number);
                let pubk:&Ed25519KeyPair = &keypairs[sender_index];
                let sender:H160 = compute_key_hash(pubk.public_key().as_ref().to_vec()).into();
                let (s_nonce, s_amount) = current_state.get(&sender).expect("txgenerator current_state.get(&sender) failed").clone();
                if s_amount<1 {
                    continue;
                }
                
                let mut recv_index:usize = rng.gen_range(0,account_number);
                while recv_index==sender_index {
                    recv_index = rng.gen_range(0,account_number);
                }
                let recv = self.accounts[recv_index];
                
                let value:usize = rng.gen_range(1, s_amount+1);
                let tx = generate_valid_signed_transaction(recv, value, s_nonce+1, &pubk);

                self.mempool.lock().expect("txgenerator error 3").push(tx.clone());
                self.all_txns.lock().expect("txgenerator error 4").insert(tx.clone().hash(), tx.clone());
                self.server.broadcast(Message::NewTransactionHashes(vec![tx.clone().hash()]));
                // info!("new tx generated:{}",self.mempool.lock().unwrap().len());
            }


            if let OperatingState::Run(i) = self.operating_state {

                if i != 0 {
                    let interval = time::Duration::from_micros(i as u64);
                    thread::sleep(interval);
                }
            }
        }
    }
}