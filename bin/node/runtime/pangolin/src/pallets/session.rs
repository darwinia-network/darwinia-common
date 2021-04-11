// --- substrate ---
use pallet_session::{historical::NoteHistoricalRoot, weights::SubstrateWeight, Config};
use sp_runtime::{impl_opaque_keys, traits::OpaqueKeys, Perbill};
use sp_std::prelude::*;
// --- darwinia ---
use crate::*;
use darwinia_staking::StashOf;

impl_opaque_keys! {
	pub struct SessionKeys {
		pub babe: Babe,
		pub grandpa: Grandpa,
		pub im_online: ImOnline,
		pub authority_discovery: AuthorityDiscovery,
	}
}
frame_support::parameter_types! {
	pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(17);
}
impl Config for Runtime {
	type Event = Event;
	type ValidatorId = AccountId;
	type ValidatorIdOf = StashOf<Self>;
	type ShouldEndSession = Babe;
	type NextSessionRotation = Babe;
	type SessionManager = NoteHistoricalRoot<Self, Staking>;
	type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
	type DisabledValidatorsThreshold = DisabledValidatorsThreshold;
	type WeightInfo = SubstrateWeight<Runtime>;
}
