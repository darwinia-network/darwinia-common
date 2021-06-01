// --- substrate ---
use frame_support::PalletId;
// --- darwinia ---
use crate::*;
use crate::bridges::substrate::millau_messages::{
    MillauCallToPayload,
    ToMillauMessagePayload,
};
pub use pallet_bridge_messages::Instance1 as WithMillauMessages;
use darwinia_s2s_relay::Config;

frame_support::parameter_types! {
	pub const S2sRelayPalletId: PalletId = PalletId(*b"da/s2sre");
}

impl Config for Runtime {
	type PalletId = S2sRelayPalletId;
	type Event = Event;
	type WeightInfo = ();
    type OutboundPayload = ToMillauMessagePayload;
    type OutboundMessageFee = Balance;
    type CallToPayload = MillauCallToPayload;
    type MessageSenderT = BridgeMillauMessages;
}
