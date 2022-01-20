// --- crates.io ---
use codec::{Decode, Encode};
use scale_info::TypeInfo;
// --- paritytech ---
use pallet_mmr::{
	primitives::{LeafDataProvider, OnNewRoot},
	Config,
};
use sp_runtime::{generic::DigestItem, RuntimeDebug};
use sp_std::borrow::ToOwned;
// --- darwinia-network ---
use crate::*;

pub struct MmrLeafDataProvider;
impl LeafDataProvider for MmrLeafDataProvider {
	type LeafData = Hash;

	fn leaf_data() -> Self::LeafData {
		System::block_hash(System::parent_hash())
	}
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct ParentMmrRootLog {
	/// Specific prefix to identify the mmr root log in the digest items with Other type.
	pub prefix: [u8; 4],
	/// The merkle mountain range root.
	pub root: Hash,
}
impl ParentMmrRootLog {
	/// The prefix of [`ParentMmrRootLog`]
	pub const PREFIX: [u8; 4] = *b"MMRR";

	pub fn new(root: Hash) -> Self {
		Self {
			prefix: Self::PREFIX,
			root,
		}
	}
}

pub struct OnNewMmrRoot;
impl OnNewRoot<Hash> for OnNewMmrRoot {
	fn on_new_root(root: &Hash) {
		let mmr_root_log = ParentMmrRootLog::new(root.to_owned());
		let mmr_item = DigestItem::Other(mmr_root_log.encode());

		System::deposit_log(mmr_item.into());
	}
}

impl Config for Runtime {
	type Hashing = Hashing;
	type Hash = Hash;
	type LeafData = MmrLeafDataProvider;
	type OnNewRoot = OnNewMmrRoot;
	type WeightInfo = ();

	const INDEXING_PREFIX: &'static [u8] = b"header-mmr-";
}
