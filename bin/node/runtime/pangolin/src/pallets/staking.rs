// --- substrate ---
use sp_npos_elections::CompactSolution;
use sp_runtime::ModuleId;
use sp_staking::SessionIndex;
// --- darwinia ---
use crate::*;
use darwinia_staking::{weights::SubstrateWeight, Config, EraIndex};

frame_support::parameter_types! {
	pub const StakingModuleId: ModuleId = ModuleId(*b"da/staki");
	pub const SessionsPerEra: SessionIndex = SESSIONS_PER_ERA;
	pub const BondingDurationInEra: EraIndex = 2;
	pub const BondingDurationInBlockNumber: BlockNumber = 2 * BLOCKS_PER_SESSION * SESSIONS_PER_ERA;
	pub const SlashDeferDuration: EraIndex = 1;
	pub const MaxNominatorRewardedPerValidator: u32 = 128;
	pub const Cap: Balance = CAP;
	pub const TotalPower: Power = TOTAL_POWER;
}
impl Config for Runtime {
	const MAX_NOMINATIONS: u32 = <NposCompactSolution16 as CompactSolution>::LIMIT as u32;
	type Event = Event;
	type ModuleId = StakingModuleId;
	type UnixTime = Timestamp;
	type SessionsPerEra = SessionsPerEra;
	type BondingDurationInEra = BondingDurationInEra;
	type BondingDurationInBlockNumber = BondingDurationInBlockNumber;
	type SlashDeferDuration = SlashDeferDuration;
	/// A super-majority of the council can cancel the slash.
	type SlashCancelOrigin = EnsureRootOrHalfCouncil;
	type SessionInterface = Self;
	type NextNewSession = Session;
	type MaxNominatorRewardedPerValidator = MaxNominatorRewardedPerValidator;
	type ElectionProvider = ElectionProviderMultiPhase;
	type RingCurrency = Ring;
	type RingRewardRemainder = Treasury;
	// send the slashed funds to the treasury.
	type RingSlash = Treasury;
	// rewards are minted from the void
	type RingReward = ();
	type KtonCurrency = Kton;
	// send the slashed funds to the treasury.
	type KtonSlash = Treasury;
	// rewards are minted from the void
	type KtonReward = ();
	type Cap = Cap;
	type TotalPower = TotalPower;
	type WeightInfo = SubstrateWeight<Runtime>;
}
