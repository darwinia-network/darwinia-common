// --- paritytech ---
use frame_support::PalletId;
// --- darwinia-network ---
use crate::*;
use bp_messages::LaneId;
use bp_runtime::{ChainId, PANGOLIN_CHAIN_ID};
use bridge_runtime_common::lanes::PANGORO_PANGOLIN_LANE;
use darwinia_support::{evm::IntoH160, s2s::LatestMessageNoncer};
use dp_asset::{TokenMetadata, NATIVE_TOKEN_TYPE};
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
	pub RingMetadata: TokenMetadata = TokenMetadata::new(
		NATIVE_TOKEN_TYPE,
		PalletId(*b"da/bring").into_h160(),
		b"Pangoro Network Native Token".to_vec(),
		b"ORING".to_vec(),
		9);
	pub const S2sBackingPalletId: PalletId = PalletId(*b"da/s2sba");
	pub const MaxLockRingAmountPerTx: Balance = 10_000 * COIN;
	pub const BridgePangolinLaneId: LaneId = PANGORO_PANGOLIN_LANE;
}

impl Config for Runtime {
	type Event = Event;
	type WeightInfo = ();
	type PalletId = S2sBackingPalletId;
	type RingMetadata = RingMetadata;
	type MaxLockRingAmountPerTx = MaxLockRingAmountPerTx;
	type RingCurrency = Ring;
	type BridgedAccountIdConverter = bp_pangolin::AccountIdConverter;
	type BridgedChainId = PangolinChainId;
	type OutboundPayloadCreator = bm_pangolin::ToPangolinOutboundPayload;
	type S2sIssuingParamsEncoder = bm_pangolin::PangolinIssuingParamsEncoder;
	type MessageNoncer = PangolinMessageNoncer;
	type MessageLaneId = BridgePangolinLaneId;
	type MessagesBridge = BridgePangolinMessages;
}
