// --- paritytech ---
use pallet_beefy_mmr::DepositBeefyDigest;
use pallet_mmr::Config;
use sp_runtime::traits::{Hash, Keccak256};
// --- darwinia-network ---
use crate::*;

pub type MmrHash = <Keccak256 as Hash>::Output;

impl Config for Runtime {
	type Hashing = Keccak256;
	type Hash = MmrHash;
	type LeafData = MmrLeaf;
	type OnNewRoot = DepositBeefyDigest<Runtime>;
	type WeightInfo = ();

	const INDEXING_PREFIX: &'static [u8] = b"mmr";
}
