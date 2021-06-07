// --- substrate ---
use frame_support::PalletId;
// --- darwinia ---
use crate::bridges::substrate::millau_messages::{MillauCallToPayload, ToMillauMessagePayload};
use crate::*;
use darwinia_s2s_relay::Config;
pub use darwinia_s2s_relay::Instance1 as ToMillauRelay;

frame_support::parameter_types! {
	pub const S2sRelayPalletId: PalletId = PalletId(*b"da/s2sre");
	pub const MillauChainId: bp_runtime::ChainId = bp_runtime::MILLAU_CHAIN_ID;
}

impl Config<ToMillauRelay> for Runtime {
	type PalletId = S2sRelayPalletId;
	type Event = Event;
	type WeightInfo = ();
	type OutboundPayload = ToMillauMessagePayload;
	type OutboundMessageFee = Balance;
	type CallToPayload = MillauCallToPayload;
	type RemoteAssetReceiverT = MillauBackingReceiver;
	type ToEthereumAddressT = darwinia_s2s_relay::ConcatToEthereumAddress;
	type RemoteAccountIdConverter = bp_millau::AccountIdConverter;
	type MessageSenderT = BridgeMillauMessages;
	type RemoteChainId = MillauChainId;
}
