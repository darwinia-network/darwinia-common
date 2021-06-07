// --- substrate ---
use frame_support::PalletId;
// --- darwinia ---
use crate::bridges::substrate::millau_messages::{MillauCallToPayload, ToMillauMessagePayload};
use crate::*;
use darwinia_s2s_relay::Config;
pub use darwinia_s2s_relay::Instance1 as DarwiniaS2sRelay;

frame_support::parameter_types! {
	pub const S2sRelayPalletId: PalletId = PalletId(*b"da/s2sre");
}

impl Config<DarwiniaS2sRelay> for Runtime {
	type PalletId = S2sRelayPalletId;
	type Event = Event;
	type WeightInfo = ();
	//TODO move target chain to runtime
	type TargetChain = [u8; 4];
	type OutboundPayload = ToMillauMessagePayload;
	type OutboundMessageFee = Balance;
	type CallToPayload = MillauCallToPayload;
	type MessageSenderT = BridgeMillauMessages;
}
