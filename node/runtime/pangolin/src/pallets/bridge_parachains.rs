pub use pallet_bridge_parachains::Instance1 as WithRococoParachainsInstance;

// --- paritytech ---
use pallet_bridge_parachains::Config;
use bp_pangolin::BRIDGE_PARAS_PALLET_NAME;
// --- darwinia-network ---
use crate::*;

frame_support::parameter_types! {
	pub const PangolinParasPalletName: &'static str = BRIDGE_PARAS_PALLET_NAME;
}

impl Config<WithRococoParachainsInstance> for Runtime {
	type BridgesGrandpaPalletInstance = WithRococoGrandpa;
	type ParasPalletName = PangolinParasPalletName;
	type HeadsToKeep = HeadersToKeep;
}
