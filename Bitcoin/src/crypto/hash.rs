use serde::{Serialize, Deserialize};
use std::convert::TryInto;
use rand::Rng;

/// An object that can be meaningfully hashed.
pub trait Hashable {
    /// Hash the object using SHA256.
    fn hash(&self) -> H256;
}

/// A SHA256 hash.
#[derive(Eq, PartialEq, Serialize, Deserialize, Clone, Hash, Default, Copy)]
pub struct H256([u8; 32]); // big endian u256

#[derive(Eq, PartialEq, Serialize, Deserialize, Clone, Hash, Default, Copy)]
pub struct H160([u8; 20]); // big endian u256

impl std::convert::From<&[u8; 20]> for H160 {
    fn from(input: &[u8; 20]) -> H160 {
        let mut buffer: [u8; 20] = [0; 20];
        buffer[..].copy_from_slice(input);
        H160(buffer)
    }
}

impl std::convert::From<[u8; 20]> for H160 {
    fn from(input: [u8; 20]) -> H160 {
        H160(input)
    }
}

impl std::convert::From<&H256> for H160 {
    fn from(input: &H256) -> H160 {
        let mut buffer: [u8; 20] = [0; 20];
        buffer[..].copy_from_slice(&input.0[0..20]);
        buffer.into()
    }
}

impl std::convert::From<H256> for H160 {
    fn from(input: H256) -> H160 {
        let mut buffer: [u8; 20] = [0; 20];
        buffer[..].copy_from_slice(&input.0[0..20]);
        buffer.into()
    }
}

impl std::fmt::Debug for H160 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{:>02x}{:>02x}..{:>02x}{:>02x}",
            &self.0[0], &self.0[1], &self.0[18], &self.0[19]
        )
    }
}



impl Hashable for H256 {
    fn hash(&self) -> H256 {
        ring::digest::digest(&ring::digest::SHA256, &self.0).into()
    }
}

impl std::fmt::Display for H256 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let start = if let Some(precision) = f.precision() {
            if precision >= 64 {
                0
            } else {
                32 - precision / 2
            }
        } else {
            0
        };
        for byte_idx in start..32 {
            write!(f, "{:>02x}", &self.0[byte_idx])?;
        }
        Ok(())
    }
}

impl std::fmt::Debug for H256 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{:>02x}{:>02x}..{:>02x}{:>02x}",
            &self.0[0], &self.0[1], &self.0[30], &self.0[31]
        )
    }
}

impl std::convert::AsRef<[u8]> for H256 {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl std::convert::From<&[u8; 32]> for H256 {
    fn from(input: &[u8; 32]) -> H256 {
        let mut buffer: [u8; 32] = [0; 32];
        buffer[..].copy_from_slice(input);
        H256(buffer)
    }
}

impl std::convert::From<&H256> for [u8; 32] {
    fn from(input: &H256) -> [u8; 32] {
        let mut buffer: [u8; 32] = [0; 32];
        buffer[..].copy_from_slice(&input.0);
        buffer
    }
}

impl std::convert::From<[u8; 32]> for H256 {
    fn from(input: [u8; 32]) -> H256 {
        H256(input)
    }
}

impl std::convert::From<H256> for [u8; 32] {
    fn from(input: H256) -> [u8; 32] {
        input.0
    }
}

impl std::convert::From<Vec<u8>> for H256 {
    fn from(input: Vec<u8>) -> H256 {
        let mut raw_hash: [u8; 32] = [0; 32];
        raw_hash[0..32].copy_from_slice(input.as_ref());
        H256(raw_hash)
    }
}

impl std::convert::From<ring::digest::Digest> for H256 {
    fn from(input: ring::digest::Digest) -> H256 {
        let mut raw_hash: [u8; 32] = [0; 32];
        raw_hash[0..32].copy_from_slice(input.as_ref());
        H256(raw_hash)
    }
}

impl Ord for H256 {
    fn cmp(&self, other: &H256) -> std::cmp::Ordering {
        let self_higher = u128::from_be_bytes(self.0[0..16].try_into().unwrap());
        let self_lower = u128::from_be_bytes(self.0[16..32].try_into().unwrap());
        let other_higher = u128::from_be_bytes(other.0[0..16].try_into().unwrap());
        let other_lower = u128::from_be_bytes(other.0[16..32].try_into().unwrap());
        let higher = self_higher.cmp(&other_higher);
        match higher {
            std::cmp::Ordering::Equal => self_lower.cmp(&other_lower),
            _ => higher,
        }
    }
}

impl PartialOrd for H256 {
    fn partial_cmp(&self, other: &H256) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub fn generate_random_hash() -> H256 {
        let mut rng = rand::thread_rng();
        let random_bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
        let mut raw_bytes = [0; 32];
        raw_bytes.copy_from_slice(&random_bytes);
        (&raw_bytes).into()
    }

pub fn hash_divide_by(input: &H256, divide: f64) -> H256 {
        let mut result_bytes = [0;32];
        for n in 1..17 {
            let value = u16::from_be_bytes(input.0[2*(n-1)..2*n].try_into().unwrap());
            //println!{"{}",value};
            let value = value as f64;
            let result = value/divide;
            let result = result as u16;
            let results:[u8;2] = result.to_be_bytes();
            //println!{"{}",result};
            result_bytes[2*(n-1)]=results[0];
            result_bytes[2*(n-1)+1]=results[1];

        }
        (&result_bytes).into()

    }

#[cfg(any(test, test_utilities))]
pub mod tests {
    use super::H256;
    use rand::Rng;
    use std::convert::TryInto;

    pub fn generate_random_hash() -> H256 {
        let mut rng = rand::thread_rng();
        let random_bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
        let mut raw_bytes = [0; 32];
        raw_bytes.copy_from_slice(&random_bytes);
        (&raw_bytes).into()
    }

    pub fn hash_divide_by(input: &H256, divide: f64) -> H256 {
        let mut result_bytes = [0;32];
        for n in 1..17 {
            let value = u16::from_be_bytes(input.0[2*(n-1)..2*n].try_into().unwrap());
            //println!{"{}",value};
            let value = value as f64;
            let result = value/divide;
            let result = result as u16;
            let results:[u8;2] = result.to_be_bytes();
            //println!{"{}",result};
            result_bytes[2*(n-1)]=results[0];
            result_bytes[2*(n-1)+1]=results[1];

        }
        (&result_bytes).into()

    }

    #[test]
    fn hash_test() {
        let hash: H256 = <H256>::from([3; 32]);
        println!("{}",hash);
        let result = hash_divide_by(&hash,1.5);
        println!("{}",result);
        let ans: H256 = <H256>::from([2; 32]);
        println!("{}",ans);
        assert_eq!(result,ans);
    }

    use vrf::openssl::{CipherSuite, ECVRF};
    use vrf::VRF;

    #[test]

    fn vrf_test() {
        let mut vrf = ECVRF::from_suite(CipherSuite::SECP256K1_SHA256_TAI).unwrap();
        // Inputs: Secret Key, Public Key (derived) & Message
        let secret_key =
            hex::decode("c9afa9d845ba75166b5c215767b1d6934e50c3db36e89b127b8a622b120f6721").unwrap();
        let public_key = vrf.derive_public_key(&secret_key).unwrap();
        let message: &[u8] = b"sample";

        // VRF proof and hash output
        let pi = vrf.prove(&secret_key, &message).unwrap();
        let hash = vrf.proof_to_hash(&pi).unwrap();
        println!("Generated VRF proof: {}", hex::encode(&pi));

        // VRF proof verification (returns VRF hash output)
        let beta = vrf.verify(&public_key, &pi, &message);

        match beta {
            Ok(beta) => {
                println!("VRF proof is valid!\nHash output: {}", hex::encode(&beta));
                assert_eq!(hash, beta);
            }
            Err(e) => {
                println!("VRF proof is not valid: {}", e);
            }
        }
    }
}

