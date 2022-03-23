#[cfg(test)]
#[macro_use]
extern crate hex_literal;

pub mod api;
pub mod block;
pub mod blockchain;
pub mod crypto;
pub mod miner;
//pub mod staker;
pub mod spv;
//pub mod fly;
pub mod network;
pub mod transaction;
pub mod txgenerator;
pub mod state;
pub mod spam_recorder;

use crate::crypto::hash::Hashable;
use std::collections::{HashMap, HashSet};
use crate::blockchain::Blockchain;
use std::sync::{Arc, Mutex};
use clap::clap_app;
use crossbeam::channel;
use log::{error, info};
use api::Server as ApiServer;
use network::{server, worker, spv_worker, selfish_worker};
use std::net;
use std::process;
use std::thread;
use std::time;
use crate::crypto::hash::H160;
use ring::signature::Ed25519KeyPair;

use vrf::openssl::{CipherSuite, ECVRF};
use vrf::VRF;  

use crate::spam_recorder::SpamRecorder;

fn main() {
    // parse command line arguments
    let matches = clap_app!(Fruitchains =>
     (version: "0.1")
     (about: "Fruitchains client")
     (@arg verbose: -v ... "Increases the verbosity of logging")
     (@arg peer_addr: --p2p [ADDR] default_value("127.0.0.1:6000") "Sets the IP address and the port of the P2P server")
     (@arg api_addr: --api [ADDR] default_value("127.0.0.1:7000") "Sets the IP address and the port of the API server")
     (@arg known_peer: -c --connect ... [PEER] "Sets the peers to connect to at start")
     (@arg p2p_workers: --("p2p-workers") [INT] default_value("4") "Sets the number of worker threads for P2P server")
     (@arg spv_client: --spv [BOOL] default_value("false") "Whether spv client or full node") // false for full node, true for spv client
     //(@arg fly_client: --fly [BOOL] default_value("false") "Whether fly client or full node") // false for full node, true for fly client
     (@arg vrf_secret_key: --sk [String] "Secret key to be used to print or validate proof" )
     (@arg initial_time: --ts [u128] "Timestamp of the genesis block" )
     (@arg txn_numerator: --txnn [usize] default_value("1") "txn generator numerator, range: [0,denominator)" )
     (@arg txn_denominator: --txnd [usize] default_value("1") "txn generator denominator" )
     (@arg selfish_node: --selfish [BOOL] default_value("false") "Whether selfish or honest node") // false for honest node, true for selfish node
    )
    .get_matches();

    //let mut vrf = ECVRF::from_suite(CipherSuite::SECP256K1_SHA256_TAI).unwrap();
    // Inputs: Secret Key, Public Key (derived) & Message
    //let vrf_secret_key = hex::decode(&matches.value_of("vrf_secret_key").unwrap()).unwrap();
    //let vrf_public_key = vrf.derive_public_key(&vrf_secret_key).unwrap();

    let vrf_secret_key = Default::default();
    let vrf_public_key = Default::default();

    
    // let initial_time = matches
    //     .value_of("initial_time")
    //     .unwrap()
    //     .parse::<u128>()
    //     .unwrap_or_else(|e| {
    //         error!("Error parsing initial_time: {}", e);
    //         process::exit(1);
    //     });


    let spv_client = matches
        .value_of("spv_client")
        .unwrap()
        .parse::<bool>()
        .unwrap_or_else(|e| {
            error!("Error parsing SPV client: {}", e);
            process::exit(1);
        });

    let selfish_node = matches
        .value_of("selfish_node")
        .unwrap()
        .parse::<bool>()
        .unwrap_or_else(|e| {
            error!("Error parsing selfish node: {}", e);
            process::exit(1);
        });

    // let fly_client = matches
    //     .value_of("fly_client")
    //     .unwrap()
    //     .parse::<bool>()
    //     .unwrap_or_else(|e| {
    //         error!("Error parsing Fly client: {}", e);
    //         process::exit(1);
    //     });



    // init logger
    let verbosity = matches.occurrences_of("verbose") as usize;
    stderrlog::new().verbosity(verbosity).init().unwrap();

    // parse p2p server address
    let p2p_addr = matches
        .value_of("peer_addr")
        .unwrap()
        .parse::<net::SocketAddr>()
        .unwrap_or_else(|e| {
            error!("Error parsing P2P server address: {}", e);
            process::exit(1);
        });

    // parse api server address
    let api_addr = matches
        .value_of("api_addr")
        .unwrap()
        .parse::<net::SocketAddr>()
        .unwrap_or_else(|e| {
            error!("Error parsing API server address: {}", e);
            process::exit(1);
        });

    let txnn= matches
        .value_of("txn_numerator")
        .unwrap()
        .parse::<usize>()
        .unwrap_or_else(|e| {
            error!("Error parsing txn_numerator: {}", e);
            process::exit(1);
        });
    let txnd= matches
        .value_of("txn_denominator")
        .unwrap()
        .parse::<usize>()
        .unwrap_or_else(|e| {
            error!("Error parsing txn_denominator: {}", e);
            process::exit(1);
        });

    // create channels between server and worker
    let (msg_tx, msg_rx) = channel::unbounded();
    // create fruit update channels (used to update txn, not update parent)
    let (fruit_context_update_send, fruit_context_update_recv) = channel::unbounded();
    // create block update channels
    let (block_context_update_send, block_context_update_recv) = channel::unbounded();

    // start the p2p server
    let (server_ctx, server) = server::new(p2p_addr, msg_tx).unwrap();
    server_ctx.start().unwrap();

    // start the worker
    let p2p_workers = matches
        .value_of("p2p_workers")
        .unwrap()
        .parse::<usize>()
        .unwrap_or_else(|e| {
            error!("Error parsing P2P workers: {}", e);
            process::exit(1);
        });

    
    let mut blockchain = Blockchain::new();
    let mut buffer = HashMap::new();
    let mut all_blocks = HashMap::new();
    let mut delays = Vec::new();
    let mut mempool = Vec::new();
    let mut tranpool = Vec::new();
    let mut all_txns = HashMap::new();
    let spam_recorder = SpamRecorder::new();
    let mut state = state::State::new();
    let blockchain = Arc::new(std::sync::Mutex::new(blockchain));
    let buffer = Arc::new(std::sync::Mutex::new(buffer));
    let all_blocks = Arc::new(std::sync::Mutex::new(all_blocks));
    let delays = Arc::new(std::sync::Mutex::new(delays));
    let mempool = Arc::new(std::sync::Mutex::new(mempool));
    let tranpool = Arc::new(std::sync::Mutex::new(tranpool));
    let all_txns = Arc::new(std::sync::Mutex::new(all_txns));
    let spam_recorder= Arc::new(std::sync::Mutex::new(spam_recorder));

    // ico 
    let ico_account_number = 2;
    let keypairs = state::create_ico_keys(ico_account_number);
    // let accounts = state::create_ico_accounts(keypairs);
    // let amount = 10000;
    //let genesis_block = block::generate_genesis_block();
    //let genesis_block_hash = blockchain.lock().unwrap().tip();
    //state.ico(genesis_block_hash, &accounts, amount);
    //info!("***** State After ICO *****");
    ///state.print_last_block_state(&genesis_block_hash);
    info!("***************************");
    


    let state = Arc::new(std::sync::Mutex::new(state));

    //let current_state = state.lock().unwrap().one_block_state(&genesis_block_hash).clone();
    //info!("ico done:{}",genesis_block_hash);

    let longestchain = Arc::new(std::sync::Mutex::new(Vec::new()));

    let (spv_ctx, spv) = spv::new(
        &longestchain, 
        &server,
    );
    spv_ctx.start();

    // let (fly_ctx, fly) = fly::new(
    //     //&longestchain, 
    //     &server,
    // );
    // fly_ctx.start();

    if spv_client {
        let spv_worker_ctx = spv_worker::new(
            p2p_workers,
            msg_rx,
            &server,
            &longestchain,
        );
        spv_worker_ctx.start();
    // } else if fly_client {
    //     let fly_worker_ctx = fly_worker::new(
    //         p2p_workers,
    //         msg_rx,
    //         &server,
    //         //&longestchain,
    //     );
    //     fly_worker_ctx.start();
    } else if selfish_node {
        let selfish_worker_ctx = selfish_worker::new(
            p2p_workers,
            msg_rx,
            &server,
            &blockchain,
            &buffer,
            &all_blocks,
            &delays,
            &mempool,
            &all_txns,
            &spam_recorder,
            &state,
            &tranpool,
            block_context_update_send.clone(),
            fruit_context_update_send.clone(),
        );
        selfish_worker_ctx.start();
    } else {
        let worker_ctx = worker::new(
            p2p_workers,
            msg_rx,
            &server,
            &blockchain,
            &buffer,
            &all_blocks,
            &delays,
            &mempool,
            &all_txns,
            &spam_recorder,
            &state,
            &tranpool,
            block_context_update_send.clone(),
            fruit_context_update_send.clone(),
        );
        worker_ctx.start();
    }
    

    let (txgenerator_ctx, txgenerator) = txgenerator::new(
        &blockchain,
        &server,
        &mempool,
        &all_txns,
        &state,
        keypairs,
        //&accounts,
        txnn,
        txnd,
    );
    txgenerator_ctx.start();

    // start the miner
    let (miner_ctx, miner) = miner::new(
        &blockchain,
        fruit_context_update_recv,
        fruit_context_update_send,
        block_context_update_recv,
        block_context_update_send,
        &server,
        &mempool,
        &spam_recorder,
        &state,
        &all_blocks,
        &tranpool,
        &vrf_secret_key,
        &vrf_public_key,
        selfish_node,
    );
    miner_ctx.start();

    // start the staker
    // let (staker_ctx, staker) = staker::new(
    //     &blockchain,
    //     context_update_recv,
    //     context_update_send,
    //     &server,
    //     //&mempool,
    //     &state,
    //     &all_blocks,
    //     &tranpool,
    //     &vrf_secret_key,
    //     &vrf_public_key,
    //     selfish_node,

    // );
    // staker_ctx.start();

    // connect to known peers
    if let Some(known_peers) = matches.values_of("known_peer") {
        let known_peers: Vec<String> = known_peers.map(|x| x.to_owned()).collect();
        let server = server.clone();
        thread::spawn(move || {
            for peer in known_peers {
                loop {
                    let addr = match peer.parse::<net::SocketAddr>() {
                        Ok(x) => x,
                        Err(e) => {
                            error!("Error parsing peer address {}: {}", &peer, e);
                            break;
                        }
                    };
                    match server.connect(addr) {
                        Ok(_) => {
                            info!("Connected to outgoing peer {}", &addr);
                            break;
                        }
                        Err(e) => {
                            error!(
                                "Error connecting to peer {}, retrying in one second: {}",
                                addr, e
                            );
                            thread::sleep(time::Duration::from_millis(1000));
                            continue;
                        }
                    }
                }
            }
        });
    }


    // start the API server
    ApiServer::start(
        api_addr,
        &miner,
        //&staker,
        &txgenerator,
        &server,
        &spv,
        &blockchain,
        //&fly,
    );

    loop {
        std::thread::park();
    }

    
}
