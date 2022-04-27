pub use pallet_membership::Instance1 as TechnicalMembershipInstance;

// --- paritytech ---
use frame_support::traits::ChangeMembers;
use pallet_membership::Config;
// --- darwinia-network ---
use crate::*;

pub struct MembershipChangedGroup;
impl ChangeMembers<AccountId> for MembershipChangedGroup {
	fn change_members_sorted(
		incoming: &[AccountId],
		outgoing: &[AccountId],
		sorted_new: &[AccountId],
	) {
		TechnicalCommittee::change_members_sorted(incoming, outgoing, sorted_new);
		EthereumRelay::change_members_sorted(incoming, outgoing, sorted_new);
	}
}

impl Config<TechnicalMembershipInstance> for Runtime {
	type AddOrigin = EnsureRootOrMoreThanHalfCouncil;
	type Event = Event;
	type MaxMembers = TechnicalMaxMembers;
	type MembershipChanged = MembershipChangedGroup;
	type MembershipInitialized = TechnicalCommittee;
	type PrimeOrigin = EnsureRootOrMoreThanHalfCouncil;
	type RemoveOrigin = EnsureRootOrMoreThanHalfCouncil;
	type ResetOrigin = EnsureRootOrMoreThanHalfCouncil;
	type SwapOrigin = EnsureRootOrMoreThanHalfCouncil;
	type WeightInfo = ();
}
