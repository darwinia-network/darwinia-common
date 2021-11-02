pub use pallet_bridge_messages::Instance1 as Pangolin;

// --- paritytech ---
use bp_message_dispatch::CallOrigin;
use bp_messages::LaneId;
use bp_runtime::{messages::DispatchFeePayment, ChainId};
use bridge_runtime_common::messages::source::FromThisChainMessagePayload;
use frame_support::{traits::PalletInfoAccess, weights::PostDispatchInfo, PalletId};
use frame_system::RawOrigin;
use sp_runtime::DispatchErrorWithPostInfo;
// --- darwinia-network ---
use crate::*;
use bridge_primitives::{AccountIdConverter, PANGORO_PANGOLIN_LANE};
use darwinia_support::s2s::{nonce_to_message_id, RelayMessageSender, TokenMessageId};
use dp_s2s::{CallParams, PayloadCreate};
use to_substrate_backing::Config;

/// Create message payload according to call parameters
pub struct PangolinPayLoadCreator;
impl PayloadCreate<AccountId, ToPangolinMessagePayload> for PangolinPayLoadCreator {
	fn payload(
		spec_version: u32,
		weight: u64,
		call_params: CallParams<AccountId>,
	) -> Result<ToPangolinMessagePayload, ()> {
		let call = Self::encode_call(49, call_params)?;
		return Ok(FromThisChainMessagePayload::<WithPangolinMessageBridge> {
			spec_version,
			weight,
			origin: CallOrigin::SourceRoot,
			call,
			dispatch_fee_payment: DispatchFeePayment::AtSourceChain,
		});
	}
}

/// Send payload to the messages pallet
pub struct ToPangolinMessageSender;
impl ToPangolinMessageSender {
	fn send_message_call(
		pallet_index: u32,
		lane_id: [u8; 4],
		payload: Vec<u8>,
		fee: u128,
	) -> Result<Call, &'static str> {
		let payload = ToPangolinMessagePayload::decode(&mut payload.as_slice())
			.map_err(|_| "decode pangolin payload failed")?;
		let call: Call = match pallet_index {
			_ if pallet_index as usize == <BridgePangolinMessages as PalletInfoAccess>::index() => {
				BridgeMessagesCall::<Runtime, Pangolin>::send_message(
					lane_id,
					payload,
					fee.saturated_into(),
				)
				.into()
			}
			_ => {
				return Err("invalid pallet index".into());
			}
		};
		Ok(call)
	}
}

impl RelayMessageSender for ToPangolinMessageSender {
	fn encode_send_message(
		pallet_index: u32,
		lane_id: [u8; 4],
		payload: Vec<u8>,
		fee: u128,
	) -> Result<Vec<u8>, &'static str> {
		let call = Self::send_message_call(pallet_index, lane_id, payload, fee)?;
		Ok(call.encode())
	}

	fn send_message_by_root(
		pallet_index: u32,
		lane_id: [u8; 4],
		payload: Vec<u8>,
		fee: u128,
	) -> Result<PostDispatchInfo, DispatchErrorWithPostInfo<PostDispatchInfo>> {
		let call = Self::send_message_call(pallet_index, lane_id, payload, fee)?;
		call.dispatch(RawOrigin::Root.into())
	}

	fn latest_token_message_id(lane_id: [u8; 4]) -> TokenMessageId {
		let nonce: u64 = BridgePangolinMessages::outbound_latest_generated_nonce(lane_id).into();
		nonce_to_message_id(&lane_id, nonce)
	}

	fn latest_received_token_message_id(lane_id: [u8; 4]) -> TokenMessageId {
		let nonce: u64 = BridgePangolinMessages::inbound_latest_received_nonce(lane_id).into();
		nonce_to_message_id(&lane_id, nonce)
	}
}

frame_support::parameter_types! {
	pub const PangolinChainId: ChainId = PANGOLIN_CHAIN_ID;
	pub const RingPalletId: PalletId = PalletId(*b"da/bring");
	pub const S2sBackingPalletId: PalletId = PalletId(*b"da/s2sba");
	pub const MaxLockRingAmountPerTx: Balance = 10_000 * COIN;
	pub BridgePangolinIndex: u32 = <BridgePangolinMessages as PalletInfoAccess>::index() as u32;
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

	type OutboundPayload = ToPangolinMessagePayload;
	type PayloadCreator = PangolinPayLoadCreator;

	type FeeAccount = RootAccountForPayments;
	type MessageSender = ToPangolinMessageSender;

	type MessageSendPalletIndex = BridgePangolinIndex;
	type MessageLaneId = BridgePangolinLaneId;
}
