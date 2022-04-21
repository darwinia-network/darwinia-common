// --- paritytech ---
use frame_support::PalletId;
// --- darwinia-network ---
use crate::*;
use bp_messages::LaneId;
use bp_runtime::{ChainId, PANGOLIN_PARACHAIN_CHAIN_ID};
use bridge_runtime_common::lanes::PANGOLIN_PANGOLIN_PARACHAIN_LANE;
use darwinia_support::{evm::IntoH160, s2s::LatestMessageNoncer};
use dp_asset::{TokenMetadata, NATIVE_TOKEN_TYPE};
use to_substrate_backing::Config;

pub struct PangolinParachainMessageNoncer;
impl LatestMessageNoncer for PangolinParachainMessageNoncer {
	fn outbound_latest_generated_nonce(lane_id: LaneId) -> u64 {
		BridgePangolinParachainMessages::outbound_latest_generated_nonce(lane_id).into()
	}

	fn inbound_latest_received_nonce(lane_id: LaneId) -> u64 {
		BridgePangolinParachainMessages::inbound_latest_received_nonce(lane_id).into()
	}
}

frame_support::parameter_types! {
	pub const PangolinParachainChainId: ChainId = PANGOLIN_PARACHAIN_CHAIN_ID;
	pub RingMetadata: TokenMetadata = TokenMetadata::new(
		NATIVE_TOKEN_TYPE,
		PalletId(*b"da/pring").into_h160(),
		b"Pangolin Network Native Token".to_vec(),
		b"PRING".to_vec(),
		9);
	pub const S2sBackingPalletId: PalletId = PalletId(*b"pl/s2sba");
	pub const MaxLockRingAmountPerTx: Balance = 10_000 * COIN;
	pub const BridgePangolinParachainLaneId: LaneId = PANGOLIN_PANGOLIN_PARACHAIN_LANE;
}

impl Config for Runtime {
	type Event = Event;
	type WeightInfo = ();
	type PalletId = S2sBackingPalletId;
	type RingMetadata = RingMetadata;
	type MaxLockRingAmountPerTx = MaxLockRingAmountPerTx;
	type RingCurrency = Ring;
	type BridgedAccountIdConverter = bp_pangolin_parachain::AccountIdConverter;
	type BridgedChainId = PangolinParachainChainId;
	type OutboundPayloadCreator = bm_pangolin_parachain::ToPangolinParachainOutboundPayload;
	type S2sIssuingParamsEncoder = bm_pangolin_parachain::PangolinParachainIssuingParamsEncoder;
	type MessageNoncer = PangolinParachainMessageNoncer;
	type MessageLaneId = BridgePangolinParachainLaneId;
	type MessagesBridge = BridgePangolinParachainMessages;
}
