// --- paritytech ---
use bp_runtime::{messages::DispatchFeePayment, ChainId};
use frame_support::PalletId;
use sp_runtime::AccountId32;
// --- darwinia-network ---
use crate::*;
use bridge_primitives::{AccountIdConverter, PANGORO_CHAIN_ID};
use darwinia_support::{s2s::ToEthAddress, to_bytes32, ChainName};
use dp_asset::token::Token;
use dp_contract::mapping_token_factory::s2s::S2sRemoteUnlockInfo;
use from_substrate_issuing::{Config, EncodeCall};

/// The bridged chain(Pangoro) dispatch info
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub enum PangoroRuntime {
	/// NOTE: The index must be the same as the backing pallet in the pangoro runtime
	#[codec(index = 20)]
	Sub2SubBacking(PangoroSub2SubBackingCall),
}

/// The backing call in the pangoro s2s backing pallet
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[allow(non_camel_case_types)]
pub enum PangoroSub2SubBackingCall {
	/// NOTE: The index depends on the call order in the s2s backing pallet.
	#[codec(index = 2)]
	unlock_from_remote(Token, AccountId),
}

pub struct PangoroCallEncoder;
impl EncodeCall<AccountId, ToPangoroMessagePayload> for PangoroCallEncoder {
	fn encode_remote_unlock(
		submitter: AccountId,
		remote_unlock_info: S2sRemoteUnlockInfo,
	) -> Result<ToPangoroMessagePayload, ()> {
		if remote_unlock_info.recipient.len() != 32 {
			return Err(());
		} else {
			let recipient_id: AccountId =
				to_bytes32(remote_unlock_info.recipient.as_slice()).into();
			let call =
				PangoroRuntime::Sub2SubBacking(PangoroSub2SubBackingCall::unlock_from_remote(
					remote_unlock_info.token,
					recipient_id,
				))
				.encode();
			return Ok(ToPangoroMessagePayload {
				spec_version: remote_unlock_info.spec_version,
				weight: remote_unlock_info.weight,
				origin: bp_message_dispatch::CallOrigin::SourceAccount(submitter),
				call,
				dispatch_fee_payment: DispatchFeePayment::AtSourceChain,
			});
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
	pub PangoroName: ChainName = (b"Pangoro").to_vec();
}

impl Config for Runtime {
	type PalletId = S2sIssuingPalletId;
	type Event = Event;
	type WeightInfo = ();
	type RingCurrency = Ring;
	type BridgedAccountIdConverter = AccountIdConverter;
	type BridgedChainId = PangoroChainId;
	type ToEthAddressT = TruncateToEthAddress;
	type OutboundPayload = ToPangoroMessagePayload;
	type CallEncoder = PangoroCallEncoder;
	type InternalTransactHandler = Ethereum;
	type BackingChainName = PangoroName;
}
