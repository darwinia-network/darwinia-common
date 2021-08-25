use crate::*;

parameter_types! {
	pub const MaxMessagesToPruneAtOnce: bp_messages::MessageNonce = 8;
	pub const MaxUnrewardedRelayerEntriesAtInboundLane: bp_messages::MessageNonce =
		bridge_primitives::MAX_UNREWARDED_RELAYER_ENTRIES_AT_INBOUND_LANE;
	pub const MaxUnconfirmedMessagesAtInboundLane: bp_messages::MessageNonce =
		bridge_primitives::MAX_UNCONFIRMED_MESSAGES_AT_INBOUND_LANE;
	// `IdentityFee` is used by Pangoro => we may use weight directly
	pub const GetDeliveryConfirmationTransactionFee: Balance =
		bridge_primitives::MAX_SINGLE_MESSAGE_DELIVERY_CONFIRMATION_TX_WEIGHT as _;
	pub RootAccountForPayments: Option<AccountId> = Some(to_bytes32(b"root").into());
}
pub type WithPangolinMessages = pallet_bridge_messages::Instance1;
impl pallet_bridge_messages::Config<WithPangolinMessages> for Runtime {
	type Event = Event;
	// FIXME
	type WeightInfo = pallet_bridge_messages::weights::RialtoWeight<Runtime>;
	type Parameter = pangolin_messages::PangoroToPangolinMessagesParameter;
	type MaxMessagesToPruneAtOnce = MaxMessagesToPruneAtOnce;
	type MaxUnrewardedRelayerEntriesAtInboundLane = MaxUnrewardedRelayerEntriesAtInboundLane;
	type MaxUnconfirmedMessagesAtInboundLane = MaxUnconfirmedMessagesAtInboundLane;

	type OutboundPayload = pangolin_messages::ToPangolinMessagePayload;
	type OutboundMessageFee = Balance;

	type InboundPayload = pangolin_messages::FromPangolinMessagePayload;
	type InboundMessageFee = pangolin_primitives::Balance;
	type InboundRelayer = pangolin_primitives::AccountId;

	type AccountIdConverter = bridge_primitives::AccountIdConverter;

	type TargetHeaderChain = pangolin_messages::Pangolin;
	type LaneMessageVerifier = pangolin_messages::ToPangolinMessageVerifier;
	type MessageDeliveryAndDispatchPayment =
		pallet_bridge_messages::instant_payments::InstantCurrencyPayments<
			Runtime,
			darwinia_balances::Pallet<Runtime, RingInstance>,
			GetDeliveryConfirmationTransactionFee,
			RootAccountForPayments,
		>;

	type SourceHeaderChain = pangolin_messages::Pangolin;
	type MessageDispatch = pangolin_messages::FromPangolinMessageDispatch;
}
