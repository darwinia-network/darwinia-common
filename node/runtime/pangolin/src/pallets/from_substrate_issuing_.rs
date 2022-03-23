// --- paritytech ---
use bp_messages::LaneId;
use bp_pangolin::AccountIdConverter;
use bp_runtime::{ChainId, PANGORO_CHAIN_ID};
use frame_support::PalletId;
use sp_runtime::AccountId32;
// --- darwinia-network ---
use crate::{pangoro_messages::ToPangoroOutboundPayLoad, *};
use darwinia_support::{s2s::ToEthAddress, ChainName};
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
	pub const BridgePangoroLaneId: LaneId = PANGORO_PANGOLIN_LANE;
	pub BackingChainName: ChainName = (b"Pangoro").to_vec();
}

impl Config for Runtime {
	type PalletId = S2sIssuingPalletId;
	type Event = Event;
	type WeightInfo = ();
	type RingCurrency = Ring;
	type BridgedAccountIdConverter = AccountIdConverter;
	type BridgedChainId = PangoroChainId;
	type ToEthAddressT = TruncateToEthAddress;
	type OutboundPayloadCreator = ToPangoroOutboundPayLoad;
	type InternalTransactHandler = Ethereum;
	type BackingChainName = BackingChainName;
	type MessageLaneId = BridgePangoroLaneId;
}
