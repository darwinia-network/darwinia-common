// --- crates.io ---
use codec::Decode;
// --- paritytech ---
use frame_support::PalletId;
use sp_runtime::AccountId32;
// --- darwinia-network ---
use crate::{weights::from_substrate_issuing::WeightInfo, *};
use bp_message_dispatch::CallOrigin;
use bp_messages::{LaneId, MessageNonce};
use bp_runtime::{ChainId, PANGORO_CHAIN_ID};
use bridge_runtime_common::lanes::PANGORO_PANGOLIN_LANE;
use darwinia_support::{s2s::OutboundMessenger, ChainName};
use from_substrate_issuing::Config;

pub struct OutboundMessageDataInfo;
impl OutboundMessenger<AccountId32> for OutboundMessageDataInfo {
	fn check_lane_id(lane_id: &LaneId) -> bool {
		*lane_id == PANGORO_PANGOLIN_LANE
	}

	fn get_valid_message_sender(nonce: MessageNonce) -> Result<AccountId32, &'static str> {
		let data = BridgePangoroMessages::outbound_message_data(PANGORO_PANGOLIN_LANE, nonce)
			.ok_or("Invalid outbound message data")?;
		let payload = bm_pangoro::ToPangoroMessagePayload::decode(&mut &data.payload[..])
			.map_err(|_| "decode message payload failed")?;
		match payload.origin {
			CallOrigin::SourceAccount(account_id) => Ok(account_id),
			_ => Err("Invalid Account Type"),
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
	type OutboundMessenger = OutboundMessageDataInfo;
	type PalletId = S2sIssuingPalletId;
	type RingCurrency = Ring;
	type WeightInfo = WeightInfo<Runtime>;
}
