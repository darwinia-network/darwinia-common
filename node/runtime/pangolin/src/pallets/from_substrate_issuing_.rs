// --- paritytech ---
use bp_messages::LaneId;
use bp_runtime::{messages::DispatchFeePayment, ChainId};
use frame_support::PalletId;
use sp_runtime::AccountId32;
// --- darwinia-network ---
use crate::*;
use bridge_primitives::{AccountIdConverter, PANGORO_CHAIN_ID, PANGORO_PANGOLIN_LANE};
use darwinia_support::{s2s::ToEthAddress, ChainName};
use dp_s2s::{CallParams, PayloadCreate};
use from_substrate_issuing::Config;

const PANGORO_S2S_BACKING_PALLET_INDEX: u8 = 20;
/// Create message payload according to the call parameters.
pub struct PangoroPayloadCreator;
impl PayloadCreate<AccountId, ToPangoroMessagePayload> for PangoroPayloadCreator {
	fn payload(
		submitter: AccountId,
		spec_version: u32,
		weight: u64,
		call_params: CallParams,
	) -> Result<ToPangoroMessagePayload, &'static str> {
		let call = Self::encode_call(PANGORO_S2S_BACKING_PALLET_INDEX, call_params)?;
		// let (submitter, call) = match call_params.clone() {
		// 	CallParams::S2sBackingPalletUnlockFromRemote(submitter, _unlock_info) => (
		// 		submitter,
		// 		Self::encode_call(PANGORO_S2S_BACKING_PALLET_INDEX, call_params)?,
		// 	),
		// 	_ => return Err("The call params is mismatched"),
		// };

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
	pub const BridgePangoroLaneId: LaneId = PANGORO_PANGOLIN_LANE;
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
	type MessageLaneId = BridgePangoroLaneId;
}
