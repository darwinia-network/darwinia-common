pub use sp_consensus_aura::sr25519::AuthorityId as AuraId;

// --- paritytech ---
use pallet_aura::Config;
// --- darwinia-network ---
use crate::*;

sp_runtime::impl_opaque_keys! {
	pub struct SessionKeys {
		pub aura: Aura,
		pub grandpa: Grandpa,
	}
}

impl Config for Runtime {
	type AuthorityId = AuraId;
	type DisabledValidators = ();
}
