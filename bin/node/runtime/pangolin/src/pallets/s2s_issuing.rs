// --- substrate ---
use millau_primitives::AccountIdConverter;
use frame_support::{dispatch::Dispatchable, weights::PostDispatchInfo, PalletId};
use frame_system::RawOrigin;
use pallet_bridge_messages::Instance1 as Millau;
use sp_runtime::DispatchErrorWithPostInfo;
// --- darwinia ---
use crate::*;
use darwinia_s2s_issuing::{Config, EncodeCall};
use darwinia_support::s2s::{RelayMessageCaller, TruncateToEthAddress};
use dp_asset::RecipientAccount;

// 0x70746d6c
pub const MILLAU_PANGO_LANE: [u8; 4] = *b"mtpl";

pub struct ToMillauMessageRelayCaller;
impl RelayMessageCaller<ToMillauMessagePayload, Balance> for ToMillauMessageRelayCaller {
	fn send_message(
		payload: ToMillauMessagePayload,
		fee: Balance,
	) -> Result<PostDispatchInfo, DispatchErrorWithPostInfo<PostDispatchInfo>> {
		let call: Call =
			BridgeMessagesCall::<Runtime, Millau>::send_message(MILLAU_PANGO_LANE, payload, fee)
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
				let call = MillauRuntime::Sub2SubBacking(MillauSub2SubBackingCall::remote_unlock(
					token, r,
				))
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
