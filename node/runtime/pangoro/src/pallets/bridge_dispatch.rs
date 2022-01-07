pub use pallet_bridge_dispatch::Instance1 as WithPangolinDispatch;

// --- paritytech ---
use bp_messages::{LaneId, MessageNonce};
use frame_support::traits::Contains;
use pallet_bridge_dispatch::Config;
// --- darwinia-network ---
use crate::*;
use drml_bridge_primitives::AccountIdConverter;
use pangolin_messages::FromPangolinEncodedCall;

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
	type SourceChainAccountId = pangolin_primitives::AccountId;
	type TargetChainAccountPublic = AccountPublic;
	type TargetChainSignature = Signature;
	type AccountIdConverter = AccountIdConverter;
}
