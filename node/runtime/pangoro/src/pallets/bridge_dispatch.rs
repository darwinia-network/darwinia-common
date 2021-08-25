pub use pallet_bridge_dispatch::Instance1 as WithPangolinDispatch;

// --- paritytech ---
use bp_messages::{LaneId, MessageNonce};
use pallet_bridge_dispatch::Config;
// --- darwinia-network ---
use crate::*;
use bridge_primitives::AccountIdConverter;
use pangolin_messages::FromPangolinEncodedCall;

impl Config<WithPangolinDispatch> for Runtime {
	type Event = Event;
	type MessageId = (LaneId, MessageNonce);
	type Call = Call;
	type CallFilter = ();
	type EncodedCall = FromPangolinEncodedCall;
	type SourceChainAccountId = pangolin_primitives::AccountId;
	type TargetChainAccountPublic = MultiSigner;
	type TargetChainSignature = MultiSignature;
	type AccountIdConverter = AccountIdConverter;
}
