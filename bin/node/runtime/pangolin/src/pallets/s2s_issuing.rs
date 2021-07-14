// --- substrate ---
use frame_support::{dispatch::Dispatchable, weights::PostDispatchInfo, PalletId};
use frame_system::RawOrigin;
use pallet_bridge_messages::Instance1 as Millau;
use sp_runtime::{AccountId32, DispatchErrorWithPostInfo};
// --- darwinia ---
use crate::*;
use darwinia_s2s_issuing::{Config, EncodeCall};
use darwinia_support::s2s::{RelayMessageCaller, ToEthAddress};
use dp_asset::{token::Token, RecipientAccount};
use millau_primitives::AccountIdConverter;

// 0x6d74706c
pub const MILLAU_PANGOLIN_LANE: [u8; 4] = *b"mtpl";

// remote chain millau's dispatch info
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub enum MillauRuntime {
	/// s2s bridge backing pallet.
	/// this index must be the same as the backing pallet in millau runtime
	#[codec(index = 14)]
	Sub2SubBacking(MillauSub2SubBackingCall),
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[allow(non_camel_case_types)]
pub enum MillauSub2SubBackingCall {
	#[codec(index = 2)]
	unlock_from_remote(Token, AccountId),
}

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

pub struct MillauCallEncoder;
impl EncodeCall<AccountId, ToMillauMessagePayload> for MillauCallEncoder {
	fn encode_remote_unlock(
		spec_version: u32,
		weight: u64,
		token: Token,
		recipient: RecipientAccount<AccountId>,
	) -> Result<ToMillauMessagePayload, ()> {
		match recipient {
			RecipientAccount::<AccountId>::DarwiniaAccount(r) => {
				let call = MillauRuntime::Sub2SubBacking(
					MillauSub2SubBackingCall::unlock_from_remote(token, r),
				)
				.encode();
				return Ok(ToMillauMessagePayload {
					spec_version,
					weight,
					origin: bp_message_dispatch::CallOrigin::SourceRoot,
					call,
				});
			}
			_ => Err(()),
		}
	}
}

pub struct TruncateToEthAddress;
impl ToEthAddress<AccountId32> for TruncateToEthAddress {
	fn into_ethereum_id(address: &AccountId32) -> H160 {
		let account20: &[u8] = &address.as_ref();
		H160::from_slice(&account20[..20])
	}
}

frame_support::parameter_types! {
	pub const S2sIssuingPalletId: PalletId = PalletId(*b"da/s2sis");
	pub const MillauChainId: bp_runtime::ChainId = bp_runtime::MILLAU_CHAIN_ID;
}

impl Config for Runtime {
	type PalletId = S2sIssuingPalletId;
	type Event = Event;
	type WeightInfo = ();
	type RingCurrency = Ring;
	type FeeAccount = RootAccountForPayments;
	type ReceiverAccountId = AccountId;
	type BridgedAccountIdConverter = AccountIdConverter;
	type BridgedChainId = MillauChainId;
	type ToEthAddressT = TruncateToEthAddress;
	type OutboundPayload = ToMillauMessagePayload;
	type CallEncoder = MillauCallEncoder;
	type MessageSender = ToMillauMessageRelayCaller;
}
