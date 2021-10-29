use crate::block::generate_genesis_block;
use crate::block::{Block,Header};
use crate::crypto::hash::{H256,Hashable,hash_divide_by};
use std::collections::{HashMap};
use serde::{Serialize, Deserialize};
//use crate::block::generate_random_block;
use log::{debug, warn,info};
use tari_mmr::{MerkleMountainRange, MerkleProof, Hash};
use sha2::{Digest, Sha256};

#[derive(Serialize, Deserialize,Hash, Eq, PartialEq, Debug,Clone)]
pub struct Data {
    blk: Block,
    height: u128,
}

pub struct Blockchain { 
	chain: HashMap<H256,Data>,
	map: HashMap<H256,MerkleMountainRange<Sha256, Vec<Hash>>>,
    tip: H256,
    depth: u128,
    num_pos: u128,
    num_pow: u128,
    epoch_size: u128,
    epoch_time: u128,
    genesis_time: u128,
}

impl Blockchain {
    /// Create a new blockchain, only containing the genesis block
    pub fn new(initial_time:u128) -> Self {
        //unimplemented!()
		let genesis = generate_genesis_block(initial_time);
		info!("Timestamp of the genesis block: {}", genesis.header.timestamp);
		let blockinfo = Data{blk:genesis.clone(),height:0}; 
		let hash: H256 = genesis.clone().hash();
		let mut chain = HashMap::new();
		chain.insert(hash,blockinfo);
		let mut map = HashMap::new();
		map.insert(hash, MerkleMountainRange::<Sha256, Vec<Hash>>::new(Vec::new()));
		let tip:H256 = hash;
		//info!("0:{}",tip);
		Blockchain{chain, map, tip, depth:0, num_pos:0, num_pow:0, epoch_size:500, epoch_time: 120_000_000,genesis_time: initial_time}
	
    }

    /// Insert a PoS block into blockchain
    pub fn insert_pos(&mut self, block: &Block) -> bool {
		//unimplemented!()
		if self.chain.contains_key(&block.hash()) {
			return false;
		}
		let header:Header = block.header.clone();
		let parenthash: H256 = header.parent;
		let parentdata: Data;
		match self.chain.get(&parenthash) {
			Some(data) => parentdata = data.clone(),
			None => return false,
		}
		let parentheight = parentdata.height;
		let newheight = parentheight+1;
		let newdata = Data{blk:block.clone(),height:newheight};
		let newhash = block.hash();
		let mut new_mmr = self.get_mmr(&parenthash);
		mmr_push_leaf(&mut new_mmr, newhash.as_ref().to_vec().clone());
		self.chain.insert(newhash,newdata);
		self.map.insert(newhash, new_mmr);
		self.num_pos = self.num_pos + 1;
		if newheight > self.depth {
			self.depth = newheight;
			self.tip = newhash;
			return true;
		} 
		return false;
    }

    /// Insert a PoW block into blockchain
    pub fn insert_pow(&mut self, block: &Block) -> bool {
		//unimplemented!()
		if self.chain.contains_key(&block.hash()) {
			return false;
		}
		let header:Header = block.header.clone();
		let parenthash: H256 = header.parent;
		let parentdata: Data;
		match self.chain.get(&parenthash) {
			Some(data) => parentdata = data.clone(),
			None => return false,
		}
		let parentheight = parentdata.height;
		let newheight = parentheight+1;
		let newdata = Data{blk:block.clone(),height:newheight};
		let newhash = block.hash();
		let mut new_mmr = self.get_mmr(&parenthash);
		mmr_push_leaf(&mut new_mmr, newhash.as_ref().to_vec().clone());
		self.chain.insert(newhash,newdata);
		self.map.insert(newhash, new_mmr);
		self.num_pow = self.num_pow + 1;

		return true;
    }

    /// Get the last block's hash of the longest chain
    pub fn tip(&self) -> H256 {
        //unimplemented!()
		self.tip
	}
	
	pub fn get_pow_difficulty(&self, current_ts:u128) -> H256 {
			let epoch_size = self.epoch_size;
			let depth = self.depth;
			let epoch_time = self.epoch_time;
			let tip = self.tip;
			let tip_time = self.chain.get(&tip).unwrap().blk.header.timestamp;
			let genesis_time = self.genesis_time;
			let tip_epoch = (tip_time - genesis_time)/epoch_time;
			let curent_epoch = (current_ts - genesis_time)/epoch_time;
			if curent_epoch > tip_epoch && depth > 1 {
				let old_diff: H256 = self.chain.get(&self.tip).unwrap().blk.header.pow_difficulty;
				//let end_time: u128 = self.chain.get(&tip).unwrap().blk.header.timestamp;
				let mut hash = tip.clone();
				let mut all_hashs = Vec::new(); 
				while true {
					let blk = self.chain.get(&hash).unwrap().blk.clone();
					let pow_blks = blk.content.transaction_ref.clone();
					for pow_blk in pow_blks {
						if !all_hashs.contains(&pow_blk) {
                            all_hashs.push(pow_blk);
                        }
					}
					hash = self.chain.get(&hash).unwrap().blk.header.parent;
					let blk_time = self.chain.get(&hash).unwrap().blk.header.timestamp;
					let blk_epoch = (blk_time - genesis_time)/epoch_time;
					if blk_epoch < tip_epoch || blk_time == self.genesis_time {
						break;
					}
				}
				let num_blk = all_hashs.len();
				//let start_time: u128 = self.chain.get(&hash).unwrap().blk.header.timestamp;
				let mut ratio = (num_blk as f64)/(epoch_size as f64);
				//println!("Ratio: {}", ratio);
				// if ratio > 4.0 {
				// 	ratio = 4.0;
				// } else if ratio < 0.25 {
				// 	ratio = 0.25;
				// }
				let new_diff:H256 = hash_divide_by(&old_diff,ratio);
				//println!("Mining difficulty changes from {} to {}",old_diff, new_diff);
				new_diff
			} else {
				self.chain.get(&self.tip).unwrap().blk.header.pow_difficulty
			}
	}

	pub fn epoch(&self, current_ts:u128) -> u128 {
			let epoch_size = self.epoch_size;
			//let depth = self.depth;
			let epoch_time = self.epoch_time;
			//let tip = self.tip;
			//let tip_time = self.chain.get(&tip).unwrap().blk.header.timestamp;
			let genesis_time = self.genesis_time;
			//let tip_epoch = (tip_time - genesis_time)/epoch_time;
			let current_epoch = (current_ts - genesis_time)/epoch_time;
			current_epoch
		}

	pub fn get_pos_difficulty(&self) -> H256 {
		self.chain.get(&self.tip).unwrap().blk.header.pos_difficulty
	}
	
	pub fn get_depth(&self) -> u128 {
		self.depth
	}

	pub fn get_num_pos(&self) -> u128 {
		self.num_pos
	}

	pub fn get_num_pow(&self) -> u128 {
		self.num_pow
	}

	pub fn get_size(&self) -> usize {
		self.chain.len()
	}

	pub fn get_mmr(&self, hash: &H256) -> MerkleMountainRange<Sha256, Vec<Hash>> {
		let mmr_ref = self.map.get(hash).unwrap();
		let leaf_hashes = mmr_ref.get_leaf_hashes(0, mmr_ref.get_leaf_count().unwrap()+1).unwrap().clone();
		let mut mmr_ret = MerkleMountainRange::<Sha256, Vec<Hash>>::new(Vec::new());
		mmr_ret.assign(leaf_hashes).unwrap();
		mmr_ret
	}
	
	pub fn contains_hash(&self, hash: &H256) -> bool {
		self.chain.contains_key(hash)
	}
	
	pub fn print_longest_chain(&self) {
		let mut longest_chain = self.all_blocks_in_longest_chain();
		info!("************* Print Longest Chain *************");
		info!("{:?}", longest_chain);
		info!("***********************************************");
	}

    /// Get the last block's hash of the longest chain
    //#[cfg(any(test, test_utilities))]
    pub fn all_blocks_in_longest_chain(&self) -> Vec<H256> {
		//unimplemented!()
		let mut all_block : Vec<H256> = vec![];
		let mut current_hash = self.tip;
		//let mut parent_hash;
		let mut parentdata: Data;

		loop {
			match self.chain.get(&current_hash) {
				None => break,
				Some(data) => parentdata = data.clone(),
			}
			all_block.push(current_hash);
			current_hash = parentdata.blk.header.parent;
			debug!("current_hash {:?}!", current_hash);
			// debug!("contains {:?}!", self.chain.get(&current_hash));
			
		}
		debug!("finish {:?}!", all_block);

		all_block.reverse();
		all_block
	}
	
	pub fn get_longest_chain(&self) -> Vec<Block> {
		//unimplemented!()
		let mut all_block : Vec<H256> = vec![];
		let mut current_hash = self.tip;
		//let mut parent_hash;
		let mut parentdata: Data;

		loop {
			match self.chain.get(&current_hash) {
				None => break,
				Some(data) => parentdata = data.clone(),
			}
			all_block.push(current_hash);
			current_hash = parentdata.blk.header.parent;
			debug!("current_hash {:?}!", current_hash);
			// debug!("contains {:?}!", self.chain.get(&current_hash));
			
		}
		all_block.reverse();
		debug!("finish {:?}!", all_block);

		let mut chain: Vec<Block> = vec![];
		for hash in all_block {
			chain.push(self.find_one_block(&hash).unwrap().clone());
		}
		chain
    }

    pub fn find_one_block(&self,hash: &H256) -> Option<Block> {
    	let one_block : Block;
    	match self.chain.get(&hash) {
			None => return None,
			Some(data) => return Some(data.clone().blk),
		}
    }
}

pub fn mmr_push_leaf(mmr: &mut MerkleMountainRange<Sha256, Vec<Hash>>, leaf_hash: Hash) {
	let mut leaf_hashes = mmr.get_leaf_hashes(0, mmr.get_leaf_count().unwrap()+1).unwrap().clone();
	leaf_hashes.push(leaf_hash);
	mmr.assign(leaf_hashes).unwrap();
}

// FlyClientProposal is a proposal sent from the prover, 
// it contains current chain depth and last block header.
#[derive(Serialize, Deserialize, Eq, PartialEq, Debug,Clone)]
pub struct FlyClientProposal { 
	pub chain_depth: usize,
	pub header: Header,
}

impl FlyClientProposal {
	pub fn new(blockchain: &Blockchain) -> Self {
		FlyClientProposal{
			chain_depth: blockchain.depth as usize,
			header: blockchain.find_one_block(&blockchain.tip()).unwrap().header,
		}
	}
}

// FlyClientQuery is the query sent from verifier to prover,
// it contains the chain depth of a proposal and a sample of
// blocks for proof. Note sample points are < query_depth - 1.
#[derive(Serialize, Deserialize, Eq, PartialEq, Debug,Clone)]
pub struct FlyClientQuery { 
	pub query_depth: usize,
	pub sample: Vec<usize>,
}

impl FlyClientQuery {
	pub fn new(proposal_depth: usize, sample: Vec<usize>) -> Self {
		FlyClientQuery{
			query_depth: proposal_depth,
			sample,
		}
	}
}

// The proof for a single point provided by the prover. To handle
// all the sample of a query, need a Vec<FlyClientProof>.
#[derive(Serialize, Deserialize, Eq, PartialEq, Debug,Clone)]
pub struct FlyClientProof { 
	// leaf_idx is corresponding to a number in the query sample
	leaf_idx: usize,
	// block header corresponding to the leaf_idx
	pub leaf_header: Header,
	// merkle proof for this block
	merkle_proof: MerkleProof,
}

impl FlyClientProof {
	// query depth is from the FlyClientQuery
	pub fn new(blockchain: &Blockchain, leaf_idx: usize, query_depth: usize) -> Self {
		// Note get_longest_chain() include genesis block with is not included in depth.
		let leaf_hash: H256 = blockchain.get_longest_chain()[leaf_idx + 1].hash();
		let leaf_header = blockchain.find_one_block(&leaf_hash).unwrap().header; 
		let mmr_hash = blockchain.get_longest_chain()[query_depth-2 + 1].hash();
		let mmr = blockchain.get_mmr(&mmr_hash);
		let merkle_proof = MerkleProof::for_leaf_node(&mmr, leaf_idx).unwrap();
		FlyClientProof{leaf_idx, leaf_header, merkle_proof,}
	}

	// only deals with first two step verification in the paper.
	pub fn verify(self, mmr_root: Hash) -> bool {
		assert!(self.merkle_proof.verify_leaf::<Sha256>(&mmr_root[..], 
				self.leaf_header.hash().as_ref(), self.leaf_idx).is_ok());
		true
	}
}



// #[cfg(any(test, test_utilities))]
// mod tests {
//     use super::*;
//     use crate::block::test::generate_random_block;
//     use crate::crypto::hash::Hashable;

//     #[test]
//     fn blockchain_mmr_test() {
//         let mut blockchain = Blockchain::new();
// 		let genesis_hash = blockchain.tip();
// 		let genesis_mmr = blockchain.get_mmr(&genesis_hash);
//         let block = generate_random_block(&genesis_hash, &genesis_mmr);
//         blockchain.insert(&block);
// 		assert_eq!(blockchain.tip(), block.hash());
// 		let tip_mmr = blockchain.get_mmr(&blockchain.tip);
// 		println!("{} {}", tip_mmr.get_leaf_count().unwrap(), tip_mmr.len().unwrap());
// 		assert!(MerkleProof::for_leaf_node(&tip_mmr, 0).is_ok());


// 		let block_hash = blockchain.tip();
// 		let block_mmr = blockchain.get_mmr(&block_hash);
//         let block1 = generate_random_block(&block_hash, &block_mmr);
//         blockchain.insert(&block1);
// 		assert_eq!(blockchain.tip(), block1.hash());
// 		let tip_mmr = blockchain.get_mmr(&blockchain.tip);
// 		println!("{} {}", tip_mmr.get_leaf_count().unwrap(), tip_mmr.len().unwrap());
// 		assert!(MerkleProof::for_leaf_node(&tip_mmr, 1).is_ok());


// 		let block1_hash = blockchain.tip();
// 		let block1_mmr = blockchain.get_mmr(&block1_hash);
//         let block2 = generate_random_block(&block1_hash, &block1_mmr);
//         blockchain.insert(&block2);
// 		assert_eq!(blockchain.tip(), block2.hash());
// 		let tip_mmr = blockchain.get_mmr(&blockchain.tip);
// 		println!("{} {}", tip_mmr.get_leaf_count().unwrap(), tip_mmr.len().unwrap());
// 		assert!(MerkleProof::for_leaf_node(&tip_mmr, 2).is_ok());
		
		
// 		let block2_hash = blockchain.tip();
// 		let block2_mmr = blockchain.get_mmr(&block2_hash);
//         let block3 = generate_random_block(&block2_hash, &block2_mmr);
//         blockchain.insert(&block3);
// 		assert_eq!(blockchain.tip(), block3.hash());
// 		let tip_mmr = blockchain.get_mmr(&blockchain.tip);
// 		println!("{} {}", tip_mmr.get_leaf_count().unwrap(), tip_mmr.len().unwrap());
// 		assert!(MerkleProof::for_leaf_node(&tip_mmr, 3).is_ok());


// 		let proposal: FlyClientProposal = FlyClientProposal::new(&blockchain);
// 		let query: FlyClientQuery = FlyClientQuery::new(proposal.chain_depth, vec![0]);
// 		let proof: FlyClientProof = FlyClientProof::new(&blockchain, 0, query.query_depth);
// 		assert!(proof.verify(proposal.header.mmr_root));
//     }
// }
