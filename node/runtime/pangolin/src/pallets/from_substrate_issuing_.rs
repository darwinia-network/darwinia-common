// --- paritytech ---
use bp_runtime::{messages::DispatchFeePayment, ChainId};
use frame_support::{dispatch::Dispatchable, weights::PostDispatchInfo, PalletId};
use frame_system::RawOrigin;
use pallet_bridge_messages::Instance1 as Pangoro;
use sp_runtime::{AccountId32, DispatchErrorWithPostInfo};
// --- darwinia-network ---
use crate::*;
use bridge_primitives::{AccountIdConverter, PANGORO_CHAIN_ID, PANGORO_PANGOLIN_LANE};
use darwinia_support::s2s::{
	nonce_to_message_id, TokenMessageId, RelayMessageCaller, ToEthAddress,
};
use dp_asset::{token::Token, RecipientAccount};
use from_substrate_issuing::{Config, EncodeCall};

// remote chain pangoro's dispatch info
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub enum PangoroRuntime {
	/// s2s bridge backing pallet.
	/// this index must be the same as the backing pallet in pangoro runtime
	#[codec(index = 20)]
	Sub2SubBacking(PangoroSub2SubBackingCall),
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[allow(non_camel_case_types)]
pub enum PangoroSub2SubBackingCall {
	#[codec(index = 2)]
	unlock_from_remote(Token, AccountId),
}

pub struct ToPangoroMessageRelayCaller;
impl RelayMessageCaller<ToPangoroMessagePayload, Balance> for ToPangoroMessageRelayCaller {
	fn send_message(
		payload: ToPangoroMessagePayload,
		fee: Balance,
	) -> Result<PostDispatchInfo, DispatchErrorWithPostInfo<PostDispatchInfo>> {
		let call: Call = BridgeMessagesCall::<Runtime, Pangoro>::send_message(
			PANGORO_PANGOLIN_LANE,
			payload,
			fee,
		)
		.into();
		call.dispatch(RawOrigin::Root.into())
	}

	fn latest_token_message_id() -> TokenMessageId {
		let nonce: u64 =
			BridgePangoroMessages::outbound_latest_generated_nonce(PANGORO_PANGOLIN_LANE).into();
		nonce_to_message_id(&PANGORO_PANGOLIN_LANE, nonce)
	}
}

pub struct PangoroCallEncoder;
impl EncodeCall<AccountId, ToPangoroMessagePayload> for PangoroCallEncoder {
	fn encode_remote_unlock(
		spec_version: u32,
		weight: u64,
		token: Token,
		recipient: RecipientAccount<AccountId>,
	) -> Result<ToPangoroMessagePayload, ()> {
		match recipient {
			RecipientAccount::<AccountId>::DarwiniaAccount(r) => {
				let call = PangoroRuntime::Sub2SubBacking(
					PangoroSub2SubBackingCall::unlock_from_remote(token, r),
				)
				.encode();
				return Ok(ToPangoroMessagePayload {
					spec_version,
					weight,
					origin: bp_message_dispatch::CallOrigin::SourceRoot,
					call,
					dispatch_fee_payment: DispatchFeePayment::AtSourceChain,
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
	pub const PangoroChainId: ChainId = PANGORO_CHAIN_ID;
}

impl Config for Runtime {
	type PalletId = S2sIssuingPalletId;
	type Event = Event;
	type WeightInfo = ();
	type RingCurrency = Ring;
	type FeeAccount = RootAccountForPayments;
	type ReceiverAccountId = AccountId;
	type BridgedAccountIdConverter = AccountIdConverter;
	type BridgedChainId = PangoroChainId;
	type ToEthAddressT = TruncateToEthAddress;
	type OutboundPayload = ToPangoroMessagePayload;
	type CallEncoder = PangoroCallEncoder;
	type MessageSender = ToPangoroMessageRelayCaller;
	type InternalTransactHandler = Ethereum;
}
