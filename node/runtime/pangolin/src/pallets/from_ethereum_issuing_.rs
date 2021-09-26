// --- paritytech ---
use frame_support::PalletId;
// --- darwinia-network ---
use crate::*;
use darwinia_support::to_bytes32;
use from_ethereum_issuing::Config;

frame_support::parameter_types! {
	pub const EthereumIssuingPalletId: PalletId = PalletId(*b"da/ethis");
	pub RopstenName: [u8; 32] = to_bytes32(b"Ropsten").into();
}

impl Config for Runtime {
	type PalletId = EthereumIssuingPalletId;
	type Event = Event;
	type RingCurrency = Ring;
	type EthereumRelay = EthereumRelay;
	type EcdsaAuthorities = EthereumRelayAuthorities;
	type WeightInfo = ();
	type InternalTransactHandler = Ethereum;
	type BridgeChainName = RopstenName;
}
