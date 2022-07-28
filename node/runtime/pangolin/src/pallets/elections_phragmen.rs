// --- paritytech ---
use frame_support::traits::{LockIdentifier, U128CurrencyToVote};
use pallet_elections_phragmen::Config;
// --- darwinia-network ---
use crate::{weights::pallet_elections_phragmen::WeightInfo, *};

#[cfg(feature = "runtime-benchmarks")]
frame_support::parameter_types! {
	pub const CandidacyBond: Balance = 1;
	// 1 storage item created, key size is 32 bytes, value size is 16+16.
	pub const VotingBondBase: Balance = 1;
	// additional data per vote is 32 bytes (account id).
	pub const VotingBondFactor: Balance = 1;
}
#[cfg(not(feature = "runtime-benchmarks"))]
frame_support::parameter_types! {
	pub const CandidacyBond: Balance = 1 * COIN;
	// 1 storage item created, key size is 32 bytes, value size is 16+16.
	pub const VotingBondBase: Balance = pangolin_deposit(1, 64);
	// additional data per vote is 32 bytes (account id).
	pub const VotingBondFactor: Balance = pangolin_deposit(0, 32);
}
frame_support::parameter_types! {
	pub const PhragmenElectionPalletId: LockIdentifier = *b"phrelect";
	pub const DesiredMembers: u32 = 7;
	pub const DesiredRunnersUp: u32 = 7;
	/// Daily council elections.
	pub const TermDuration: BlockNumber = 24 * HOURS;
}

impl Config for Runtime {
	type CandidacyBond = CandidacyBond;
	type ChangeMembers = Council;
	type Currency = Ring;
	type CurrencyToVote = U128CurrencyToVote;
	type DesiredMembers = DesiredMembers;
	type DesiredRunnersUp = DesiredRunnersUp;
	type Event = Event;
	// NOTE: this implies that council's genesis members cannot be set directly and must come from
	// this module.
	type InitializeMembers = Council;
	type KickedMember = Treasury;
	type LoserCandidate = Treasury;
	type PalletId = PhragmenElectionPalletId;
	type TermDuration = TermDuration;
	type VotingBondBase = VotingBondBase;
	type VotingBondFactor = VotingBondFactor;
	type WeightInfo = WeightInfo<Runtime>;
}
