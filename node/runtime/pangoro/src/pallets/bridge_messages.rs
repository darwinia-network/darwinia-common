pub use pallet_bridge_messages::Instance1 as WithPangolinMessages;

// --- darwinia-network ---
use crate::*;
use bp_messages::MessageNonce;
use bp_runtime::{ChainId, PANGOLIN_CHAIN_ID};
use darwinia_support::evm::{ConcatConverter, DeriveEthereumAddress, DeriveSubstrateAddress};
use pallet_bridge_messages::Config;
use pallet_fee_market::s2s::{
	FeeMarketMessageAcceptedHandler, FeeMarketMessageConfirmedHandler, FeeMarketPayment,
};

frame_support::parameter_types! {
	pub const MaxMessagesToPruneAtOnce: MessageNonce = 8;
	// TODO: remove this after FeeMarketPayment upgrade
	pub const RootAccountForPayments: Option<AccountId> = Some(ConcatConverter::<_>::derive_substrate_address((&b"root"[..]).derive_ethereum_address()));
	pub const MaxUnrewardedRelayerEntriesAtInboundLane: MessageNonce =
		bp_pangolin::MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX;
	pub const MaxUnconfirmedMessagesAtInboundLane: MessageNonce =
		bp_pangolin::MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX;
	pub const BridgedChainId: ChainId = PANGOLIN_CHAIN_ID;
}

impl Config<WithPangolinMessages> for Runtime {
	type AccountIdConverter = bp_pangoro::AccountIdConverter;
	type BridgedChainId = BridgedChainId;
	type Event = Event;
	type InboundMessageFee = bp_pangolin::Balance;
	type InboundPayload = bm_pangolin::FromPangolinMessagePayload;
	type InboundRelayer = bp_pangolin::AccountId;
	type LaneMessageVerifier = bm_pangolin::ToPangolinMessageVerifier;
	type MaxMessagesToPruneAtOnce = MaxMessagesToPruneAtOnce;
	type MaxUnconfirmedMessagesAtInboundLane = MaxUnconfirmedMessagesAtInboundLane;
	type MaxUnrewardedRelayerEntriesAtInboundLane = MaxUnrewardedRelayerEntriesAtInboundLane;
	type MessageDeliveryAndDispatchPayment =
		FeeMarketPayment<Self, WithPangolinFeeMarket, Ring, RootAccountForPayments>;
	type MessageDispatch = bm_pangolin::FromPangolinMessageDispatch;
	type OnDeliveryConfirmed =
		(Substrate2SubstrateBacking, FeeMarketMessageConfirmedHandler<Self, WithPangolinFeeMarket>);
	type OnMessageAccepted = FeeMarketMessageAcceptedHandler<Self, WithPangolinFeeMarket>;
	type OutboundMessageFee = bp_pangoro::Balance;
	type OutboundPayload = bm_pangolin::ToPangolinMessagePayload;
	type Parameter = bm_pangolin::PangoroToPangolinMessagesParameter;
	type SourceHeaderChain = bm_pangolin::Pangolin;
	type TargetHeaderChain = bm_pangolin::Pangolin;
	type WeightInfo = ();
}
