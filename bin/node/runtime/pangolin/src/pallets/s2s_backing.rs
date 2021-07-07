use crate::*;
use bridge_runtime_common::messages::{
	source::{estimate_message_dispatch_and_delivery_fee, FromThisChainMessagePayload},
	MessageBridge,
};
use codec::{Decode, Encode};
use darwinia_s2s_backing::EncodeCall;
use darwinia_support::s2s::{to_bytes32, RelayMessageCaller};
use dp_asset::RecipientAccount;
use frame_support::{dispatch::Dispatchable, weights::PostDispatchInfo, PalletId};
use frame_system::RawOrigin;
use millau_messages::{ToMillauMessagePayload, WithMillauMessageBridge};
use sp_runtime::DispatchErrorWithPostInfo;

/// Bridged chain Millau call info
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub enum MillauRuntime {
	/// Note: this index must be the same as the backing pallet in millau chain runtime
	#[codec(index = 49)]
	Sub2SubIssing(MillauSub2SubIssuingCall),
}

/// Something important to note:
/// The index below represent the call order in the millau issuing pallet call.
/// For example, `index = 1` point to the `register_from_remote` (second)call in millau runtime.
/// You must update the index here if you change the call order in Panglin runtime.
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[allow(non_camel_case_types)]
pub enum MillauSub2SubIssuingCall {
	#[codec(index = 1)]
	register_from_remote(Token),
	#[codec(index = 2)]
	issue_from_remote(Token, H160),
}

pub struct MillauCallEncoder;
impl EncodeCall<AccountId, ToMillauMessagePayload> for MillauCallEncoder {
	/// Encode issuing pallet remote_register call
	fn encode_remote_register(
		spec_version: u32,
		weight: u64,
		token: Token,
	) -> ToMillauMessagePayload {
		let call =
			MillauRuntime::Sub2SubIssing(MillauSub2SubIssuingCall::register_from_remote(token))
				.encode();
		Self::to_payload(spec_version, weight, call)
	}
	/// Encode issuing pallet remote_issue call
	fn encode_remote_issue(
		spec_version: u32,
		weight: u64,
		token: Token,
		recipient: RecipientAccount<AccountId>,
	) -> Result<ToMillauMessagePayload, ()> {
		let call = match recipient {
			RecipientAccount::<AccountId>::EthereumAccount(r) => {
				MillauRuntime::Sub2SubIssing(MillauSub2SubIssuingCall::issue_from_remote(token, r))
					.encode()
			}
			_ => return Err(()),
		};
		Ok(Self::to_payload(spec_version, weight, call))
	}
}

impl MillauCallEncoder {
	/// Transfer call to message payload
	fn to_payload(spec_version: u32, weight: u64, call: Vec<u8>) -> ToMillauMessagePayload {
		return FromThisChainMessagePayload::<WithMillauMessageBridge> {
			spec_version,
			weight,
			origin: bp_message_dispatch::CallOrigin::SourceRoot,
			call,
		};
	}
}

pub const MILLAU_PANGOLIN_LANE: [u8; 4] = *b"mtpl";
use pallet_bridge_messages::Instance1 as Millau;
pub struct ToMillauMessageRelayCaller;
impl RelayMessageCaller<ToMillauMessagePayload, Balance> for ToMillauMessageRelayCaller {
	fn send_message(
		payload: ToMillauMessagePayload,
		fee: Balance,
	) -> Result<PostDispatchInfo, DispatchErrorWithPostInfo<PostDispatchInfo>> {
		let call: Call =
			BridgeMessagesCall::<Runtime, Millau>::send_message(MILLAU_PANGOLIN_LANE, payload, fee)
				.into();
		call.dispatch(RawOrigin::Root.into())
	}
}

frame_support::parameter_types! {
	pub const MillauChainId: bp_runtime::ChainId = bp_runtime::MILLAU_CHAIN_ID;
	pub const S2sBackingPalletId: PalletId = PalletId(*b"da/s2sba");
	pub const RingLockLimit: Balance = 10_000_000 * 1_000_000_000;
	pub RootAccountForPayments: Option<AccountId> = Some(to_bytes32(b"root").into());
}

impl darwinia_s2s_backing::Config for Runtime {
	type PalletId = S2sBackingPalletId;
	type Event = Event;
	type WeightInfo = ();
	type RingLockMaxLimit = RingLockLimit;
	type RingCurrency = Ring;

	type BridgedAccountIdConverter = millau_bridge_primitives::AccountIdConverter;
	type BridgedChainId = MillauChainId;

	type OutboundPayload = ToMillauMessagePayload;
	type CallEncoder = MillauCallEncoder;

	type FeeAccount = RootAccountForPayments;
	type MessageSender = ToMillauMessageRelayCaller;
}
