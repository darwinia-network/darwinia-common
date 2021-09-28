pub use pallet_bridge_messages::Instance1 as WithPangoroMessages;

// --- substrate ---
use frame_support::pallet_prelude::Weight;
use sp_std::marker::PhantomData;
// --- paritytech ---
use bp_messages::{source_chain::OnDeliveryConfirmed, DeliveredMessages, LaneId, MessageNonce};

use pallet_bridge_messages::{weights::RialtoWeight, Config};
// --- darwinia-network ---
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
	PANGORO_CHAIN_ID, PANGORO_PANGOLIN_LANE,
};
use darwinia_fee_market::payment::FeeMarketPayment;
use darwinia_support::{
	s2s::{nonce_to_message_id, MessageConfirmer},
	to_bytes32,
};
use pallet_bridge_messages::Instance1;

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
	type MessageDeliveryAndDispatchPayment = FeeMarketPayment<
		Runtime,
		Instance1,
		darwinia_balances::Pallet<Runtime, RingInstance>,
		GetDeliveryConfirmationTransactionFee,
		RootAccountForPayments,
	>;

	type OnMessageAccepted = ();
	type OnDeliveryConfirmed = PangolinDeliveryConfirmer<Substrate2SubstrateIssuing>;

	type SourceHeaderChain = Pangoro;
	type MessageDispatch = FromPangoroMessageDispatch;
	type BridgedChainId = BridgedChainId;
}

pub struct PangolinDeliveryConfirmer<T: MessageConfirmer>(PhantomData<T>);

impl<T: MessageConfirmer> OnDeliveryConfirmed for PangolinDeliveryConfirmer<T> {
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
