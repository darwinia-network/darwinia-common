// --- substrate ---
use sp_runtime::ModuleId;
// --- darwinia ---
use crate::*;
use darwinia_ethereum_issuing::Config;

frame_support::parameter_types! {
	pub const EthereumIssuingModuleId: ModuleId = ModuleId(*b"da/ethis");
	pub const FeeEstimate: Balance = 30 * COIN;
}

impl Config for Runtime {
	type ModuleId = EthereumIssuingModuleId;
	type Event = Event;
	type DvmCaller = Ethereum;
	type EthereumRelay = EthereumRelay;
	type RingCurrency = Ring;
	type EcdsaAuthorities = EthereumRelayAuthorities;
	type WeightInfo = ();
	type FeeEstimate = FeeEstimate;
}
