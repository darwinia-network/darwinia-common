// --- paritytech ---
use bp_messages::LaneId;
use bp_runtime::{messages::DispatchFeePayment, ChainId};
use frame_support::PalletId;
use sp_runtime::AccountId32;
// --- darwinia-network ---
use crate::{
	pangoro_messages::{ToPangoroMessagePayloadBox, PANGORO_S2S_BACKING_PALLET_INDEX},
	*,
};
use bridge_primitives::{AccountIdConverter, PANGORO_CHAIN_ID, PANGORO_PANGOLIN_LANE};
use darwinia_support::{s2s::ToEthAddress, ChainName};
use dp_s2s::{CallParams, PayloadCreate};
use from_substrate_issuing::Config;
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
	type OutboundPayload = ToPangoroMessagePayloadBox;
	type InternalTransactHandler = Ethereum;
	type BackingChainName = PangoroName;
	type MessageLaneId = BridgePangoroLaneId;
}
