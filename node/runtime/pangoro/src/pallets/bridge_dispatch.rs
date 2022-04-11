pub use pallet_bridge_dispatch::Instance1 as WithPangolinDispatch;

// --- paritytech ---
use frame_support::traits::Everything;
// --- darwinia-network ---
use crate::{bridges_message::bm_pangolin, *};
use bp_messages::{LaneId, MessageNonce};
use pallet_bridge_dispatch::Config;

impl Config<WithPangolinDispatch> for Runtime {
	type Event = Event;
	type BridgeMessageId = (LaneId, MessageNonce);
	type Call = Call;
	type CallFilter = Everything;
	type EncodedCall = bm_pangolin::FromPangolinEncodedCall;
	type SourceChainAccountId = bp_pangolin::AccountId;
	type TargetChainAccountPublic = AccountPublic;
	type TargetChainSignature = Signature;
	type AccountIdConverter = bp_pangoro::AccountIdConverter;
}
