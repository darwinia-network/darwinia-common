pub use pallet_bridge_dispatch::{
	Instance1 as WithPangoroDispatch, Instance2 as WithPangolinParachainDispatch,
};

// --- paritytech ---
use bp_messages::{LaneId, MessageNonce};
use bp_pangolin::AccountIdConverter;
use frame_support::traits::Contains;
use pallet_bridge_dispatch::Config;
// --- darwinia-network ---
use crate::{
	pangolin_parachain_messages::FromPangolinParachainEncodedCall,
	pangoro_messages::FromPangoroEncodedCall, *,
};
use pangoro_primitives::AccountId;

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
	type EncodedCall = FromPangoroEncodedCall;
	type SourceChainAccountId = AccountId;
	type TargetChainAccountPublic = AccountPublic;
	type TargetChainSignature = Signature;
	type AccountIdConverter = AccountIdConverter;
}

impl Config<WithPangolinParachainDispatch> for Runtime {
	type Event = Event;
	type BridgeMessageId = (LaneId, MessageNonce);
	type Call = Call;

	/// todo
	type CallFilter = frame_support::traits::Everything;
	type EncodedCall = FromPangolinParachainEncodedCall;
	type SourceChainAccountId = bp_pangolin_parachain::AccountId;
	type TargetChainAccountPublic = AccountPublic;
	type TargetChainSignature = Signature;
	type AccountIdConverter = AccountIdConverter;
}
