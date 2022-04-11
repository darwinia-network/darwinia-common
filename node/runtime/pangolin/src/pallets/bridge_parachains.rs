pub use pallet_bridge_parachains::Instance1 as WithRococoParachainsInstance;

// --- darwinia-network ---
use crate::*;
use pallet_bridge_parachains::Config;

frame_support::parameter_types! {
	pub const PangolinParasPalletName: &'static str = bp_pangolin::BRIDGE_PARAS_PALLET_NAME;
}

impl Config<WithRococoParachainsInstance> for Runtime {
	type BridgesGrandpaPalletInstance = WithRococoGrandpa;
	type ParasPalletName = PangolinParasPalletName;
	type HeadsToKeep = HeadersToKeep;
}
