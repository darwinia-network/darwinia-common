use codec::{Decode, Encode};
use scale_info::TypeInfo;
// --- paritytech ---
use frame_support::{PalletId, RuntimeDebug};
// --- darwinia-network ---
use crate::{pangolin_parachain::*, weights::to_parachain_backing::WeightInfo, *};
use bp_darwinia_core::{AccountId, AccountIdConverter, AccountPublic, Balance, Signature};
use bp_message_dispatch::CallOrigin;
use bp_messages::LaneId;
use bp_runtime::{messages::DispatchFeePayment, ChainId, PANGOLIN_PARACHAIN_CHAIN_ID};
use bridge_runtime_common::lanes::PANGOLIN_PANGOLIN_PARACHAIN_LANE;
use to_parachain_backing::{Config, IssueFromRemotePayload, IssuingCall, LatestMessageNoncer};

pub struct PangolinParachainMessageNoncer;
impl LatestMessageNoncer for PangolinParachainMessageNoncer {
	fn outbound_latest_generated_nonce(lane_id: LaneId) -> u64 {
		pallet_bridge_messages::OutboundLanes::<Runtime, WithPangolinParachainMessages>::get(
			&lane_id,
		)
		.latest_generated_nonce
	}

	fn inbound_latest_received_nonce(lane_id: LaneId) -> u64 {
		pallet_bridge_messages::InboundLanes::<Runtime, WithPangolinParachainMessages>::get(
			&lane_id,
		)
		.last_delivered_nonce()
	}
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct ToPangolinParachainOutboundPayLoad;
impl IssueFromRemotePayload<AccountId, AccountPublic, Signature, Runtime>
	for ToPangolinParachainOutboundPayLoad
{
	type Payload = ToPangolinParachainMessagePayload;

	fn create(
		origin: CallOrigin<AccountId, AccountPublic, Signature>,
		spec_version: u32,
		weight: u64,
		call_params: IssuingCall<Runtime>,
		dispatch_fee_payment: DispatchFeePayment,
	) -> Result<Self::Payload, &'static str> {
		const PANGOLIN_PARACHAIN_ISSUING_PALLET_INDEX: u8 = 24;

		let mut call = vec![PANGOLIN_PARACHAIN_ISSUING_PALLET_INDEX];
		call.append(&mut call_params.encode());
		Ok(Self::Payload { spec_version, weight, origin, call, dispatch_fee_payment })
	}
}

frame_support::parameter_types! {
	pub const PangolinParachainChainId: ChainId = PANGOLIN_PARACHAIN_CHAIN_ID;
	pub const S2sBackingPalletId: PalletId = PalletId(*b"pl/s2sba");
	pub const MaxLockRingAmountPerTx: Balance = 10_000 * COIN;
	pub const BridgePangolinParachainLaneId: LaneId = PANGOLIN_PANGOLIN_PARACHAIN_LANE;
}

impl Config for Runtime {
	type BridgedAccountIdConverter = AccountIdConverter;
	type BridgedChainId = PangolinParachainChainId;
	type Event = Event;
	type MaxLockRingAmountPerTx = MaxLockRingAmountPerTx;
	type MessageLaneId = BridgePangolinParachainLaneId;
	type MessageNoncer = PangolinParachainMessageNoncer;
	type MessagesBridge = BridgePangolinParachainMessages;
	type OutboundPayloadCreator = ToPangolinParachainOutboundPayLoad;
	type PalletId = S2sBackingPalletId;
	type RingCurrency = Ring;
	type WeightInfo = WeightInfo<Self>;
}
