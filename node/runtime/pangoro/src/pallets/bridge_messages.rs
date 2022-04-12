pub use pallet_bridge_messages::Instance1 as WithPangolinMessages;

// --- darwinia-network ---
use crate::{bridges_message::bm_pangolin, *};
use bp_messages::MessageNonce;
use bp_runtime::{ChainId, PANGOLIN_CHAIN_ID};
use darwinia_fee_market::s2s::{
	FeeMarketMessageAcceptedHandler, FeeMarketMessageConfirmedHandler, FeeMarketPayment,
};
use darwinia_support::evm::{ConcatConverter, IntoAccountId, IntoH160};
use pallet_bridge_messages::Config;

frame_support::parameter_types! {
	pub const MaxMessagesToPruneAtOnce: MessageNonce = 8;
	pub const MaxUnrewardedRelayerEntriesAtInboundLane: MessageNonce =
		bp_pangolin::MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX;
	pub const MaxUnconfirmedMessagesAtInboundLane: MessageNonce =
		bp_pangolin::MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX;
	// `IdentityFee` is used by Pangoro => we may use weight directly
	pub const GetDeliveryConfirmationTransactionFee: Balance =
		bp_pangoro::MAX_SINGLE_MESSAGE_DELIVERY_CONFIRMATION_TX_WEIGHT as _;
	pub RootAccountForPayments: Option<AccountId> = Some(ConcatConverter::<_>::into_account_id((&b"root"[..]).into_h160()));
	pub const BridgedChainId: ChainId = PANGOLIN_CHAIN_ID;
}

impl Config<WithPangolinMessages> for Runtime {
	type Event = Event;
	type WeightInfo = ();
	type Parameter = bm_pangolin::PangoroToPangolinMessagesParameter;
	type MaxMessagesToPruneAtOnce = MaxMessagesToPruneAtOnce;
	type MaxUnrewardedRelayerEntriesAtInboundLane = MaxUnrewardedRelayerEntriesAtInboundLane;
	type MaxUnconfirmedMessagesAtInboundLane = MaxUnconfirmedMessagesAtInboundLane;

	type OutboundPayload = bm_pangolin::ToPangolinMessagePayload;
	type OutboundMessageFee = Balance;

	type InboundPayload = bm_pangolin::FromPangolinMessagePayload;
	type InboundMessageFee = bp_pangolin::Balance;
	type InboundRelayer = bp_pangolin::AccountId;

	type AccountIdConverter = bp_pangoro::AccountIdConverter;

	type TargetHeaderChain = bm_pangolin::Pangolin;
	type LaneMessageVerifier =
		bm_pangolin::ToPangolinMessageVerifier<Self, FeeMarketWorkForPangolin>;
	type MessageDeliveryAndDispatchPayment = FeeMarketPayment<
		Runtime,
		WithPangolinMessages,
		Ring,
		GetDeliveryConfirmationTransactionFee,
		RootAccountForPayments,
	>;

	type OnMessageAccepted = FeeMarketMessageAcceptedHandler<Self, FeeMarketWorkForPangolin>;
	type OnDeliveryConfirmed = (
		Substrate2SubstrateBacking,
		FeeMarketMessageConfirmedHandler<Self, FeeMarketWorkForPangolin>,
	);

	type SourceHeaderChain = bm_pangolin::Pangolin;
	type MessageDispatch = bm_pangolin::FromPangolinMessageDispatch;
	type BridgedChainId = BridgedChainId;
}
