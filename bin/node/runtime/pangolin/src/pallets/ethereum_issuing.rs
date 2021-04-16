// --- substrate ---
use sp_runtime::ModuleId;
// --- darwinia ---
use crate::*;
use darwinia_ethereum_issuing::Config;

frame_support::parameter_types! {
	pub const EthereumIssuingModuleId: ModuleId = ModuleId(*b"da/ethis");
}

impl Config for Runtime {
	type ModuleId = EthereumIssuingModuleId;
	type Event = Event;
	type EthereumRelay = EthereumRelay;
	type RingCurrency = Ring;
	type EcdsaAuthorities = EthereumRelayAuthorities;
	type WeightInfo = ();
}
