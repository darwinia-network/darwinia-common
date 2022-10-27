pub use pallet_bridge_messages::Instance1 as WithPangolinMessages;

// --- darwinia-network ---
use crate::*;
use bp_darwinia_core::{
	AccountId, AccountIdConverter, Balance, MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX,
	MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX,
};
use bp_messages::MessageNonce;
use bp_runtime::{ChainId, PANGOLIN_CHAIN_ID};
use pallet_bridge_messages::Config;
use pallet_fee_market::s2s::{
	FeeMarketMessageAcceptedHandler, FeeMarketMessageConfirmedHandler, FeeMarketPayment,
};

frame_support::parameter_types! {
	pub const BridgedChainId: ChainId = PANGOLIN_CHAIN_ID;
	pub const MaxMessagesToPruneAtOnce: MessageNonce = 8;
	pub const MaxUnconfirmedMessagesAtInboundLane: MessageNonce =
		MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX;
	pub const MaxUnrewardedRelayerEntriesAtInboundLane: MessageNonce =
	MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX;
}

impl Config<WithPangolinMessages> for Runtime {
	type AccountIdConverter = AccountIdConverter;
	type BridgedChainId = BridgedChainId;
	type Event = Event;
	type InboundMessageFee = Balance;
	type InboundPayload = bm_pangolin::FromPangolinMessagePayload;
	type InboundRelayer = AccountId;
	type LaneMessageVerifier = bm_pangolin::ToPangolinMessageVerifier;
	type MaxMessagesToPruneAtOnce = MaxMessagesToPruneAtOnce;
	type MaxUnconfirmedMessagesAtInboundLane = MaxUnconfirmedMessagesAtInboundLane;
	type MaxUnrewardedRelayerEntriesAtInboundLane = MaxUnrewardedRelayerEntriesAtInboundLane;
	// TODO: update this?
	type MaximalOutboundPayloadSize = frame_support::traits::ConstU32<1024>;
	type MessageDeliveryAndDispatchPayment = FeeMarketPayment<Self, WithPangolinFeeMarket, Ring>;
	type MessageDispatch = bm_pangolin::FromPangolinMessageDispatch;
	type OnDeliveryConfirmed = FeeMarketMessageConfirmedHandler<Self, WithPangolinFeeMarket>;
	type OnMessageAccepted = FeeMarketMessageAcceptedHandler<Self, WithPangolinFeeMarket>;
	type OutboundMessageFee = Balance;
	type OutboundPayload = bm_pangolin::ToPangolinMessagePayload;
	type Parameter = bm_pangolin::PangoroToPangolinMessagesParameter;
	type SourceHeaderChain = bm_pangolin::Pangolin;
	type TargetHeaderChain = bm_pangolin::Pangolin;
	type WeightInfo = ();
}
