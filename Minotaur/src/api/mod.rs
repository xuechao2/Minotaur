use serde::Serialize;
use crate::blockchain::Blockchain;
use crate::crypto::hash::H256;
use crate::miner::Handle as MinerHandle;
use crate::staker::Handle as StakerHandle;
use crate::spv::Handle as SPVHandle;
use crate::transaction::SignedTransaction;
use crate::transaction::SpamId;
//use crate::fly::Handle as FlyHandle;
use crate::txgenerator::Handle as TxgeneratorHandle;
use crate::network::server::Handle as NetworkServerHandle;
use crate::network::message::Message;

use log::info;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use tiny_http::Header;
use tiny_http::Response;
use tiny_http::Server as HTTPServer;
use url::Url;

pub struct Server {
    handle: HTTPServer,
    miner: MinerHandle,
    staker: StakerHandle,
    txgenerator: TxgeneratorHandle,
    network: NetworkServerHandle,
    spv: SPVHandle,
    //fly: FlyHandle
    blockchain: Arc<Mutex<Blockchain>>,
}

#[derive(Serialize)]
struct ApiResponse {
    success: bool,
    message: String,
}

macro_rules! respond_result {
    ( $req:expr, $success:expr, $message:expr ) => {{
        let content_type = "Content-Type: application/json".parse::<Header>().unwrap();
        let payload = ApiResponse {
            success: $success,
            message: $message.to_string(),
        };
        let resp = Response::from_string(serde_json::to_string_pretty(&payload).unwrap())
            .with_header(content_type);
        $req.respond(resp).unwrap();
    }};
}

macro_rules! respond_json {
    ( $req:expr, $message:expr ) => {{
        let content_type = "Content-Type: application/json".parse::<Header>().unwrap();
        let resp = Response::from_string(serde_json::to_string(&$message).unwrap())
            .with_header(content_type);
        $req.respond(resp).unwrap();
    }};
}

impl Server {
    pub fn start(
        addr: std::net::SocketAddr,
        miner: &MinerHandle,
        staker: &StakerHandle,
        txgenerator: &TxgeneratorHandle,
        network: &NetworkServerHandle,
        spv: &SPVHandle,
        blockchain: &Arc<Mutex<Blockchain>>,
        //fly: &FlyHandle,
    ) {
        let handle = HTTPServer::http(&addr).unwrap();
        let server = Self {
            handle,
            miner: miner.clone(),
            staker: staker.clone(),
            txgenerator: txgenerator.clone(),
            network: network.clone(),
            spv: spv.clone(),
            //fly: fly.clone(),
            blockchain: Arc::clone(blockchain),
        };
        thread::spawn(move || {
            for req in server.handle.incoming_requests() {
                let miner = server.miner.clone();
                let staker = server.staker.clone();
                let txgenerator = server.txgenerator.clone();
                let network = server.network.clone();
                let spv = server.spv.clone();
                let blockchain = Arc::clone(&server.blockchain);
                //let fly = server.fly.clone();
                thread::spawn(move || {
                    // a valid url requires a base
                    let base_url = Url::parse(&format!("http://{}/", &addr)).unwrap();
                    let url = match base_url.join(req.url()) {
                        Ok(u) => u,
                        Err(e) => {
                            respond_result!(req, false, format!("error parsing url: {}", e));
                            return;
                        }
                    };
                    match url.path() {
                        "/miner/start" => {
                            let params = url.query_pairs();
                            let params: HashMap<_, _> = params.into_owned().collect();
                            let lambda = match params.get("lambda") {
                                Some(v) => v,
                                None => {
                                    respond_result!(req, false, "missing lambda");
                                    return;
                                }
                            };
                            let lambda = match lambda.parse::<u64>() {
                                Ok(v) => v,
                                Err(e) => {
                                    respond_result!(
                                        req,
                                        false,
                                        format!("error parsing lambda: {}", e)
                                    );
                                    return;
                                }
                            };
                            miner.start(lambda);
                            respond_result!(req, true, "ok");
                        }
                        "/staker/start" => {
                            let params = url.query_pairs();
                            let params: HashMap<_, _> = params.into_owned().collect();
                            let zeta = match params.get("zeta") {
                                Some(v) => v,
                                None => {
                                    respond_result!(req, false, "missing zeta");
                                    return;
                                }
                            };
                            let zeta = match zeta.parse::<u64>() {
                                Ok(v) => v,
                                Err(e) => {
                                    respond_result!(
                                        req,
                                        false,
                                        format!("error parsing zeta: {}", e)
                                    );
                                    return;
                                }
                            };
                            staker.start(zeta);
                            respond_result!(req, true, "ok");
                        }
                        "/spv/start" => {
                            let params = url.query_pairs();
                            let params: HashMap<_, _> = params.into_owned().collect();
                            let lambda = match params.get("lambda") {
                                Some(v) => v,
                                None => {
                                    respond_result!(req, false, "missing lambda");
                                    return;
                                }
                            };
                            let lambda = match lambda.parse::<u64>() {
                                Ok(v) => v,
                                Err(e) => {
                                    respond_result!(
                                        req,
                                        false,
                                        format!("error parsing lambda: {}", e)
                                    );
                                    return;
                                }
                            };
                            spv.start(lambda);
                            respond_result!(req, true, "ok");
                        }
                        // "/fly/start" => {
                        //     let params = url.query_pairs();
                        //     let params: HashMap<_, _> = params.into_owned().collect();
                        //     let lambda = match params.get("lambda") {
                        //         Some(v) => v,
                        //         None => {
                        //             respond_result!(req, false, "missing lambda");
                        //             return;
                        //         }
                        //     };
                        //     let lambda = match lambda.parse::<u64>() {
                        //         Ok(v) => v,
                        //         Err(e) => {
                        //             respond_result!(
                        //                 req,
                        //                 false,
                        //                 format!("error parsing lambda: {}", e)
                        //             );
                        //             return;
                        //         }
                        //     };
                        //     fly.start(lambda);
                        //     respond_result!(req, true, "ok");
                        // }
                        "/tx-generator/start" => {
                            let params = url.query_pairs();
                            let params: HashMap<_, _> = params.into_owned().collect();
                            let theta = match params.get("theta") {
                                Some(v) => v,
                                None => {
                                    respond_result!(req, false, "missing theta");
                                    return;
                                }
                            };
                            let theta = match theta.parse::<u64>() {
                                Ok(v) => v,
                                Err(e) => {
                                    respond_result!(
                                        req,
                                        false,
                                        format!("error parsing theta: {}", e)
                                    );
                                    return;
                                }
                            };
                            txgenerator.start(theta);
                            respond_result!(req, true, "ok");
                        }
                        "/ledger/txn" => {
                            let blockchain = blockchain.lock().unwrap();
                            let pos_blocks = blockchain.get_longest_chain();
                            let pow_blocks: Vec<H256> = pos_blocks.into_iter().map(|b|b.content.transaction_ref).flatten().collect();
                            let txns: Vec<Vec<SignedTransaction>> = pow_blocks.into_iter().map(|h|blockchain.find_one_block(&h).unwrap().content.data).collect();
                            let ids: Vec<Vec<SpamId>> = txns.into_iter().map(|x|x.into_iter().map(|t|(&t).into()).collect()).collect();
                            // let txns: Vec<Vec<SignedTransaction>> = blocks.into_iter().map(|b|b.content.data).collect();
                            respond_json!(req, ids);
                        }
                        "/ledger/spam" => {
                            let blockchain = blockchain.lock().unwrap();
                            let pos_blocks = blockchain.get_longest_chain();
                            let pow_blocks: Vec<H256> = pos_blocks.into_iter().map(|b|b.content.transaction_ref).flatten().collect();
                            let txns: Vec<Vec<SignedTransaction>> = pow_blocks.into_iter().map(|h|blockchain.find_one_block(&h).unwrap().content.data).collect();
                            let ids: Vec<Vec<SpamId>> = txns.into_iter().map(|x|x.into_iter().map(|t|(&t).into()).collect()).collect();
                            let total_num: usize = ids.iter().map(|v|v.len()).sum();
                            let unique_set: HashSet<SpamId> = ids.into_iter().flatten().collect();
                            let unique_num: usize = unique_set.len();
                            #[derive(Serialize)]
                            struct SpamReport {
                                total_txn_num: usize,
                                unique_txn_num: usize,
                                meaningful_ratio: f32,
                                spam_ratio: f32,
                            }
                            respond_json!(req, SpamReport {
                                total_txn_num: total_num,
                                unique_txn_num: unique_num,
                                meaningful_ratio: (unique_num as f32)/(total_num as f32),
                                spam_ratio: 1f32-(unique_num as f32)/(total_num as f32),
                            });
                        }
                        "/network/ping" => {
                            network.broadcast(Message::Ping(String::from("Test ping")));
                            respond_result!(req, true, "ok");
                        }
                        _ => {
                            let content_type =
                                "Content-Type: application/json".parse::<Header>().unwrap();
                            let payload = ApiResponse {
                                success: false,
                                message: "endpoint not found".to_string(),
                            };
                            let resp = Response::from_string(
                                serde_json::to_string_pretty(&payload).unwrap(),
                            )
                            .with_header(content_type)
                            .with_status_code(404);
                            req.respond(resp).unwrap();
                        }
                    }
                });
            }
        });
        info!("API server listening at {}", &addr);
    }
}
