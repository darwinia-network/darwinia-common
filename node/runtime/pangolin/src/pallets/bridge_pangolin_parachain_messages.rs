pub use pallet_bridge_messages::Instance2 as WithPangolinParachainMessages;

// --- paritytech ---
use bp_messages::MessageNonce;
use pallet_bridge_messages::Config;
// --- darwinia-netwrok ---
use crate::{
	pangolin_parachain_messages::{
		FromPangolinParachainMessageDispatch, FromPangolinParachainMessagePayload,
		PangolinParachain, PangolinToPangolinParachainParameter, ToPangolinParachainMessagePayload,
		ToPangolinParachainMessageVerifier,
	},
	*,
};
use bp_pangolin::{
	AccountIdConverter, MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX,
	MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX,
};
use bp_runtime::PANGOLIN_PARACHAIN_CHAIN_ID;

frame_support::parameter_types! {
	pub const MaxMessagesToPruneAtOnce: MessageNonce = 8;
	pub const MaxUnrewardedRelayerEntriesAtInboundLane: MessageNonce = MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX;
	pub const MaxUnconfirmedMessagesAtInboundLane: MessageNonce = MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX;
	pub const BridgedParachainId: bp_runtime::ChainId = PANGOLIN_PARACHAIN_CHAIN_ID;
}

impl Config<WithPangolinParachainMessages> for Runtime {
	type Event = Event;
	type WeightInfo = ();
	type BridgedChainId = BridgedParachainId;
	type Parameter = PangolinToPangolinParachainParameter;
	type MaxMessagesToPruneAtOnce = MaxMessagesToPruneAtOnce;
	type MaxUnrewardedRelayerEntriesAtInboundLane = MaxUnrewardedRelayerEntriesAtInboundLane;
	type MaxUnconfirmedMessagesAtInboundLane = MaxUnconfirmedMessagesAtInboundLane;
	type OutboundPayload = ToPangolinParachainMessagePayload;
	type OutboundMessageFee = Balance;
	type InboundPayload = FromPangolinParachainMessagePayload;
	type InboundMessageFee = bp_pangolin_parachain::Balance;
	type InboundRelayer = bp_pangolin_parachain::AccountId;
	type AccountIdConverter = AccountIdConverter;
	type TargetHeaderChain = PangolinParachain;
	type LaneMessageVerifier = ToPangolinParachainMessageVerifier;
	type MessageDeliveryAndDispatchPayment =
		pallet_bridge_messages::instant_payments::InstantCurrencyPayments<
			Runtime,
			WithPangolinParachainMessages,
			Ring,
			GetDeliveryConfirmationTransactionFee,
			RootAccountForPayments,
		>;
	type OnMessageAccepted = ();
	type OnDeliveryConfirmed = ();
	type SourceHeaderChain = PangolinParachain;
	type MessageDispatch = FromPangolinParachainMessageDispatch;
}
