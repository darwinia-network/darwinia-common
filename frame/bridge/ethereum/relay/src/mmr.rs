/// MMR for Ethereum
/// No mater the hash function of chain,
/// the Merge of Ethereum MMR used in shadow service is blake2b
use blake2_rfc::blake2b::blake2b;
pub use ckb_merkle_mountain_range::{
	leaf_index_to_mmr_size, leaf_index_to_pos, Merge, MerkleProof,
};
use sp_std::vec;

/// BlakeTwo256 hash function
pub fn hash(data: &[u8]) -> [u8; 32] {
	let mut dest = [0; 32];
	dest.copy_from_slice(blake2b(32, &[], data).as_bytes());
	dest
}

/// MMR Merge trait for ETHash
pub struct MergeHash;

impl Merge for MergeHash {
	type Item = [u8; 32];
	fn merge(lhs: &Self::Item, rhs: &Self::Item) -> Self::Item {
		let mut data = vec![];
		data.append(&mut lhs.to_vec());
		data.append(&mut rhs.to_vec());
		hash(&data.as_slice())
	}
}
