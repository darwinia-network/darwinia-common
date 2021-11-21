pub use pallet_bridge_dispatch::Instance1 as WithPangoroDispatch;

// --- paritytech ---
use bp_messages::{LaneId, MessageNonce};
use pallet_bridge_dispatch::Config;
use pangoro_primitives::AccountId;
use sp_runtime::{MultiSignature, MultiSigner};
// --- darwinia-network ---
use crate::{pangoro_messages::FromPangoroEncodedCall, *};
use bridge_primitives::AccountIdConverter;

pub struct Sub2SubFilter;
impl frame_support::traits::Contains<Call> for Sub2SubFilter {
	fn contains(call: &Call) -> bool {
		matches!(
			*call,
			Call::Substrate2SubstrateIssuing(from_substrate_issuing::Call::register_from_remote(
				..
			))
		) || matches!(
			*call,
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
	type TargetChainAccountPublic = MultiSigner;
	type TargetChainSignature = MultiSignature;
	type AccountIdConverter = AccountIdConverter;
}
