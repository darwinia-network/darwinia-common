pub use pallet_bridge_messages::Instance1 as WithPangoroMessages;

// --- darwinia-network ---
use crate::{bridges_message::bm_pangoro, *};
use bp_messages::MessageNonce;
use bp_runtime::{ChainId, PANGORO_CHAIN_ID};
use darwinia_fee_market::s2s::{
	FeeMarketMessageAcceptedHandler, FeeMarketMessageConfirmedHandler, FeeMarketPayment,
};
use darwinia_support::evm::{ConcatConverter, IntoAccountId, IntoH160};
use pallet_bridge_messages::Config;

frame_support::parameter_types! {
	pub const MaxMessagesToPruneAtOnce: MessageNonce = 8;
	pub const MaxUnrewardedRelayerEntriesAtInboundLane: MessageNonce =
		bp_pangoro::MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX;
	pub const MaxUnconfirmedMessagesAtInboundLane: MessageNonce =
		bp_pangoro::MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX;
	// `IdentityFee` is used by Pangoro => we may use weight directly
	pub const GetDeliveryConfirmationTransactionFee: Balance =
		bp_pangolin::MAX_SINGLE_MESSAGE_DELIVERY_CONFIRMATION_TX_WEIGHT as _;
	pub RootAccountForPayments: Option<AccountId> = Some(ConcatConverter::<_>::into_account_id((&b"root"[..]).into_h160()));
	pub const BridgedChainId: ChainId = PANGORO_CHAIN_ID;
}

impl Config<WithPangoroMessages> for Runtime {
	type Event = Event;
	type WeightInfo = ();
	type Parameter = bm_pangoro::PangolinToPangoroMessagesParameter;
	type MaxMessagesToPruneAtOnce = MaxMessagesToPruneAtOnce;
	type MaxUnrewardedRelayerEntriesAtInboundLane = MaxUnrewardedRelayerEntriesAtInboundLane;
	type MaxUnconfirmedMessagesAtInboundLane = MaxUnconfirmedMessagesAtInboundLane;

	type OutboundPayload = bm_pangoro::ToPangoroMessagePayload;
	type OutboundMessageFee = Balance;

	type InboundPayload = bm_pangoro::FromPangoroMessagePayload;
	type InboundMessageFee = bp_pangoro::Balance;
	type InboundRelayer = bp_pangoro::AccountId;

	type AccountIdConverter = bp_pangolin::AccountIdConverter;

	type TargetHeaderChain = bm_pangoro::Pangoro;
	type LaneMessageVerifier = bm_pangoro::ToPangoroMessageVerifier<Self, WithPangoroFeeMarket>;
	type MessageDeliveryAndDispatchPayment =
		FeeMarketPayment<Runtime, WithPangoroFeeMarket, Ring, RootAccountForPayments>;

	type OnMessageAccepted = FeeMarketMessageAcceptedHandler<Self, WithPangoroFeeMarket>;
	type OnDeliveryConfirmed = (
		Substrate2SubstrateIssuing,
		FeeMarketMessageConfirmedHandler<Self, WithPangoroFeeMarket>,
	);

	type SourceHeaderChain = bm_pangoro::Pangoro;
	type MessageDispatch = bm_pangoro::FromPangoroMessageDispatch;
	type BridgedChainId = BridgedChainId;
}
