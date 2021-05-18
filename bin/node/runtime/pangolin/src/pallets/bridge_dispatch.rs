pub use pallet_bridge_dispatch::Instance1 as WithMillauDispatch;

// --- substrate ---
use bp_messages::{LaneId, MessageNonce};
use bp_millau::AccountId;
use pallet_bridge_dispatch::Config;
use sp_runtime::{MultiSignature, MultiSigner};
// --- darwinia ---
use crate::{millau_messages::FromMillauEncodedCall, *};
use pangolin_bridge_primitives::AccountIdConverter;

impl Config<WithMillauDispatch> for Runtime {
	type Event = Event;
	type MessageId = (LaneId, MessageNonce);
	type Call = Call;
	type CallFilter = ();
	type EncodedCall = FromMillauEncodedCall;
	type SourceChainAccountId = AccountId;
	type TargetChainAccountPublic = MultiSigner;
	type TargetChainSignature = MultiSignature;
	type AccountIdConverter = AccountIdConverter;
}
