pub use beefy_primitives::crypto::AuthorityId as BeefyId;

// --- paritytech ---
use pallet_beefy::Config;
// --- darwinia-network ---
use crate::*;

impl Config for Runtime {
	type BeefyId = BeefyId;
}
