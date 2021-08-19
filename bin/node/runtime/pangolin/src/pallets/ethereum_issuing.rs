// --- substrate ---
use frame_support::PalletId;
// --- darwinia ---
use crate::*;
use darwinia_ethereum_issuing::Config;

frame_support::parameter_types! {
	pub const EthereumIssuingPalletId: PalletId = PalletId(*b"da/ethis");
}

impl Config for Runtime {
	// FIXME: Remove tight couple with dvm_ethereum and change it to PalletId
	type IssuingPalletId = EthereumIssuingPalletId;
	type Event = Event;
	type EthereumRelay = EthereumRelay;
	type EcdsaAuthorities = EthereumRelayAuthorities;
	type WeightInfo = ();
}
