pub use pallet_bridge_messages::{
	Instance1 as WithPangoroMessages, Instance2 as WithPangolinParachainMessages,
	Instance3 as WithPangolinParachainAlphaMessages,
};

// --- darwinia-network ---
use crate::*;
use bp_darwinia_core::{
	AccountId, AccountIdConverter, Balance, MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX,
	MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX,
};
use bp_messages::MessageNonce;
use bp_runtime::{
	ChainId, PANGOLIN_PARACHAIN_ALPHA_CHAIN_ID, PANGOLIN_PARACHAIN_CHAIN_ID, PANGORO_CHAIN_ID,
};
use pallet_bridge_messages::Config;
use pallet_fee_market::s2s::{
	FeeMarketMessageAcceptedHandler, FeeMarketMessageConfirmedHandler, FeeMarketPayment,
};

frame_support::parameter_types! {
	// Shared configurations.
	pub const MaxMessagesToPruneAtOnce: MessageNonce = 8;
	pub const MaxUnconfirmedMessagesAtInboundLane: MessageNonce =
		MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX;
	pub const MaxUnrewardedRelayerEntriesAtInboundLane: MessageNonce =
		MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX;
	// Pangoro configurations.
	pub const PangoroChainId: ChainId = PANGORO_CHAIN_ID;
	// Pangolin Parachain configurations.
	pub const PangolinParachainChainId: ChainId = PANGOLIN_PARACHAIN_CHAIN_ID;
	pub const PangolinParachainAlphaChainId: ChainId = PANGOLIN_PARACHAIN_ALPHA_CHAIN_ID;
}

impl Config<WithPangoroMessages> for Runtime {
	type AccountIdConverter = AccountIdConverter;
	type BridgedChainId = PangoroChainId;
	type Event = Event;
	type InboundMessageFee = Balance;
	type InboundPayload = bm_pangoro::FromPangoroMessagePayload;
	type InboundRelayer = AccountId;
	type LaneMessageVerifier = bm_pangoro::ToPangoroMessageVerifier;
	type MaxMessagesToPruneAtOnce = MaxMessagesToPruneAtOnce;
	type MaxUnconfirmedMessagesAtInboundLane = MaxUnconfirmedMessagesAtInboundLane;
	type MaxUnrewardedRelayerEntriesAtInboundLane = MaxUnrewardedRelayerEntriesAtInboundLane;
	// TODO: update this?
	type MaximalOutboundPayloadSize = frame_support::traits::ConstU32<1024>;
	type MessageDeliveryAndDispatchPayment = FeeMarketPayment<Self, WithPangoroFeeMarket, Ring>;
	type MessageDispatch = bm_pangoro::FromPangoroMessageDispatch;
	type OnDeliveryConfirmed = FeeMarketMessageConfirmedHandler<Self, WithPangoroFeeMarket>;
	type OnMessageAccepted = FeeMarketMessageAcceptedHandler<Self, WithPangoroFeeMarket>;
	type OutboundMessageFee = Balance;
	type OutboundPayload = bm_pangoro::ToPangoroMessagePayload;
	type Parameter = bm_pangoro::PangolinToPangoroMessagesParameter;
	type SourceHeaderChain = bm_pangoro::Pangoro;
	type TargetHeaderChain = bm_pangoro::Pangoro;
	type WeightInfo = ();
}
impl Config<WithPangolinParachainMessages> for Runtime {
	type AccountIdConverter = AccountIdConverter;
	type BridgedChainId = PangolinParachainChainId;
	type Event = Event;
	type InboundMessageFee = Balance;
	type InboundPayload = bm_pangolin_parachain::FromPangolinParachainMessagePayload;
	type InboundRelayer = AccountId;
	type LaneMessageVerifier = bm_pangolin_parachain::ToPangolinParachainMessageVerifier;
	type MaxMessagesToPruneAtOnce = MaxMessagesToPruneAtOnce;
	type MaxUnconfirmedMessagesAtInboundLane = MaxUnconfirmedMessagesAtInboundLane;
	type MaxUnrewardedRelayerEntriesAtInboundLane = MaxUnrewardedRelayerEntriesAtInboundLane;
	// TODO: update this?
	type MaximalOutboundPayloadSize = frame_support::traits::ConstU32<1024>;
	type MessageDeliveryAndDispatchPayment =
		FeeMarketPayment<Self, WithPangolinParachainFeeMarket, Ring>;
	type MessageDispatch = bm_pangolin_parachain::FromPangolinParachainMessageDispatch;
	type OnDeliveryConfirmed = (
		ToPangolinParachainBacking,
		FeeMarketMessageConfirmedHandler<Self, WithPangolinParachainFeeMarket>,
	);
	type OnMessageAccepted = FeeMarketMessageAcceptedHandler<Self, WithPangolinParachainFeeMarket>;
	type OutboundMessageFee = Balance;
	type OutboundPayload = bm_pangolin_parachain::ToPangolinParachainMessagePayload;
	type Parameter = bm_pangolin_parachain::PangolinToPangolinParachainParameter;
	type SourceHeaderChain = bm_pangolin_parachain::PangolinParachain;
	type TargetHeaderChain = bm_pangolin_parachain::PangolinParachain;
	type WeightInfo = ();
}
impl Config<WithPangolinParachainAlphaMessages> for Runtime {
	type AccountIdConverter = AccountIdConverter;
	type BridgedChainId = PangolinParachainAlphaChainId;
	type Event = Event;
	type InboundMessageFee = Balance;
	type InboundPayload = bm_pangolin_parachain_alpha::FromPangolinParachainAlphaMessagePayload;
	type InboundRelayer = AccountId;
	type LaneMessageVerifier = bm_pangolin_parachain_alpha::ToPangolinParachainAlphaMessageVerifier;
	type MaxMessagesToPruneAtOnce = MaxMessagesToPruneAtOnce;
	type MaxUnconfirmedMessagesAtInboundLane = MaxUnconfirmedMessagesAtInboundLane;
	type MaxUnrewardedRelayerEntriesAtInboundLane = MaxUnrewardedRelayerEntriesAtInboundLane;
	// TODO: update this?
	type MaximalOutboundPayloadSize = frame_support::traits::ConstU32<1024>;
	type MessageDeliveryAndDispatchPayment =
		FeeMarketPayment<Self, WithPangolinParachainAlphaFeeMarket, Ring>;
	type MessageDispatch = bm_pangolin_parachain_alpha::FromPangolinParachainAlphaMessageDispatch;
	type OnDeliveryConfirmed = (
		ToPangolinParachainBacking,
		FeeMarketMessageConfirmedHandler<Self, WithPangolinParachainAlphaFeeMarket>,
	);
	type OnMessageAccepted =
		FeeMarketMessageAcceptedHandler<Self, WithPangolinParachainAlphaFeeMarket>;
	type OutboundMessageFee = Balance;
	type OutboundPayload = bm_pangolin_parachain_alpha::ToPangolinParachainAlphaMessagePayload;
	type Parameter = bm_pangolin_parachain_alpha::PangolinToPangolinParachainAlphaParameter;
	type SourceHeaderChain = bm_pangolin_parachain_alpha::PangolinParachainAlpha;
	type TargetHeaderChain = bm_pangolin_parachain_alpha::PangolinParachainAlpha;
	type WeightInfo = ();
}
