pub use pallet_bridge_messages::Instance2 as WithPangolinParachainMessages;

// --- paritytech ---
use bp_messages::MessageNonce;
use pallet_bridge_messages::Config;
// --- darwinia-network ---
use crate::{bridges_message::bm_pangolin_parachain, *};
use bp_runtime::{ChainId, PANGOLIN_PARACHAIN_CHAIN_ID};
use darwinia_fee_market::s2s::{
	FeeMarketMessageAcceptedHandler, FeeMarketMessageConfirmedHandler, FeeMarketPayment,
};
use darwinia_support::evm::{ConcatConverter, IntoAccountId, IntoH160};

frame_support::parameter_types! {
	pub const MaxMessagesToPruneAtOnce: MessageNonce = 8;
	pub const MaxUnrewardedRelayerEntriesAtInboundLane: MessageNonce =
		bp_pangolin_parachain::MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX;
	pub const MaxUnconfirmedMessagesAtInboundLane: MessageNonce =
		bp_pangolin_parachain::MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX;
	pub const BridgedParachainId: ChainId = PANGOLIN_PARACHAIN_CHAIN_ID;
	pub RootAccountForPayments: Option<AccountId> = Some(ConcatConverter::<_>::into_account_id((&b"root"[..]).into_h160()));
}

impl Config<WithPangolinParachainMessages> for Runtime {
	type Event = Event;
	type WeightInfo = ();
	type Parameter = bm_pangolin_parachain::PangolinToPangolinParachainParameter;
	type MaxMessagesToPruneAtOnce = MaxMessagesToPruneAtOnce;
	type MaxUnrewardedRelayerEntriesAtInboundLane = MaxUnrewardedRelayerEntriesAtInboundLane;
	type MaxUnconfirmedMessagesAtInboundLane = MaxUnconfirmedMessagesAtInboundLane;

	type OutboundPayload = bm_pangolin_parachain::ToPangolinParachainMessagePayload;
	type OutboundMessageFee = Balance;

	type InboundPayload = bm_pangolin_parachain::FromPangolinParachainMessagePayload;
	type InboundMessageFee = bp_pangolin_parachain::Balance;
	type InboundRelayer = bp_pangolin_parachain::AccountId;

	type AccountIdConverter = bp_pangolin::AccountIdConverter;

	type TargetHeaderChain = bm_pangolin_parachain::PangolinParachain;
	type LaneMessageVerifier = bm_pangolin_parachain::ToPangolinParachainMessageVerifier;
	type MessageDeliveryAndDispatchPayment =
		FeeMarketPayment<Runtime, PangolinParachainFeeMarket, Ring, RootAccountForPayments>;

	type OnMessageAccepted = FeeMarketMessageAcceptedHandler<Self, PangolinParachainFeeMarket>;
	type OnDeliveryConfirmed = (
		Substrate2SubstrateIssuing,
		FeeMarketMessageConfirmedHandler<Self, PangolinParachainFeeMarket>,
	);

	type SourceHeaderChain = bm_pangolin_parachain::PangolinParachain;
	type MessageDispatch = bm_pangolin_parachain::FromPangolinParachainMessageDispatch;
	type BridgedChainId = BridgedParachainId;
}
