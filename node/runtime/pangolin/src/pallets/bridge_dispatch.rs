pub use pallet_bridge_dispatch::Instance1 as WithPangoroDispatch;

// --- paritytech ---
use bp_messages::{LaneId, MessageNonce};
use frame_support::traits::Contains;
use pallet_bridge_dispatch::Config;
use pangoro_primitives::AccountId;
// --- darwinia-network ---
use crate::{pangoro_messages::FromPangoroEncodedCall, *};
use bridge_primitives::AccountIdConverter;

pub struct Sub2SubFilter;
impl Contains<Call> for Sub2SubFilter {
	fn contains(c: &Call) -> bool {
		matches!(
			c,
			Call::Substrate2SubstrateIssuing(from_substrate_issuing::Call::register_from_remote(
				..
			))
		) || matches!(
			c,
			Call::Substrate2SubstrateIssuing(from_substrate_issuing::Call::issue_from_remote(..))
		)
	}
}

impl Config<WithPangoroDispatch> for Runtime {
	type Event = Event;
	type BridgeMessageId = (LaneId, MessageNonce);
	type Call = Call;
	type CallFilter = Sub2SubFilter;
	type EncodedCall = FromPangoroEncodedCall;
	type SourceChainAccountId = AccountId;
	type TargetChainAccountPublic = AccountPublic;
	type TargetChainSignature = Signature;
	type AccountIdConverter = AccountIdConverter;
}
