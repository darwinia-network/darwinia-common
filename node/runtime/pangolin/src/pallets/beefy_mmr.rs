// --- paritytech ---
use beefy_primitives::mmr::MmrLeafVersion;
use pallet_beefy_mmr::{BeefyEcdsaToEthereum, Config};
// --- darwinia-network ---
use crate::*;

frame_support::parameter_types! {
	/// Version of the produced MMR leaf.
	///
	/// The version consists of two parts;
	/// - `major` (3 bits)
	/// - `minor` (5 bits)
	///
	/// `major` should be updated only if decoding the previous MMR Leaf format from the payload
	/// is not possible (i.e. backward incompatible change).
	/// `minor` should be updated if fields are added to the previous MMR Leaf, which given SCALE
	/// encoding does not prevent old leafs from being decoded.
	///
	/// Hence we expect `major` to be changed really rarely (think never).
	/// See [`MmrLeafVersion`] type documentation for more details.
	pub LeafVersion: MmrLeafVersion = MmrLeafVersion::new(0, 0);
}

impl Config for Runtime {
	type BeefyAuthorityToMerkleLeaf = BeefyEcdsaToEthereum;
	type LeafVersion = LeafVersion;
	type ParachainHeads = ();
}
