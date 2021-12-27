// --- paritytech ---
use frame_election_provider_support::{onchain, SequentialPhragmen};
use pallet_election_provider_multi_phase::{Config, NoFallback, SolutionAccuracyOf};
use sp_runtime::{transaction_validity::TransactionPriority, PerU16, Perbill};
// --- darwinia-network ---
use crate::*;

sp_npos_elections::generate_solution_type!(
	#[compact]
	pub struct NposCompactSolution16::<
		VoterIndex = u32,
		TargetIndex = u16,
		Accuracy = PerU16,
	>(16)
);

/// The numbers configured here should always be more than the the maximum limits of staking pallet
/// to ensure election snapshot will not run out of memory.
pub struct BenchmarkConfig;
impl pallet_election_provider_multi_phase::BenchmarkingConfig for BenchmarkConfig {
	const VOTERS: [u32; 2] = [5_000, 10_000];
	const TARGETS: [u32; 2] = [1_000, 2_000];
	const ACTIVE_VOTERS: [u32; 2] = [1000, 4_000];
	const DESIRED_TARGETS: [u32; 2] = [400, 800];
	const SNAPSHOT_MAXIMUM_VOTERS: u32 = 25_000;
	const MINER_MAXIMUM_VOTERS: u32 = 15_000;
	const MAXIMUM_TARGETS: u32 = 2000;
}

frame_support::parameter_types! {
	// no signed phase for now, just unsigned.
	pub const SignedPhase: u32 = 0;
	pub const UnsignedPhase: u32 = PANGORO_BLOCKS_PER_SESSION / 4;

	// signed config
	pub const SignedMaxSubmissions: u32 = 10;
	pub const SignedRewardBase: Balance = 1 * MILLI;
	pub const SignedDepositBase: Balance = 1 * MILLI;
	pub const SignedDepositByte: Balance = 1 * MICRO;

	pub SolutionImprovementThreshold: Perbill = Perbill::from_rational(5u32, 10_000);

	// miner configs
	pub NposSolutionPriority: TransactionPriority = Perbill::from_percent(90) * TransactionPriority::max_value();
	pub const OffchainRepeat: BlockNumber = 5;
}

impl Config for Runtime {
	type Event = Event;
	type Currency = Ring;
	type EstimateCallFee = TransactionPayment;
	type SignedPhase = SignedPhase;
	type UnsignedPhase = UnsignedPhase;
	type SolutionImprovementThreshold = SolutionImprovementThreshold;
	type MinerMaxWeight = OffchainSolutionWeightLimit;
	type MinerMaxLength = OffchainSolutionLengthLimit; // For now use the one from staking.
	type OffchainRepeat = OffchainRepeat;
	type MinerTxPriority = NposSolutionPriority;
	type SignedMaxSubmissions = SignedMaxSubmissions;
	type SignedRewardBase = SignedRewardBase;
	type SignedDepositBase = SignedDepositBase;
	type SignedDepositByte = SignedDepositByte;
	type SignedDepositWeight = ();
	type SignedMaxWeight = Self::MinerMaxWeight;
	type SlashHandler = (); // burn slashes
	type RewardHandler = (); // nothing to do upon rewards
	type DataProvider = Staking;
	type Solution = NposCompactSolution16;
	type Fallback = NoFallback<Self>;
	type Solver = SequentialPhragmen<AccountId, SolutionAccuracyOf<Self>, OffchainRandomBalancing>;
	type WeightInfo = ();
	type ForceOrigin = RootOrigin;
	type BenchmarkingConfig = BenchmarkConfig;
}
