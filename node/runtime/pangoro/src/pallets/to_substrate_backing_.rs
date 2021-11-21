// --- paritytech ---
use bp_messages::LaneId;
use bp_runtime::ChainId;
use frame_support::PalletId;
// --- darwinia-network ---
use crate::{pangolin_messages::ToPangolinOutboundPayload, *};
use bridge_primitives::{AccountIdConverter, PANGORO_PANGOLIN_LANE};
use darwinia_support::s2s::LatestMessageNoncer;
use to_substrate_backing::Config;
pub struct PangolinMessageNoncer;
impl LatestMessageNoncer for PangolinMessageNoncer {
	fn outbound_latest_generated_nonce(lane_id: LaneId) -> u64 {
		BridgePangolinMessages::outbound_latest_generated_nonce(lane_id).into()
	}

	fn inbound_latest_received_nonce(lane_id: LaneId) -> u64 {
		BridgePangolinMessages::inbound_latest_received_nonce(lane_id).into()
	}
}

frame_support::parameter_types! {
	pub const PangolinChainId: ChainId = PANGOLIN_CHAIN_ID;
	pub const RingPalletId: PalletId = PalletId(*b"da/bring");
	pub const S2sBackingPalletId: PalletId = PalletId(*b"da/s2sba");
	pub const MaxLockRingAmountPerTx: Balance = 10_000 * COIN;
	pub const BridgePangolinLaneId: LaneId = PANGORO_PANGOLIN_LANE;
}

impl Config for Runtime {
	type Event = Event;
	type WeightInfo = ();
	type PalletId = S2sBackingPalletId;
	type RingPalletId = RingPalletId;
	type MaxLockRingAmountPerTx = MaxLockRingAmountPerTx;
	type RingCurrency = Ring;
	type BridgedAccountIdConverter = AccountIdConverter;
	type BridgedChainId = PangolinChainId;
	type OutboundPayloadCreator = ToPangolinOutboundPayload;
	type MessageNoncer = PangolinMessageNoncer;
	type MessageLaneId = BridgePangolinLaneId;
	type MessagesBridge = BridgePangolinMessages;
}
