// --- paritytech ---
use frame_support::PalletId;
// --- darwinia-network ---
use crate::*;
use bp_messages::LaneId;
use bp_runtime::{ChainId, PANGOLIN_CHAIN_ID};
use bridge_runtime_common::lanes::PANGORO_PANGOLIN_LANE;
use darwinia_support::{evm::DeriveEthAddress, s2s::LatestMessageNoncer};
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
		PalletId(*b"da/bring").derive_eth_address(),
		b"Pangoro Network Native Token".to_vec(),
		b"ORING".to_vec(),
		9);
	pub const S2sBackingPalletId: PalletId = PalletId(*b"da/s2sba");
	pub const MaxLockRingAmountPerTx: Balance = 10_000 * COIN;
	pub const BridgePangolinLaneId: LaneId = PANGORO_PANGOLIN_LANE;
}

impl Config for Runtime {
	type BridgedAccountIdConverter = bp_pangolin::AccountIdConverter;
	type BridgedChainId = PangolinChainId;
	type Event = Event;
	type MaxLockRingAmountPerTx = MaxLockRingAmountPerTx;
	type MessageLaneId = BridgePangolinLaneId;
	type MessageNoncer = PangolinMessageNoncer;
	type MessagesBridge = BridgePangolinMessages;
	type OutboundPayloadCreator = bm_pangolin::ToPangolinOutboundPayload;
	type PalletId = S2sBackingPalletId;
	type RingCurrency = Ring;
	type RingMetadata = RingMetadata;
	type WeightInfo = ();
}
