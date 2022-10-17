pub use pallet_bridge_parachains::{
	Instance1 as WithRococoParachainInstance, Instance2 as WithMoonbaseRelayParachainInstance,
};

// --- darwinia-network ---
use crate::*;
use pallet_bridge_parachains::Config;

impl Config<WithRococoParachainInstance> for Runtime {
	type BridgesGrandpaPalletInstance = WithRococoGrandpa;
	type HeadsToKeep = RococoHeadersToKeep;
}
impl Config<WithMoonbaseRelayParachainInstance> for Runtime {
	type BridgesGrandpaPalletInstance = WithMoonbaseRelayGrandpa;
	type HeadsToKeep = RococoHeadersToKeep;
}
