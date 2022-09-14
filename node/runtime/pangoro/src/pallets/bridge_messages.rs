pub use pallet_bridge_messages::Instance1 as WithPangolinMessages;

// --- darwinia-network ---
use crate::*;
use bp_messages::MessageNonce;
use bp_runtime::{ChainId, PANGOLIN_CHAIN_ID};
use darwinia_support::evm::{ConcatConverter, DeriveSubstrateAddress};
use pallet_bridge_messages::Config;
use pallet_fee_market::s2s::{
	FeeMarketMessageAcceptedHandler, FeeMarketMessageConfirmedHandler, FeeMarketPayment,
};

frame_support::parameter_types! {
	pub const BridgedChainId: ChainId = PANGOLIN_CHAIN_ID;
	pub const MaxMessagesToPruneAtOnce: MessageNonce = 8;
	pub const MaxUnconfirmedMessagesAtInboundLane: MessageNonce =
		bp_pangolin::MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX;
	pub const MaxUnrewardedRelayerEntriesAtInboundLane: MessageNonce =
		bp_pangolin::MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX;
	// 0x726f6f7400000000000000000000000000000000, b"root"
	pub RootAccountForPayments: Option<AccountId> = Some(ConcatConverter::<_>::derive_substrate_address(&H160([0x72, 0x6f, 0x6f, 0x74, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])));
}

impl Config<WithPangolinMessages> for Runtime {
	type AccountIdConverter = bp_pangoro::AccountIdConverter;
	type BridgedChainId = BridgedChainId;
	type Event = Event;
	type InboundMessageFee = bp_pangolin::Balance;
	type InboundPayload = bm_pangolin::FromPangolinMessagePayload;
	type InboundRelayer = bp_pangolin::AccountId;
	type LaneMessageVerifier = bm_pangolin::ToPangolinMessageVerifier;
	type MaxMessagesToPruneAtOnce = MaxMessagesToPruneAtOnce;
	type MaxUnconfirmedMessagesAtInboundLane = MaxUnconfirmedMessagesAtInboundLane;
	type MaxUnrewardedRelayerEntriesAtInboundLane = MaxUnrewardedRelayerEntriesAtInboundLane;
	type MessageDeliveryAndDispatchPayment =
		FeeMarketPayment<Self, WithPangolinFeeMarket, Ring, RootAccountForPayments>;
	type MessageDispatch = bm_pangolin::FromPangolinMessageDispatch;
	type OnDeliveryConfirmed = FeeMarketMessageConfirmedHandler<Self, WithPangolinFeeMarket>;
	type OnMessageAccepted = FeeMarketMessageAcceptedHandler<Self, WithPangolinFeeMarket>;
	type OutboundMessageFee = bp_pangoro::Balance;
	type OutboundPayload = bm_pangolin::ToPangolinMessagePayload;
	type Parameter = bm_pangolin::PangoroToPangolinMessagesParameter;
	type SourceHeaderChain = bm_pangolin::Pangolin;
	type TargetHeaderChain = bm_pangolin::Pangolin;
	type WeightInfo = ();
}
