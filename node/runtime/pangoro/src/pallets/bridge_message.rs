pub use pallet_bridge_messages::Instance1 as WithPangolinMessages;

// --- paritytech ---
use bp_messages::{source_chain::OnDeliveryConfirmed, DeliveredMessages, LaneId, MessageNonce};
use bp_runtime::ChainId;
use frame_support::pallet_prelude::Weight;
use pallet_bridge_messages::{
	instant_payments::InstantCurrencyPayments, weights::RialtoWeight, Config,
};
use sp_std::marker::PhantomData;
// --- darwinia-network ---
use crate::*;
use bridge_primitives::{
	AccountIdConverter, MAX_SINGLE_MESSAGE_DELIVERY_CONFIRMATION_TX_WEIGHT,
	MAX_UNCONFIRMED_MESSAGES_AT_INBOUND_LANE, MAX_UNREWARDED_RELAYER_ENTRIES_AT_INBOUND_LANE,
	PANGOLIN_CHAIN_ID, PANGORO_PANGOLIN_LANE,
};
use darwinia_support::{
	s2s::{nonce_to_message_id, MessageConfirmer},
	to_bytes32,
};
use pallet_bridge_messages::Instance1;
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
	pub RootAccountForPayments: Option<AccountId> = Some(to_bytes32(b"root").into());
	pub const BridgedChainId: ChainId = PANGOLIN_CHAIN_ID;
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
		WithPangolinMessages,
		Ring,
		GetDeliveryConfirmationTransactionFee,
		RootAccountForPayments,
	>;

	type OnMessageAccepted = ();
	type OnDeliveryConfirmed = PangoroDeliveryConfirmer<Substrate2SubstrateBacking>;

	type SourceHeaderChain = Pangolin;
	type MessageDispatch = FromPangolinMessageDispatch;
	type BridgedChainId = BridgedChainId;
}

pub struct PangoroDeliveryConfirmer<T: MessageConfirmer>(PhantomData<T>);

impl<T: MessageConfirmer> OnDeliveryConfirmed for PangoroDeliveryConfirmer<T> {
	fn on_messages_delivered(lane: &LaneId, messages: &DeliveredMessages) -> Weight {
		if *lane != PANGORO_PANGOLIN_LANE {
			return 0;
		}
		let mut total_weight: Weight = 0;
		for nonce in messages.begin..messages.end + 1 {
			let result = messages.message_dispatch_result(nonce);
			let message_id = nonce_to_message_id(lane, nonce);
			total_weight =
				total_weight.saturating_add(T::on_messages_confirmed(message_id, result));
		}
		total_weight
	}
}
