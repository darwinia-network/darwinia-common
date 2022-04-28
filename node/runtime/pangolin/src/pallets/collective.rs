// --- paritytech ---
pub use pallet_collective::{Instance1 as CouncilCollective, Instance2 as TechnicalCollective};

// --- paritytech ---
use frame_system::EnsureOneOf;
use pallet_collective::{
	Config, EnsureProportionAtLeast, EnsureProportionMoreThan, PrimeDefaultVote,
};
use sp_core::u32_trait::{_1, _2, _3, _5};
// --- darwinia-network ---
use crate::*;

pub type EnsureRootOrHalfCouncil = EnsureOneOf<
	AccountId,
	RootOrigin,
	EnsureProportionAtLeast<_1, _2, AccountId, CouncilCollective>,
>;
pub type EnsureRootOrMoreThanHalfCouncil = EnsureOneOf<
	AccountId,
	RootOrigin,
	EnsureProportionMoreThan<_1, _2, AccountId, CouncilCollective>,
>;
pub type EnsureRootOrHalfTechnicalComittee = EnsureOneOf<
	AccountId,
	RootOrigin,
	EnsureProportionMoreThan<_1, _2, AccountId, TechnicalCollective>,
>;

pub type ApproveOrigin = EnsureOneOf<
	AccountId,
	RootOrigin,
	EnsureProportionAtLeast<_3, _5, AccountId, CouncilCollective>,
>;

frame_support::parameter_types! {
	pub const CouncilMotionDuration: BlockNumber = 3 * DAYS;
	pub const CouncilMaxProposals: u32 = 100;
	pub const CouncilMaxMembers: u32 = 100;
	pub const TechnicalMotionDuration: BlockNumber = 3 * DAYS;
	pub const TechnicalMaxProposals: u32 = 100;
	pub const TechnicalMaxMembers: u32 = 100;
}

// Make sure that there are no more than `MaxMembers` members elected via elections-phragmen.
static_assertions::const_assert!(DesiredMembers::get() <= CouncilMaxMembers::get());

impl Config<CouncilCollective> for Runtime {
	type DefaultVote = PrimeDefaultVote;
	type Event = Event;
	type MaxMembers = CouncilMaxMembers;
	type MaxProposals = CouncilMaxProposals;
	type MotionDuration = CouncilMotionDuration;
	type Origin = Origin;
	type Proposal = Call;
	type WeightInfo = ();
}
impl Config<TechnicalCollective> for Runtime {
	type DefaultVote = PrimeDefaultVote;
	type Event = Event;
	type MaxMembers = TechnicalMaxMembers;
	type MaxProposals = TechnicalMaxProposals;
	type MotionDuration = TechnicalMotionDuration;
	type Origin = Origin;
	type Proposal = Call;
	type WeightInfo = ();
}
