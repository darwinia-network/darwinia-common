// --- paritytech ---
use frame_support::PalletId;
// --- darwinia-network ---
use crate::*;
use darwinia_support::ChainName;
use from_ethereum_issuing::Config;

frame_support::parameter_types! {
	pub const EthereumIssuingPalletId: PalletId = PalletId(*b"da/ethis");
	pub BackingChainName: ChainName = (b"Ropsten").to_vec();
}

impl Config for Runtime {
	type BackingChainName = BackingChainName;
	type EcdsaAuthorities = EthereumRelayAuthorities;
	type EthereumRelay = EthereumRelay;
	type Event = Event;
	type InternalTransactHandler = Ethereum;
	type PalletId = EthereumIssuingPalletId;
	type RingCurrency = Ring;
	type WeightInfo = ();
}
