pub use pallet_bridge_messages::Instance1 as WithPangolinMessages;

// --- paritytech ---
use bp_messages::MessageNonce;
use bp_pangolin::{
	AccountIdConverter, MAX_SINGLE_MESSAGE_DELIVERY_CONFIRMATION_TX_WEIGHT,
	MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX, MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX,
};
use bp_runtime::{ChainId, PANGOLIN_CHAIN_ID};
use pallet_bridge_messages::Config;
// --- darwinia-network ---
use crate::*;
use darwinia_fee_market::s2s::{
	FeeMarketMessageAcceptedHandler, FeeMarketMessageConfirmedHandler, FeeMarketPayment,
};
use darwinia_support::evm::{ConcatConverter, IntoAccountId, IntoH160};
use pangolin_messages::{
	FromPangolinMessageDispatch, FromPangolinMessagePayload, Pangolin,
	PangoroToPangolinMessagesParameter, ToPangolinMessagePayload, ToPangolinMessageVerifier,
};

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
	pub const BridgedChainId: ChainId = PANGOLIN_CHAIN_ID;
}

impl Config<WithPangolinMessages> for Runtime {
	type Event = Event;
	type WeightInfo = ();
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
	type LaneMessageVerifier = ToPangolinMessageVerifier<Self>;
	type MessageDeliveryAndDispatchPayment = FeeMarketPayment<
		Runtime,
		WithPangolinMessages,
		Ring,
		GetDeliveryConfirmationTransactionFee,
		RootAccountForPayments,
	>;

	type OnMessageAccepted = FeeMarketMessageAcceptedHandler<Self>;
	type OnDeliveryConfirmed = (
		Substrate2SubstrateBacking,
		FeeMarketMessageConfirmedHandler<Self>,
	);

	type SourceHeaderChain = Pangolin;
	type MessageDispatch = FromPangolinMessageDispatch;
	type BridgedChainId = BridgedChainId;
}
