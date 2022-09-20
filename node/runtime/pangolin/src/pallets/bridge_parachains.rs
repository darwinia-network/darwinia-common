pub use pallet_bridge_parachains::Instance1 as WithRococoParachainsInstance;

// --- darwinia-network ---
use crate::*;
use pallet_bridge_parachains::Config;

impl Config<WithRococoParachainsInstance> for Runtime {
	type BridgesGrandpaPalletInstance = WithRococoGrandpa;
	type HeadsToKeep = RococoHeadersToKeep;
}
