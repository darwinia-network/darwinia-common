pub use pallet_bridge_messages::Instance1 as Pangolin;

// --- paritytech ---
use bp_message_dispatch::CallOrigin;
use bp_messages::LaneId;
use bp_runtime::{messages::DispatchFeePayment, ChainId};
use bridge_runtime_common::messages::source::FromThisChainMessagePayload;
use frame_support::{traits::PalletInfoAccess, weights::PostDispatchInfo, PalletId};
use frame_system::RawOrigin;
use sp_core::H160;
use sp_runtime::DispatchErrorWithPostInfo;
// --- darwinia-network ---
use crate::*;
use bridge_primitives::{AccountIdConverter, PANGORO_PANGOLIN_LANE};
use darwinia_support::s2s::{nonce_to_message_id, RelayMessageSender, TokenMessageId};
use dp_asset::{token::Token, RecipientAccount};
use to_substrate_backing::{Config, EncodeCall};

/// Bridged chain pangolin call info
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub enum PangolinRuntime {
	/// Note: this index must be the same as the backing pallet in pangolin chain runtime
	#[codec(index = 49)]
	Sub2SubIssuing(PangolinSub2SubIssuingCall),
}

/// Something important to note:
/// The index below represent the call order in the pangolin issuing pallet call.
/// For example, `index = 1` point to the `register_from_remote` (second)call in pangolin runtime.
/// You must update the index here if you change the call order in Pangolin runtime.
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[allow(non_camel_case_types)]
pub enum PangolinSub2SubIssuingCall {
	#[codec(index = 1)]
	register_from_remote(Token),
	#[codec(index = 2)]
	issue_from_remote(Token, H160),
}

pub struct PangolinCallEncoder;
impl PangolinCallEncoder {
	/// Transfer call to message payload
	fn to_payload(spec_version: u32, weight: u64, call: Vec<u8>) -> ToPangolinMessagePayload {
		return FromThisChainMessagePayload::<WithPangolinMessageBridge> {
			spec_version,
			weight,
			origin: CallOrigin::SourceRoot,
			call,
			dispatch_fee_payment: DispatchFeePayment::AtSourceChain,
		};
	}
}
impl EncodeCall<AccountId, ToPangolinMessagePayload> for PangolinCallEncoder {
	/// Encode issuing pallet remote_register call
	fn encode_remote_register(
		spec_version: u32,
		weight: u64,
		token: Token,
	) -> ToPangolinMessagePayload {
		let call = PangolinRuntime::Sub2SubIssuing(
			PangolinSub2SubIssuingCall::register_from_remote(token),
		)
		.encode();
		Self::to_payload(spec_version, weight, call)
	}
	/// Encode issuing pallet remote_issue call
	fn encode_remote_issue(
		spec_version: u32,
		weight: u64,
		token: Token,
		recipient: RecipientAccount<AccountId>,
	) -> Result<ToPangolinMessagePayload, ()> {
		let call = match recipient {
			RecipientAccount::<AccountId>::EthereumAccount(r) => PangolinRuntime::Sub2SubIssuing(
				PangolinSub2SubIssuingCall::issue_from_remote(token, r),
			)
			.encode(),
			_ => return Err(()),
		};
		Ok(Self::to_payload(spec_version, weight, call))
	}
}

pub struct ToPangolinMessageRelayCaller;

impl ToPangolinMessageRelayCaller {
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

impl RelayMessageSender for ToPangolinMessageRelayCaller {
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
	type CallEncoder = PangolinCallEncoder;

	type FeeAccount = RootAccountForPayments;
	type MessageSender = ToPangolinMessageRelayCaller;

	type MessageSendPalletIndex = BridgePangolinIndex;
	type MessageLaneId = BridgePangolinLaneId;
}
