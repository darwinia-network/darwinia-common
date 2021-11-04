pub use pallet_bridge_messages::Instance1 as Pangolin;

// --- paritytech ---
use bp_messages::LaneId;
use bp_runtime::{messages::DispatchFeePayment, ChainId};
use bridge_runtime_common::messages::source::FromThisChainMessagePayload;
use frame_support::PalletId;
use pangoro_primitives::AccountId;
// --- darwinia-network ---
use crate::{
	pangolin_messages::{ToPangolinMessagePayloadBox, PANGOLIN_S2S_ISSUING_PALLET_INDEX},
	*,
};
use bridge_primitives::{AccountIdConverter, PANGORO_PANGOLIN_LANE};
use darwinia_support::s2s::LatestMessageNoncer;
use dp_s2s::{CallParams, PayloadCreate};
use to_substrate_backing::Config;

// /// Create message payload according to call parameters
// pub struct PangolinPayLoadCreator;
// impl PayloadCreate<AccountId, ToPangolinMessagePayload> for PangolinPayLoadCreator {
// 	fn payload(
// 		submitter: AccountId,
// 		spec_version: u32,
// 		weight: u64,
// 		call_params: CallParams,
// 	) -> Result<ToPangolinMessagePayload, &'static str> {
// 		let call = Self::encode_call(PANGOLIN_S2S_ISSUING_PALLET_INDEX, call_params)?;
// 		return Ok(FromThisChainMessagePayload::<WithPangolinMessageBridge> {
// 			spec_version,
// 			weight,
// 			origin: bp_message_dispatch::CallOrigin::SourceAccount(submitter),
// 			call,
// 			dispatch_fee_payment: DispatchFeePayment::AtSourceChain,
// 		});
// 	}
// }

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
	type OutboundPayload = ToPangolinMessagePayloadBox;
	type MessageNoncer = PangolinMessageNoncer;
	// type PayloadCreator = PangolinPayLoadCreator;
	type MessageLaneId = BridgePangolinLaneId;
	type MessagesBridge = BridgePangolinMessages;
}
