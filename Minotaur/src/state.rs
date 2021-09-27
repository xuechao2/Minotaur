use ring::signature::{Ed25519KeyPair, Signature, KeyPair, VerificationAlgorithm, EdDSAParameters};
use crate::block::Block;
use crate::crypto::hash::{H256,H160,Hashable};
use std::collections::HashMap;
use crate::transaction::{SignedTransaction,verify_signedtxn};
use log::{info, debug, warn};
use rand::{Rng, SeedableRng, StdRng};
use std::io::BufReader;
use std::io::BufRead;
use std::io;
use std::fs;


type state = HashMap<H160, (usize, usize)>;

pub struct State {
    state_per_block: HashMap<H256, state>,
}

pub fn file_to_vec(filename: String) -> io::Result<Vec<String>> {
    let file_in = fs::File::open(filename)?;
    let file_reader = BufReader::new(file_in);
    Ok(file_reader.lines().filter_map(io::Result::ok).collect())
}

pub fn create_ico_keys(n: usize) -> Vec<Ed25519KeyPair> {
    let lines: Vec<String> = file_to_vec("pubkeys.txt".to_string()).unwrap();

    let mut keys: Vec<Ed25519KeyPair> = Vec::new();
    for i in 0..n {
        let pkcs8_bytes = hex::decode(lines[i].clone()).unwrap();
        let key = Ed25519KeyPair::from_pkcs8((&pkcs8_bytes[..]).into()).unwrap();
        keys.push(key);
    }
    keys
}

pub fn create_ico_accounts(keys: Vec<Ed25519KeyPair>) -> Vec<H160> {
    let mut accounts: Vec<H160> = Vec::new();
    for key in keys {
        let account: H160 = compute_key_hash(key.public_key().as_ref().to_vec()).into();
        accounts.push(account);
    }
    accounts
}

pub fn compute_key_hash(key: Vec<u8>) -> H256 {
    let bytes: &[u8] = &key;
    ring::digest::digest(&ring::digest::SHA256, bytes).into()
}

pub fn transaction_check(current_state: &mut state, tx: &SignedTransaction) -> bool {
	if verify_signedtxn(&tx) {
		let copy = tx.clone();
        let pubk = copy.sign.pubk.clone();
        let nonce = copy.transaction.nonce.clone();
        let value = copy.transaction.value.clone();
        let recv = copy.transaction.recv.clone();

        let sender: H160 = compute_key_hash(pubk).into();
        let (s_nonce, s_amount) = current_state.get(&sender).unwrap().clone();
        let (r_nonce, r_amount) = current_state.get(&recv).unwrap().clone();

        if nonce == s_nonce+1 &&  s_amount >= value{
            current_state.insert(sender, (s_nonce+1, s_amount-value));
            current_state.insert(recv, (r_nonce, r_amount+value));
            return true;
        } else {
            return false;
        }
    } else {
    	return false;
    }

}

impl State {
    pub fn new() -> Self {
        let state_per_block = HashMap::new();
        State{state_per_block}
    }

    pub fn ico(&mut self, genesis_hash: H256, accounts: &Vec<H160>, amount: usize) {
        if self.state_per_block.len()>0 {
            info!("Already did an ICO!");
            return;
        }
        let mut s = state::new();
        for account in accounts {
            s.insert(*account, (0, amount));
        }
        self.state_per_block.insert(genesis_hash, s);
    }

    pub fn update_block(&mut self, block: &Block) {
        if self.state_per_block.contains_key(&block.hash()) {
            return;
        }
        let parent_hash = block.header.parent;
        if !self.state_per_block.contains_key(&parent_hash) {
            info!("Updating a block before its parent!");
            return;
        }
        let mut parent_state = self.state_per_block.get(&parent_hash).unwrap().clone();
        let txns = block.content.data.clone();
        for txn in txns {
            let sender: H160 = compute_key_hash(txn.sign.pubk).into();
            let recv = txn.transaction.recv;
            // todo: add txn check, if the account exists or amount is enough
            
            let (s_nonce, s_amount) = parent_state.get(&sender).unwrap().clone();
            let (r_nonce, r_amount) = parent_state.get(&recv).unwrap().clone();
            parent_state.insert(sender, (s_nonce+1, s_amount-txn.transaction.value));
            parent_state.insert(recv, (r_nonce, r_amount+txn.transaction.value));
        }
        self.state_per_block.insert(block.hash(), parent_state);
    }

    pub fn update_blocks(&mut self, blocks: &Vec<Block>) {
        for block in blocks {
            self.update_block(block);
        }
    }

    pub fn one_block_state(&mut self, hash: &H256) -> state {
    	let find_state = self.state_per_block.get(&hash).unwrap().clone();
    	find_state

    }

    pub fn print_last_block_state(&mut self, hash: &H256) {
        let last_state = self.state_per_block.get(&hash).unwrap().clone();
        for (key, value) in last_state {
            let (nonce, amount) = value;
            info!("account {:?} has nonce {} value {}", key, nonce, amount);
        }
    }
 }