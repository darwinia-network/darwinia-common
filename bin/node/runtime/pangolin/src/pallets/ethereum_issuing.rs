// --- substrate ---
use frame_support::PalletId;
// --- darwinia ---
use crate::*;
use darwinia_ethereum_issuing::Config;

frame_support::parameter_types! {
	pub const EthereumIssuingPalletId: PalletId = PalletId(*b"da/ethis");
}

impl Config for Runtime {
	type PalletId = EthereumIssuingPalletId;
	type Event = Event;
	type EthereumRelay = EthereumRelay;
	type RingCurrency = Ring;
	type EcdsaAuthorities = EthereumRelayAuthorities;
	type WeightInfo = ();
}
