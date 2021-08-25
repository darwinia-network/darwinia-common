pub use pallet_bridge_dispatch::Instance1 as WithPangoroDispatch;

// --- substrate ---
use bp_messages::{LaneId, MessageNonce};
use pallet_bridge_dispatch::Config;
use pangoro_primitives::AccountId;
use sp_runtime::{MultiSignature, MultiSigner};
// --- darwinia ---
use crate::{pangoro_messages::FromPangoroEncodedCall, *};
use bridge_primitives::AccountIdConverter;

impl Config<WithPangoroDispatch> for Runtime {
	type Event = Event;
	type MessageId = (LaneId, MessageNonce);
	type Call = Call;
	type CallFilter = ();
	type EncodedCall = FromPangoroEncodedCall;
	type SourceChainAccountId = AccountId;
	type TargetChainAccountPublic = MultiSigner;
	type TargetChainSignature = MultiSignature;
	type AccountIdConverter = AccountIdConverter;
}
