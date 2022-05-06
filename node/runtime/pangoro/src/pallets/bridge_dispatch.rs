pub use pallet_bridge_dispatch::Instance1 as WithPangolinDispatch;

// --- paritytech ---
use frame_support::traits::Everything;
// --- darwinia-network ---
use crate::*;
use bp_messages::{LaneId, MessageNonce};
use pallet_bridge_dispatch::Config;

impl Config<WithPangolinDispatch> for Runtime {
	type AccountIdConverter = bp_pangoro::AccountIdConverter;
	type BridgeMessageId = (LaneId, MessageNonce);
	type Call = Call;
	type CallFilter = Everything;
	type EncodedCall = bm_pangolin::FromPangolinEncodedCall;
	type Event = Event;
	type SourceChainAccountId = bp_pangolin::AccountId;
	type TargetChainAccountPublic = bp_pangoro::AccountPublic;
	type TargetChainSignature = bp_pangoro::Signature;
}
