// --- substrate ---
use frame_support::PalletId;
// --- darwinia ---
use crate::bridges::substrate::millau_messages::{MillauCallToPayload, ToMillauMessagePayload};
use crate::*;
use darwinia_s2s_relay::Config;
pub use pallet_bridge_messages::Instance1 as WithMillauMessages;

frame_support::parameter_types! {
	pub const S2sRelayPalletId: PalletId = PalletId(*b"da/s2sre");
}

impl Config<DarwiniaS2sRelay> for Runtime {
	type PalletId = S2sRelayPalletId;
	type Event = Event;
	type WeightInfo = ();
	type OutboundPayload = ToMillauMessagePayload;
	type OutboundMessageFee = Balance;
	type CallToPayload = MillauCallToPayload;
	type MessageSenderT = BridgeMillauMessages;
}
