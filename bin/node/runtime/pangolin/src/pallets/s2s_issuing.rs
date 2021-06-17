// --- substrate ---
use bp_millau::AccountIdConverter;
use frame_support::{dispatch::Dispatchable, weights::PostDispatchInfo, PalletId};
use frame_system::RawOrigin;
use pallet_bridge_messages::Instance1 as Millau;
use sp_runtime::DispatchErrorWithPostInfo;
// --- darwinia ---
use crate::*;
use darwinia_s2s_issuing::Config;
use darwinia_support::s2s::{RelayMessageCaller, TruncateToEthAddress};
use dp_asset::BridgedAssetReceiver;

pub struct ToMillauMessageRelayCaller;
impl RelayMessageCaller<ToMillauMessagePayload, AccountId> for ToMillauMessageRelayCaller {
	fn send_message(
		payload: ToMillauMessagePayload,
		account: AccountId,
	) -> Result<PostDispatchInfo, DispatchErrorWithPostInfo<PostDispatchInfo>> {
		let call: Call =
			BridgeMessagesCall::<Runtime, Millau>::send_message([0; 4], payload, 0u128.into())
				.into();
		call.dispatch(RawOrigin::Signed(account).into())
	}
}

pub struct MillauBackingUnlockAsset;
impl BridgedAssetReceiver<RelayAccount<AccountId>> for MillauBackingUnlockAsset {
	fn encode_call(token: Token, recipient: RelayAccount<AccountId>) -> Result<Vec<u8>, ()> {
		match recipient {
			RelayAccount::<AccountId>::DarwiniaAccount(r) => {
				return Ok(MillauRuntime::Sub2SubBacking(
					MillauSub2SubBackingCall::remote_burn_and_unlock(token, r),
				)
				.encode())
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
	type ReceiverAccountId = AccountId;
	type BridgedAccountIdConverter = AccountIdConverter;
	type BridgedChainId = MillauChainId;
	type ToEthAddressT = TruncateToEthAddress;
	type RemoteUnlockCall = MillauBackingUnlockAsset;
	type OutboundPayload = ToMillauMessagePayload;
	type CallToPayload = MillauCallToPayload;
	type MessageSender = ToMillauMessageRelayCaller;
}
