pub use pallet_bridge_messages::Instance1 as WithPangoroMessages;

// --- substrate ---
use bp_messages::MessageNonce;
use pallet_bridge_messages::{
	instant_payments::InstantCurrencyPayments, weights::RialtoWeight, Config,
};
// --- darwinia ---
use crate::{
	pangoro_messages::{
		FromPangoroMessageDispatch, FromPangoroMessagePayload, PangolinToPangoroMessagesParameter,
		Pangoro, ToPangoroMessagePayload, ToPangoroMessageVerifier,
	},
	*,
};
use bridge_primitives::{
	AccountIdConverter, MAX_SINGLE_MESSAGE_DELIVERY_CONFIRMATION_TX_WEIGHT,
	MAX_UNCONFIRMED_MESSAGES_AT_INBOUND_LANE, MAX_UNREWARDED_RELAYER_ENTRIES_AT_INBOUND_LANE,
	PANGORO_CHAIN_ID,
};
use darwinia_support::s2s::to_bytes32;

frame_support::parameter_types! {
	pub const MaxMessagesToPruneAtOnce: MessageNonce = 8;
	pub const MaxUnrewardedRelayerEntriesAtInboundLane: MessageNonce =
		MAX_UNREWARDED_RELAYER_ENTRIES_AT_INBOUND_LANE;
	pub const MaxUnconfirmedMessagesAtInboundLane: MessageNonce =
		MAX_UNCONFIRMED_MESSAGES_AT_INBOUND_LANE;
	// `IdentityFee` is used by Pangoro => we may use weight directly
	pub const GetDeliveryConfirmationTransactionFee: Balance =
		MAX_SINGLE_MESSAGE_DELIVERY_CONFIRMATION_TX_WEIGHT as _;
	pub RootAccountForPayments: Option<AccountId> = Some(to_bytes32(b"root").into());
	pub const BridgedChainId: bp_runtime::ChainId = PANGORO_CHAIN_ID;
}

impl Config<WithPangoroMessages> for Runtime {
	type Event = Event;
	// FIXME
	type WeightInfo = RialtoWeight<Runtime>;
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
	type LaneMessageVerifier = ToPangoroMessageVerifier;
	type MessageDeliveryAndDispatchPayment = InstantCurrencyPayments<
		Runtime,
		darwinia_balances::Pallet<Runtime, RingInstance>,
		GetDeliveryConfirmationTransactionFee,
		RootAccountForPayments,
	>;

	type OnDeliveryConfirmed = ();

	type SourceHeaderChain = Pangoro;
	type MessageDispatch = FromPangoroMessageDispatch;
	type BridgedChainId = BridgedChainId;
}
