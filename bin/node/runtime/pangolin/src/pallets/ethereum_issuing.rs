// --- substrate ---
use frame_support::PalletId;
// --- darwinia ---
use crate::*;
use darwinia_ethereum_issuing::Config;

frame_support::parameter_types! {
	pub const EthereumIssuingPalletId: PalletId = PalletId(*b"da/ethis");
}

frame_support::parameter_types! {
	pub RawCallGasLimit: U256 = U256::from(300_000_000);
}

impl Config for Runtime {
	type PalletId = EthereumIssuingPalletId;
	type Event = Event;
	type EthereumRelay = EthereumRelay;
	type EcdsaAuthorities = EthereumRelayAuthorities;
	type RawCallGasLimit = RawCallGasLimit;
	type WeightInfo = ();
}
