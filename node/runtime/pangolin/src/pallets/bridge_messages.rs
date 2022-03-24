pub use pallet_bridge_messages::Instance1 as WithPangoroMessages;

// --- paritytech ---
use bp_messages::MessageNonce;
use bp_pangolin::{
	AccountIdConverter, MAX_SINGLE_MESSAGE_DELIVERY_CONFIRMATION_TX_WEIGHT,
	MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX, MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX,
};
use bp_runtime::PANGORO_CHAIN_ID;
use pallet_bridge_messages::Config;
// --- darwinia-network ---
use crate::{
	pangoro_messages::{
		FromPangoroMessageDispatch, FromPangoroMessagePayload, PangolinToPangoroMessagesParameter,
		Pangoro, ToPangoroMessagePayload, ToPangoroMessageVerifier,
	},
	*,
};
use darwinia_fee_market::s2s::{
	FeeMarketMessageAcceptedHandler, FeeMarketMessageConfirmedHandler, FeeMarketPayment,
};
use darwinia_support::evm::{ConcatConverter, IntoAccountId, IntoH160};

frame_support::parameter_types! {
	pub const MaxMessagesToPruneAtOnce: MessageNonce = 8;
	pub const MaxUnrewardedRelayerEntriesAtInboundLane: MessageNonce =
		MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX;
	pub const MaxUnconfirmedMessagesAtInboundLane: MessageNonce =
		MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX;
	// `IdentityFee` is used by Pangoro => we may use weight directly
	pub const GetDeliveryConfirmationTransactionFee: Balance =
		MAX_SINGLE_MESSAGE_DELIVERY_CONFIRMATION_TX_WEIGHT as _;
	pub RootAccountForPayments: Option<AccountId> = Some(ConcatConverter::<_>::into_account_id((&b"root"[..]).into_h160()));
	pub const BridgedChainId: bp_runtime::ChainId = PANGORO_CHAIN_ID;
}

impl Config<WithPangoroMessages> for Runtime {
	type Event = Event;
	type WeightInfo = ();
	type Parameter = PangolinToPangoroMessagesParameter;
	type MaxMessagesToPruneAtOnce = MaxMessagesToPruneAtOnce;
	type MaxUnrewardedRelayerEntriesAtInboundLane = MaxUnrewardedRelayerEntriesAtInboundLane;
	type MaxUnconfirmedMessagesAtInboundLane = MaxUnconfirmedMessagesAtInboundLane;

	type OutboundPayload = ToPangoroMessagePayload;
	type OutboundMessageFee = Balance;

	type InboundPayload = FromPangoroMessagePayload;
	type InboundMessageFee = pangoro_primitives::Balance;
	type InboundRelayer = pangoro_primitives::AccountId;

	type AccountIdConverter = AccountIdConverter;

	type TargetHeaderChain = Pangoro;
	type LaneMessageVerifier = ToPangoroMessageVerifier<Self>;
	type MessageDeliveryAndDispatchPayment = FeeMarketPayment<
		Runtime,
		WithPangoroMessages,
		Ring,
		GetDeliveryConfirmationTransactionFee,
		RootAccountForPayments,
	>;

	type OnMessageAccepted = FeeMarketMessageAcceptedHandler<Self>;
	type OnDeliveryConfirmed = (
		Substrate2SubstrateIssuing,
		FeeMarketMessageConfirmedHandler<Self>,
	);

	type SourceHeaderChain = Pangoro;
	type MessageDispatch = FromPangoroMessageDispatch;
	type BridgedChainId = BridgedChainId;
}
