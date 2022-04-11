pub use pallet_bridge_dispatch::{
	Instance1 as WithPangoroDispatch, Instance2 as WithPangolinParachainDispatch,
};

// --- paritytech ---
use frame_support::traits::{Contains, Everything};
// --- darwinia-network ---
use crate::{
	bridges_message::{bm_pangolin_parachain, bm_pangoro},
	*,
};
use bp_messages::{LaneId, MessageNonce};
use pallet_bridge_dispatch::Config;

pub struct S2sCallFilter;
impl Contains<Call> for S2sCallFilter {
	fn contains(c: &Call) -> bool {
		matches!(
			c,
			Call::Substrate2SubstrateIssuing(
				from_substrate_issuing::Call::register_from_remote { .. }
			) | Call::Substrate2SubstrateIssuing(
				from_substrate_issuing::Call::issue_from_remote { .. }
			)
		)
	}
}

impl Config<WithPangoroDispatch> for Runtime {
	type Event = Event;
	type BridgeMessageId = (LaneId, MessageNonce);
	type Call = Call;
	type CallFilter = S2sCallFilter;
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
