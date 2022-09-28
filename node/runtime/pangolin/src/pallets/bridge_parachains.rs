pub use pallet_bridge_parachains::{Instance1 as WithRococoParachainsInstance, Instance2 as WithMoonbaseRelayParachainsInstance};

// --- darwinia-network ---
use crate::*;
use pallet_bridge_parachains::Config;

frame_support::parameter_types! {
	pub const PangolinParasPalletName: &'static str = bp_pangolin::BRIDGE_PARAS_PALLET_NAME;
}

impl Config<WithRococoParachainsInstance> for Runtime {
	type BridgesGrandpaPalletInstance = WithRococoGrandpa;
	type HeadsToKeep = RococoHeadersToKeep;
	type ParasPalletName = PangolinParasPalletName;
}
impl Config<WithMoonbaseRelayParachainsInstance> for Runtime {
	type BridgesGrandpaPalletInstance = WithMoonbaseRelayGrandpa;
	type HeadsToKeep = RococoHeadersToKeep;
	type ParasPalletName = PangolinParasPalletName;
}
