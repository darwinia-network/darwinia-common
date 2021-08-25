// --- paritytech ---
use pallet_aura::Config;
// --- darwinia-network ---
use crate::*;

impl Config for Runtime {
	type AuthorityId = AuraId;
}
