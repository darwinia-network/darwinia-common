use codec::Decode;
// --- paritytech ---
use frame_support::PalletId;
use sp_runtime::AccountId32;
// --- darwinia-network ---
use crate::*;
use bp_message_dispatch::CallOrigin;
use bp_messages::{LaneId, MessageNonce};
use bp_runtime::{ChainId, PANGORO_CHAIN_ID};
use bridge_runtime_common::lanes::PANGORO_PANGOLIN_LANE;
use darwinia_support::{
	s2s::{OutboundMessager, ToEthAddress},
	ChainName,
};
use from_substrate_issuing::Config;

// Convert from AccountId32 to H160
pub struct TruncateToEthAddress;
impl ToEthAddress<AccountId32> for TruncateToEthAddress {
	fn into_ethereum_id(address: &AccountId32) -> H160 {
		let account20: &[u8] = &address.as_ref();

		H160::from_slice(&account20[..20])
	}
}

pub struct OutboundMessageDataInfo;
impl OutboundMessager<AccountId32> for OutboundMessageDataInfo {
	fn check_lane_id(lane_id: &LaneId) -> bool {
		return *lane_id == PANGORO_PANGOLIN_LANE;
	}

	fn get_valid_message_sender(nonce: MessageNonce) -> Result<AccountId32, &'static str> {
		let data = BridgePangoroMessages::outbound_message_data(PANGORO_PANGOLIN_LANE, nonce)
			.ok_or_else(|| "Invalid outbound message data")?;
		let payload = bm_pangoro::ToPangoroMessagePayload::decode(&mut &data.payload[..])
			.map_err(|_| "decode message payload failed")?;
		match payload.origin {
			CallOrigin::SourceAccount(account_id) => Ok(account_id),
			_ => Err("Invalid Account Type".into()),
		}
	}
}

frame_support::parameter_types! {
	pub const S2sIssuingPalletId: PalletId = PalletId(*b"da/s2sis");
	pub const PangoroChainId: ChainId = PANGORO_CHAIN_ID;
	pub BackingChainName: ChainName = (b"Pangoro").to_vec();
}

impl Config for Runtime {
	type BackingChainName = BackingChainName;
	type BridgedAccountIdConverter = bp_pangolin::AccountIdConverter;
	type BridgedChainId = PangoroChainId;
	type Event = Event;
	type InternalTransactHandler = Ethereum;
	type OutboundMessager = OutboundMessageDataInfo;
	type PalletId = S2sIssuingPalletId;
	type RingCurrency = Ring;
	type ToEthAddressT = TruncateToEthAddress;
	type WeightInfo = ();
}
