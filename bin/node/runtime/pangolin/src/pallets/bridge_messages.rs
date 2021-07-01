pub use pallet_bridge_messages::Instance1 as WithMillauMessages;

// --- substrate ---
use bp_messages::MessageNonce;
use millau_primitives::{
	MAX_SINGLE_MESSAGE_DELIVERY_CONFIRMATION_TX_WEIGHT, MAX_UNCONFIRMED_MESSAGES_AT_INBOUND_LANE,
	MAX_UNREWARDED_RELAYER_ENTRIES_AT_INBOUND_LANE,
};
use pallet_bridge_messages::{
	instant_payments::InstantCurrencyPayments, weights::RialtoWeight, Config,
};
// --- darwinia ---
use crate::{
	millau_messages::{
		FromMillauMessageDispatch, FromMillauMessagePayload, Millau,
		PangolinToMillauMessagesParameter, ToMillauMessagePayload, ToMillauMessageVerifier,
	},
	*,
};
use pangolin_bridge_primitives::AccountIdConverter;

frame_support::parameter_types! {
	pub const MaxMessagesToPruneAtOnce: MessageNonce = 8;
	pub const MaxUnrewardedRelayerEntriesAtInboundLane: MessageNonce =
		MAX_UNREWARDED_RELAYER_ENTRIES_AT_INBOUND_LANE;
	pub const MaxUnconfirmedMessagesAtInboundLane: MessageNonce =
		MAX_UNCONFIRMED_MESSAGES_AT_INBOUND_LANE;
	// `IdentityFee` is used by Millau => we may use weight directly
	pub const GetDeliveryConfirmationTransactionFee: Balance =
		MAX_SINGLE_MESSAGE_DELIVERY_CONFIRMATION_TX_WEIGHT as _;
	pub const RootAccountForPayments: Option<AccountId> = None;
}

impl Config<WithMillauMessages> for Runtime {
	type Event = Event;
	// FIXME
	type WeightInfo = RialtoWeight<Runtime>;
	type Parameter = PangolinToMillauMessagesParameter;
	type MaxMessagesToPruneAtOnce = MaxMessagesToPruneAtOnce;
	type MaxUnrewardedRelayerEntriesAtInboundLane = MaxUnrewardedRelayerEntriesAtInboundLane;
	type MaxUnconfirmedMessagesAtInboundLane = MaxUnconfirmedMessagesAtInboundLane;

	type OutboundPayload = ToMillauMessagePayload;
	type OutboundMessageFee = Balance;

	type InboundPayload = FromMillauMessagePayload;
	type InboundMessageFee = millau_primitives::Balance;
	type InboundRelayer = millau_primitives::AccountId;

	type AccountIdConverter = AccountIdConverter;

	type TargetHeaderChain = Millau;
	type LaneMessageVerifier = ToMillauMessageVerifier;
	type MessageDeliveryAndDispatchPayment = InstantCurrencyPayments<
		Runtime,
		darwinia_balances::Pallet<Runtime, RingInstance>,
		GetDeliveryConfirmationTransactionFee,
		RootAccountForPayments,
	>;

	type SourceHeaderChain = Millau;
	type MessageDispatch = FromMillauMessageDispatch;
}
