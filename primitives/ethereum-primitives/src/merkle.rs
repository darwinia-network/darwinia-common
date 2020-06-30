use array_bytes::{array_unchecked, fixed_hex_bytes_unchecked};
use codec::{Decode, Encode};
pub use ethereum_types::{H128, H512};
use sp_core::hashing::sha2_256;
use sp_runtime::RuntimeDebug;
use sp_std::vec::Vec;

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug)]
pub struct DoubleNodeWithMerkleProof {
	pub dag_nodes: [H512; 2],
	pub proof: Vec<H128>,
}

impl Default for DoubleNodeWithMerkleProof {
	fn default() -> DoubleNodeWithMerkleProof {
		DoubleNodeWithMerkleProof {
			dag_nodes: <[H512; 2]>::default(),
			proof: Vec::new(),
		}
	}
}

impl DoubleNodeWithMerkleProof {
	pub fn from_str_unchecked(s: &str) -> Self {
		let mut dag_nodes: Vec<H512> = Vec::new();
		let mut proof: Vec<H128> = Vec::new();
		for e in s.splitn(60, '"') {
			let l = e.len();
			if l == 34 {
				proof.push(fixed_hex_bytes_unchecked!(e, 16).into());
			} else if l == 130 {
				dag_nodes.push(fixed_hex_bytes_unchecked!(e, 64).into());
			} else if l > 34 {
				// should not be here
				panic!("the proofs are longer than 25");
			}
		}
		DoubleNodeWithMerkleProof {
			dag_nodes: [dag_nodes[0], dag_nodes[1]],
			proof,
		}
	}
}

impl DoubleNodeWithMerkleProof {
	pub fn apply_merkle_proof(&self, index: u64) -> H128 {
		fn hash_h128(l: H128, r: H128) -> H128 {
			let mut data = [0u8; 64];
			data[16..32].copy_from_slice(&(l.0));
			data[48..64].copy_from_slice(&(r.0));

			// `H256` is 32 length, truncate is safe; qed
			array_unchecked!(sha2_256(&data), 16, 16).into()
		}

		let mut data = [0u8; 128];
		data[..64].copy_from_slice(&(self.dag_nodes[0].0));
		data[64..].copy_from_slice(&(self.dag_nodes[1].0));

		// `H256` is 32 length, truncate is safe; qed
		let mut leaf = array_unchecked!(sha2_256(&data), 16, 16).into();
		for i in 0..self.proof.len() {
			if (index >> i as u64) % 2 == 0 {
				leaf = hash_h128(leaf, self.proof[i]);
			} else {
				leaf = hash_h128(self.proof[i], leaf);
			}
		}

		leaf
	}
}
