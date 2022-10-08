pub use pallet_membership::Instance1 as TechnicalMembershipInstance;

// --- paritytech ---
use pallet_membership::Config;
// --- darwinia-network ---
use crate::{weights::pallet_membership::WeightInfo, *};

impl Config<TechnicalMembershipInstance> for Runtime {
	type AddOrigin = RootOrMoreThanHalf<CouncilCollective>;
	type Event = Event;
	type MaxMembers = TechnicalMaxMembers;
	type MembershipChanged = TechnicalCommittee;
	type MembershipInitialized = TechnicalCommittee;
	type PrimeOrigin = RootOrMoreThanHalf<CouncilCollective>;
	type RemoveOrigin = RootOrMoreThanHalf<CouncilCollective>;
	type ResetOrigin = RootOrMoreThanHalf<CouncilCollective>;
	type SwapOrigin = RootOrMoreThanHalf<CouncilCollective>;
	type WeightInfo = WeightInfo<Self>;
}
