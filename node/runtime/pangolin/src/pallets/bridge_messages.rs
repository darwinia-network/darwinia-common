pub use pallet_bridge_messages::{
	Instance1 as WithPangoroMessages, Instance2 as WithPangolinParachainMessages,
};

// --- darwinia-network ---
use crate::*;
use bp_messages::MessageNonce;
use bp_runtime::{ChainId, PANGOLIN_PARACHAIN_CHAIN_ID, PANGORO_CHAIN_ID};
use darwinia_fee_market::s2s::{
	FeeMarketMessageAcceptedHandler, FeeMarketMessageConfirmedHandler, FeeMarketPayment,
};
use darwinia_support::evm::{ConcatConverter, IntoAccountId, IntoH160};
use pallet_bridge_messages::Config;

frame_support::parameter_types! {
	// Shared configurations.
	pub const MaxMessagesToPruneAtOnce: MessageNonce = 8;
	pub RootAccountForPayments: Option<AccountId> = Some(ConcatConverter::<_>::into_account_id((&b"root"[..]).into_h160()));
	// Pangoro configurations.
	pub const PangoroMaxUnrewardedRelayerEntriesAtInboundLane: MessageNonce =
		bp_pangoro::MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX;
	pub const PangoroMaxUnconfirmedMessagesAtInboundLane: MessageNonce =
		bp_pangoro::MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX;
	pub const PangoroChainId: ChainId = PANGORO_CHAIN_ID;
	// Pangolin Parachain configurations.
	pub const PangolinParachainMaxUnrewardedRelayerEntriesAtInboundLane: MessageNonce =
		bp_pangolin_parachain::MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX;
	pub const PangolinParachainMaxUnconfirmedMessagesAtInboundLane: MessageNonce =
		bp_pangolin_parachain::MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX;
	pub const PangolinParachainChainId: ChainId = PANGOLIN_PARACHAIN_CHAIN_ID;
}

impl Config<WithPangoroMessages> for Runtime {
	type Event = Event;
	type WeightInfo = ();
	type Parameter = bm_pangoro::PangolinToPangoroMessagesParameter;
	type MaxMessagesToPruneAtOnce = MaxMessagesToPruneAtOnce;
	type MaxUnrewardedRelayerEntriesAtInboundLane = PangoroMaxUnrewardedRelayerEntriesAtInboundLane;
	type MaxUnconfirmedMessagesAtInboundLane = PangoroMaxUnconfirmedMessagesAtInboundLane;

	type OutboundPayload = bm_pangoro::ToPangoroMessagePayload;
	type OutboundMessageFee = bp_pangolin::Balance;

	type InboundPayload = bm_pangoro::FromPangoroMessagePayload;
	type InboundMessageFee = bp_pangoro::Balance;
	type InboundRelayer = bp_pangoro::AccountId;

	type AccountIdConverter = bp_pangolin::AccountIdConverter;

	type TargetHeaderChain = bm_pangoro::Pangoro;
	type LaneMessageVerifier = bm_pangoro::ToPangoroMessageVerifier;
	type MessageDeliveryAndDispatchPayment =
		FeeMarketPayment<Runtime, WithPangoroFeeMarket, Ring, RootAccountForPayments>;

	type OnMessageAccepted = FeeMarketMessageAcceptedHandler<Self, WithPangoroFeeMarket>;
	type OnDeliveryConfirmed = (
		Substrate2SubstrateIssuing,
		FeeMarketMessageConfirmedHandler<Self, WithPangoroFeeMarket>,
	);

	type SourceHeaderChain = bm_pangoro::Pangoro;
	type MessageDispatch = bm_pangoro::FromPangoroMessageDispatch;
	type BridgedChainId = PangoroChainId;
}
impl Config<WithPangolinParachainMessages> for Runtime {
	type Event = Event;
	type WeightInfo = ();
	type Parameter = bm_pangolin_parachain::PangolinToPangolinParachainParameter;
	type MaxMessagesToPruneAtOnce = MaxMessagesToPruneAtOnce;
	type MaxUnrewardedRelayerEntriesAtInboundLane =
		PangolinParachainMaxUnrewardedRelayerEntriesAtInboundLane;
	type MaxUnconfirmedMessagesAtInboundLane = PangolinParachainMaxUnconfirmedMessagesAtInboundLane;

	type OutboundPayload = bm_pangolin_parachain::ToPangolinParachainMessagePayload;
	type OutboundMessageFee = Balance;

	type InboundPayload = bm_pangolin_parachain::FromPangolinParachainMessagePayload;
	type InboundMessageFee = bp_pangolin_parachain::Balance;
	type InboundRelayer = bp_pangolin_parachain::AccountId;

	type AccountIdConverter = bp_pangolin::AccountIdConverter;

	type TargetHeaderChain = bm_pangolin_parachain::PangolinParachain;
	type LaneMessageVerifier = bm_pangolin_parachain::ToPangolinParachainMessageVerifier;
	type MessageDeliveryAndDispatchPayment =
		FeeMarketPayment<Runtime, WithPangolinParachainFeeMarket, Ring, RootAccountForPayments>;

	type OnMessageAccepted = FeeMarketMessageAcceptedHandler<Self, WithPangolinParachainFeeMarket>;
	type OnDeliveryConfirmed = (
		Substrate2SubstrateIssuing,
		FeeMarketMessageConfirmedHandler<Self, WithPangolinParachainFeeMarket>,
	);

	type SourceHeaderChain = bm_pangolin_parachain::PangolinParachain;
	type MessageDispatch = bm_pangolin_parachain::FromPangolinParachainMessageDispatch;
	type BridgedChainId = PangolinParachainChainId;
}
