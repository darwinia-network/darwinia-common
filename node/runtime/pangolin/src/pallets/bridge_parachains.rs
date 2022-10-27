pub use pallet_bridge_parachains::{
	Instance1 as WithRococoParachainInstance, Instance2 as WithMoonbaseRelayParachainInstance,
};

// --- darwinia-network ---
use crate::*;
use bp_polkadot_core::parachains::ParaId;
use frame_support::traits::IsInVec;
use pallet_bridge_parachains::Config;

pub const PARAS_PALLET_NAME: &str = "Paras";

frame_support::parameter_types! {
	// TODO: update these two config
	pub const ParasPalletName: &'static str = PARAS_PALLET_NAME;
	pub GetTenFirstParachains: Vec<ParaId> = (0..10).map(ParaId).collect();
}

impl Config<WithRococoParachainInstance> for Runtime {
	type BridgesGrandpaPalletInstance = WithRococoGrandpa;
	type Event = Event;
	type HeadsToKeep = RococoHeadersToKeep;
	type ParasPalletName = ParasPalletName;
	type TrackedParachains = IsInVec<GetTenFirstParachains>;
	type WeightInfo = ();
}
impl Config<WithMoonbaseRelayParachainInstance> for Runtime {
	type BridgesGrandpaPalletInstance = WithMoonbaseRelayGrandpa;
	type Event = Event;
	type HeadsToKeep = RococoHeadersToKeep;
	type ParasPalletName = ParasPalletName;
	type TrackedParachains = IsInVec<GetTenFirstParachains>;
	type WeightInfo = ();
}
