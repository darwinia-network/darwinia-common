pub use pallet_membership::Instance1 as TechnicalMembershipInstance;

// --- paritytech ---
use frame_support::traits::ChangeMembers;
use pallet_collective::Prime;
use pallet_membership::Config;
// --- darwinia-network ---
use crate::{weights::pallet_membership::WeightInfo, *};

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

	fn set_prime(prime: Option<AccountId>) {
		<Prime<Runtime, TechnicalCollective>>::set(prime);
	}

	fn get_prime() -> Option<AccountId> {
		<Prime<Runtime, TechnicalCollective>>::get()
	}
}

impl Config<TechnicalMembershipInstance> for Runtime {
	type AddOrigin = RootOrMoreThanHalf<CouncilCollective>;
	type Event = Event;
	type MaxMembers = TechnicalMaxMembers;
	type MembershipChanged = MembershipChangedGroup;
	type MembershipInitialized = TechnicalCommittee;
	type PrimeOrigin = RootOrMoreThanHalf<CouncilCollective>;
	type RemoveOrigin = RootOrMoreThanHalf<CouncilCollective>;
	type ResetOrigin = RootOrMoreThanHalf<CouncilCollective>;
	type SwapOrigin = RootOrMoreThanHalf<CouncilCollective>;
	type WeightInfo = WeightInfo<Runtime>;
}
