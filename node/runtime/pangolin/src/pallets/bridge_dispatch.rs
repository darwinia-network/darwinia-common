pub use pallet_bridge_dispatch::{
	Instance1 as WithPangoroDispatch, Instance2 as WithPangolinParachainDispatch,
};

// --- paritytech ---
use frame_support::traits::Everything;
// --- darwinia-network ---
use crate::*;
use bp_messages::{LaneId, MessageNonce};
use pallet_bridge_dispatch::Config;

impl Config<WithPangoroDispatch> for Runtime {
	type Event = Event;
	type BridgeMessageId = (LaneId, MessageNonce);
	type Call = Call;
	type CallFilter = Everything;
	type EncodedCall = bm_pangoro::FromPangoroEncodedCall;
	type SourceChainAccountId = bp_pangoro::AccountId;
	type TargetChainAccountPublic = AccountPublic;
	type TargetChainSignature = Signature;
	type AccountIdConverter = bp_pangolin::AccountIdConverter;
}
impl Config<WithPangolinParachainDispatch> for Runtime {
	type Event = Event;
	type BridgeMessageId = (LaneId, MessageNonce);
	type Call = Call;
	type CallFilter = Everything;
	type EncodedCall = bm_pangolin_parachain::FromPangolinParachainEncodedCall;
	type SourceChainAccountId = bp_pangolin_parachain::AccountId;
	type TargetChainAccountPublic = AccountPublic;
	type TargetChainSignature = Signature;
	type AccountIdConverter = bp_pangolin::AccountIdConverter;
}
