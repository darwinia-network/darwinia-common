pub use pallet_bridge_messages::Instance1 as WithPangolinMessages;

// --- paritytech ---
use bp_messages::MessageNonce;
use pallet_bridge_messages::{
	instant_payments::InstantCurrencyPayments, weights::RialtoWeight, Config,
};
// --- darwinia-network ---
use crate::*;
use bridge_primitives::{
	AccountIdConverter, MAX_SINGLE_MESSAGE_DELIVERY_CONFIRMATION_TX_WEIGHT,
	MAX_UNCONFIRMED_MESSAGES_AT_INBOUND_LANE, MAX_UNREWARDED_RELAYER_ENTRIES_AT_INBOUND_LANE,
	PANGOLIN_CHAIN_ID,
};
use darwinia_support::s2s;
use pangolin_messages::{
	FromPangolinMessageDispatch, FromPangolinMessagePayload, Pangolin,
	PangoroToPangolinMessagesParameter, ToPangolinMessagePayload, ToPangolinMessageVerifier,
};

frame_support::parameter_types! {
	pub const MaxMessagesToPruneAtOnce: MessageNonce = 8;
	pub const MaxUnrewardedRelayerEntriesAtInboundLane: MessageNonce =
		MAX_UNREWARDED_RELAYER_ENTRIES_AT_INBOUND_LANE;
	pub const MaxUnconfirmedMessagesAtInboundLane: MessageNonce =
		MAX_UNCONFIRMED_MESSAGES_AT_INBOUND_LANE;
	// `IdentityFee` is used by Pangoro => we may use weight directly
	pub const GetDeliveryConfirmationTransactionFee: Balance =
		MAX_SINGLE_MESSAGE_DELIVERY_CONFIRMATION_TX_WEIGHT as _;
	pub RootAccountForPayments: Option<AccountId> = Some(s2s::to_bytes32(b"root").into());
	pub const BridgedChainId: bp_runtime::ChainId = PANGOLIN_CHAIN_ID;
}

impl Config<WithPangolinMessages> for Runtime {
	type Event = Event;
	// FIXME
	type WeightInfo = RialtoWeight<Runtime>;
	type Parameter = PangoroToPangolinMessagesParameter;
	type MaxMessagesToPruneAtOnce = MaxMessagesToPruneAtOnce;
	type MaxUnrewardedRelayerEntriesAtInboundLane = MaxUnrewardedRelayerEntriesAtInboundLane;
	type MaxUnconfirmedMessagesAtInboundLane = MaxUnconfirmedMessagesAtInboundLane;

	type OutboundPayload = ToPangolinMessagePayload;
	type OutboundMessageFee = Balance;

	type InboundPayload = FromPangolinMessagePayload;
	type InboundMessageFee = pangolin_primitives::Balance;
	type InboundRelayer = pangolin_primitives::AccountId;

	type AccountIdConverter = AccountIdConverter;

	type TargetHeaderChain = Pangolin;
	type LaneMessageVerifier = ToPangolinMessageVerifier;
	type MessageDeliveryAndDispatchPayment = InstantCurrencyPayments<
		Runtime,
		darwinia_balances::Pallet<Runtime, RingInstance>,
		GetDeliveryConfirmationTransactionFee,
		RootAccountForPayments,
	>;

	type OnDeliveryConfirmed = ();

	type SourceHeaderChain = Pangolin;
	type MessageDispatch = FromPangolinMessageDispatch;
	type BridgedChainId = BridgedChainId;
}
