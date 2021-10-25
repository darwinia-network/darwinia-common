// --- paritytech ---
use bp_runtime::{messages::DispatchFeePayment, ChainId};
use frame_support::PalletId;
use sp_runtime::AccountId32;
// --- darwinia-network ---
use crate::*;
use bridge_primitives::{
	call::{CallParams, EncodeRuntimeCall, RuntimeCall},
	AccountIdConverter, PANGORO_CHAIN_ID,
};
use darwinia_support::{s2s::ToEthAddress, to_bytes32, ChainName};
use dp_asset::token::Token;
use dp_contract::mapping_token_factory::s2s::S2sRemoteUnlockInfo;
use from_substrate_issuing::{Config, EncodeCall};
pub struct PangoroCallEncoder;
impl EncodeCall<AccountId, ToPangoroMessagePayload> for PangoroCallEncoder {
	fn encode_remote_unlock(
		submitter: AccountId,
		remote_unlock_info: S2sRemoteUnlockInfo,
	) -> Result<ToPangoroMessagePayload, ()> {
		let call = RuntimeCall::encode_call(
			0,
			0,
			CallParams::UnlockFromRemote(submitter.clone(), remote_unlock_info.clone()),
		)?;
		return Ok(ToPangoroMessagePayload {
			spec_version: remote_unlock_info.spec_version,
			weight: remote_unlock_info.weight,
			origin: bp_message_dispatch::CallOrigin::SourceAccount(submitter),
			call,
			dispatch_fee_payment: DispatchFeePayment::AtSourceChain,
		});
		// }
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
