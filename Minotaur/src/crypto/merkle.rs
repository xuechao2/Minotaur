use super::hash::{Hashable, H256};
use log::info;

/// A Merkle tree.
#[derive(Debug, Default)]
pub struct MerkleTree {
    nodes: Vec<H256>,
    size: usize,
    height: usize,
    leaf_size: usize,
}

pub fn proof_path(index: usize, size: usize) -> Vec<usize> {
    let mut ans: Vec<usize> = Vec::new();
    let mut pos = index;
    let mut leaf_size = size;
    while leaf_size>1 {
        if leaf_size%2!=0 { leaf_size+=1; }
        if pos%2==0 {
            ans.push(pos+1);
        } else {
            ans.push(pos-1);
        }
        pos /= 2;
        leaf_size /= 2;
    }
    return ans;
}

impl MerkleTree {
    pub fn print(&self) {
        for i in 0..self.size {
            println!("{:?}", self.nodes[i]);
        }
    }

    pub fn new<T>(data: &[T]) -> Self where T: Hashable, {
        // unimplemented!()
        let mut length = data.len();
        let mut nodes = Vec::new();
        let mut last_level = Vec::new();
        for i in data {
            let h: H256 = i.hash();
            last_level.push(h);
            nodes.push(h);
        }
        let mut height = 1;
        while length>1 {
            if length%2!=0 { 
                last_level.push(data[length-1].hash()); 
                nodes.push(data[length-1].hash()); 
                length+=1;
            }
            let mut temp = Vec::new();
            for i in 0..length/2 {
                let h: H256 = add_hash(&last_level[2*i],&last_level[2*i+1]);
                temp.push(h);
                nodes.push(h);
            }
            last_level = temp.clone();
            length /= 2;
            height += 1;
        };
        let size = nodes.len();
        MerkleTree {
            nodes: nodes,
            size: size,
            height: height,
            leaf_size: data.len(),
        }
    }

    pub fn root(&self) -> H256 {
        self.nodes[self.size-1]
    }

    /// Returns the Merkle Proof of data at index i
    pub fn proof(&self, index: usize) -> Vec<H256> {
        let mut proof: Vec<H256> = Vec::new();
        let mut offset: usize = 0;
        let mut leaf_size = self.leaf_size;

        let proof_index = proof_path(index, leaf_size);

        for i in 0..self.height-1 {
            proof.push(self.nodes[offset+proof_index[i]]);
            if leaf_size%2!=0 { leaf_size+=1; }
            offset += leaf_size;
            leaf_size /= 2;
        }
        proof
    }
}

pub fn add_hash(a: &H256, b:&H256) -> H256 {
    let c = [a.as_ref(), b.as_ref()].concat();
    let combined = ring::digest::digest(&ring::digest::SHA256, &c);
    <H256>::from(combined)
}

/// Verify that the datum hash with a vector of proofs will produce the Merkle root. Also need the
/// index of datum and `leaf_size`, the total number of leaves.
pub fn verify(root: &H256, datum: &H256, proof: &[H256], index: usize, leaf_size: usize) -> bool {
    let mut h: H256 = *datum;
    let proof_index = proof_path(index, leaf_size);
    for i in 0..proof.len() {
        if proof_index[i]%2==0 {
            h = add_hash(&proof[i], &h);
        } else {
            h = add_hash(&h, &proof[i]);
        }
    }
    *root == h
}

#[cfg(test)]
mod tests {
    use crate::crypto::hash::H256;
    use super::*;

    macro_rules! gen_merkle_tree_data {
        () => {{
            vec![
                (hex!("0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d")).into(),
                (hex!("0101010101010101010101010101010101010101010101010101010101010202")).into(),
            ]
        }};
    }

    macro_rules! gen_merkle_tree_data2 {
        () => {{
            vec![
                (hex!("0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d")).into(),
                (hex!("0101010101010101010101010101010101010101010101010101010101010202")).into(),
                (hex!("0101010101010101010101010101010101010101010101010101010101010202")).into(),
                (hex!("0101010101010101010101010101010101010101010101010101010101010202")).into(),
                (hex!("0101010101010101010101010101010101010101010101010101010101010202")).into(),
            ]
        }};
    }

    #[test]
    fn root() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        let merkle_tree = MerkleTree::new(&input_data);
        let root = merkle_tree.root();

        // println!("{:?}", merkle_tree.size);
        // merkle_tree.print();

        assert_eq!(
            root,
            (hex!("6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920")).into()
        );
        // "b69566be6e1720872f73651d1851a0eae0060a132cf0f64a0ffaea248de6cba0" is the hash of
        // "0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d"
        // "965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f" is the hash of
        // "0101010101010101010101010101010101010101010101010101010101010202"
        // "6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920" is the hash of
        // the concatenation of these two hashes "b69..." and "965..."
        // notice that the order of these two matters
    }

    #[test]
    fn proof() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        let merkle_tree = MerkleTree::new(&input_data);
        let proof = merkle_tree.proof(0);
        assert_eq!(proof,
                   vec![hex!("965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f").into()]
        );
        // "965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f" is the hash of
        // "0101010101010101010101010101010101010101010101010101010101010202"
    }

    #[test]
    fn show_proof() {
        let input_data: Vec<H256> = gen_merkle_tree_data2!();
        let merkle_tree = MerkleTree::new(&input_data);
        // merkle_tree.print();
        let index = 3;
        let proof = merkle_tree.proof(index);
        info!("{:?}", proof);
        assert!(verify(&merkle_tree.root(), &input_data[index].hash(), &proof, index, input_data.len()));
        // "965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f" is the hash of
        // "0101010101010101010101010101010101010101010101010101010101010202"
    }

    #[test]
    fn verifying() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        let merkle_tree = MerkleTree::new(&input_data);
        let proof = merkle_tree.proof(0);
        assert!(verify(&merkle_tree.root(), &input_data[0].hash(), &proof, 0, input_data.len()));
    }
}
