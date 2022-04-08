pub use pallet_bridge_dispatch::Instance1 as WithPangolinDispatch;

// --- paritytech ---
use frame_support::traits::Contains;
// --- darwinia-network ---
use crate::{bridges_message::pangolin::FromPangolinEncodedCall, *};
use bp_messages::{LaneId, MessageNonce};
use pallet_bridge_dispatch::Config;

pub struct S2sCallFilter;
impl Contains<Call> for S2sCallFilter {
	fn contains(c: &Call) -> bool {
		matches!(
			c,
			Call::Substrate2SubstrateBacking(to_substrate_backing::Call::unlock_from_remote { .. })
		)
	}
}

impl Config<WithPangolinDispatch> for Runtime {
	type Event = Event;
	type BridgeMessageId = (LaneId, MessageNonce);
	type Call = Call;
	type CallFilter = S2sCallFilter;
	type EncodedCall = FromPangolinEncodedCall;
	type SourceChainAccountId = bp_pangolin::AccountId;
	type TargetChainAccountPublic = AccountPublic;
	type TargetChainSignature = Signature;
	type AccountIdConverter = bp_pangolin::AccountIdConverter;
}
