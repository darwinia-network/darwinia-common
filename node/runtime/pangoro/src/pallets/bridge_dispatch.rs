use crate::*;

pub type WithPangolinDispatch = pallet_bridge_dispatch::Instance1;
impl pallet_bridge_dispatch::Config<WithPangolinDispatch> for Runtime {
	type Event = Event;
	type MessageId = (bp_messages::LaneId, bp_messages::MessageNonce);
	type Call = Call;
	type CallFilter = ();
	type EncodedCall = pangolin_messages::FromPangolinEncodedCall;
	type SourceChainAccountId = pangolin_primitives::AccountId;
	type TargetChainAccountPublic = MultiSigner;
	type TargetChainSignature = MultiSignature;
	type AccountIdConverter = bridge_primitives::AccountIdConverter;
}
