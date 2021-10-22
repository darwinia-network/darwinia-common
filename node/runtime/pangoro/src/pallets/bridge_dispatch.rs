pub use pallet_bridge_dispatch::Instance1 as WithPangolinDispatch;

// --- paritytech ---
use bp_messages::{LaneId, MessageNonce};
use pallet_bridge_dispatch::Config;
use sp_runtime::{MultiSignature, MultiSigner};
// --- darwinia-network ---
use crate::*;
use bridge_primitives::AccountIdConverter;
use pangolin_messages::FromPangolinEncodedCall;

pub struct Sub2SubFilter;
impl frame_support::traits::Contains<Call> for Sub2SubFilter {
	fn contains(call: &Call) -> bool {
		matches!(
			*call,
			Call::Substrate2SubstrateBacking(to_substrate_backing::Call::unlock_from_remote(..))
		)
	}
}

impl Config<WithPangolinDispatch> for Runtime {
	type Event = Event;
	type BridgeMessageId = (LaneId, MessageNonce);
	type Call = Call;
	type CallFilter = Sub2SubFilter;
	type EncodedCall = FromPangolinEncodedCall;
	type SourceChainAccountId = pangolin_primitives::AccountId;
	type TargetChainAccountPublic = MultiSigner;
	type TargetChainSignature = MultiSignature;
	type AccountIdConverter = AccountIdConverter;
}
