// --- substrate ---
use frame_support::traits::ChangeMembers;
use pallet_membership::{weights::SubstrateWeight, Config, Instance1};
// --- darwinia ---
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
impl Config<Instance1> for Runtime {
	type Event = Event;
	type AddOrigin = EnsureRootOrMoreThanHalfCouncil;
	type RemoveOrigin = EnsureRootOrMoreThanHalfCouncil;
	type SwapOrigin = EnsureRootOrMoreThanHalfCouncil;
	type ResetOrigin = EnsureRootOrMoreThanHalfCouncil;
	type PrimeOrigin = EnsureRootOrMoreThanHalfCouncil;
	type MembershipInitialized = TechnicalCommittee;
	type MembershipChanged = MembershipChangedGroup;
	type MaxMembers = TechnicalMaxMembers;
	type WeightInfo = SubstrateWeight<Runtime>;
}
