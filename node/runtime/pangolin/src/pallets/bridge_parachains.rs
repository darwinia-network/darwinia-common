pub use pallet_bridge_parachains::{
	Instance1 as WithRococoParachainInstance, Instance2 as WithMoonbaseRelayParachainInstance,
};

// --- darwinia-network ---
use crate::*;
use bp_polkadot_core::parachains::{ParaId, PARAS_PALLET_NAME};
use frame_support::traits::IsInVec;
use pallet_bridge_parachains::Config;

frame_support::parameter_types! {
	pub const ParasPalletName: &'static str = PARAS_PALLET_NAME;
	// TODO: Use this or Everything
	pub GetTenFirstParachains: Vec<ParaId> = (0..10).map(ParaId).collect();
}

impl Config<WithRococoParachainInstance> for Runtime {
	type BridgesGrandpaPalletInstance = WithRococoGrandpa;
	type Event = Event;
	type HeadsToKeep = RococoHeadersToKeep;
	type ParasPalletName = ParasPalletName;
	type TrackedParachains = frame_support::traits::Everything;
	type WeightInfo = ();
}
impl Config<WithMoonbaseRelayParachainInstance> for Runtime {
	type BridgesGrandpaPalletInstance = WithMoonbaseRelayGrandpa;
	type Event = Event;
	type HeadsToKeep = RococoHeadersToKeep;
	type ParasPalletName = ParasPalletName;
	type TrackedParachains = frame_support::traits::Everything;
	type WeightInfo = ();
}
