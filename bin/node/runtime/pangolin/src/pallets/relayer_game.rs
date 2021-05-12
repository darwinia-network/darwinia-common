// --- darwinia ---
pub use darwinia_relayer_game::Instance1 as EthereumRelayerGameInstance;

// --- substrate ---
use frame_support::traits::LockIdentifier;
// --- darwinia ---
use crate::*;
use darwinia_relayer_game::Config;

frame_support::parameter_types! {
	pub const EthereumRelayerGameLockId: LockIdentifier = *b"ethrgame";
}
impl Config<EthereumRelayerGameInstance> for Runtime {
	type RingCurrency = Ring;
	type LockId = EthereumRelayerGameLockId;
	type RingSlash = Treasury;
	type RelayerGameAdjustor = relay::EthereumRelayerGameAdjustor;
	type RelayableChain = EthereumRelay;
	type WeightInfo = ();
}
