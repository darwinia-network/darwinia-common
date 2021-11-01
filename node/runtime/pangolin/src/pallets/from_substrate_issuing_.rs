// --- paritytech ---
use bp_runtime::{messages::DispatchFeePayment, ChainId};
use frame_support::PalletId;
use sp_runtime::AccountId32;
// --- darwinia-network ---
use crate::*;
use bridge_primitives::{AccountIdConverter, PANGORO_CHAIN_ID};
use darwinia_support::{s2s::ToEthAddress, to_bytes32, ChainName};
use dp_s2s::{CallParams, PayloadCreate};
use from_substrate_issuing::Config;
use to_substrate_backing::S2SBackingCall;

/// Pangoro chain's dispatch call info
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub enum PangoroRuntime {
	/// NOTE: The index must be the same as the backing pallet in the pangoro runtime
	#[codec(index = 20)]
	Sub2SubBacking(S2SBackingCall<AccountId>),
}

/// Create message payload according to the call parameters.
pub struct PangoroPayloadCreator;
impl PayloadCreate<AccountId, ToPangoroMessagePayload> for PangoroPayloadCreator {
	fn payload(
		spec_version: u32,
		weight: u64,
		call_params: CallParams<AccountId>,
	) -> Result<ToPangoroMessagePayload, ()> {
		let (submitter, call) = match call_params {
			CallParams::UnlockFromRemote(submitter, unlock_info) => {
				if unlock_info.recipient.len() != 32 {
					return Err(());
				}

				let recipient_id: AccountId = to_bytes32(unlock_info.recipient.as_slice()).into();
				(
					submitter,
					PangoroRuntime::Sub2SubBacking(S2SBackingCall::unlock_from_remote(
						unlock_info.token,
						recipient_id,
					))
					.encode(),
				)
			}
			_ => return Err(()),
		};
		Ok(ToPangoroMessagePayload {
			spec_version,
			weight,
			origin: bp_message_dispatch::CallOrigin::SourceAccount(submitter),
			call,
			dispatch_fee_payment: DispatchFeePayment::AtSourceChain,
		})
	}
}

// Convert from AccountId32 to H160
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
	type PayloadCreator = PangoroPayloadCreator;
	type InternalTransactHandler = Ethereum;
	type BackingChainName = PangoroName;
}
