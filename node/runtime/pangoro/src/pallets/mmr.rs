// --- paritytech ---
use pallet_mmr::Config;
use sp_runtime::traits::{Hash, Keccak256};
// --- darwinia-network ---
use crate::*;
use darwinia_message_gadget::DepositBeefyDigest;
use dp_message::network_ids;

pub type MmrHash = <Keccak256 as Hash>::Output;

impl Config for Runtime {
	type Hash = MmrHash;
	type Hashing = Keccak256;
	type LeafData = MmrLeaf;
	type OnNewRoot = DepositBeefyDigest<Runtime, network_ids::Pangolin>;
	type WeightInfo = ();

	const INDEXING_PREFIX: &'static [u8] = b"mmr";
}
