// --- paritytech ---
use frame_election_provider_support::onchain::OnChainSequentialPhragmen;
use frame_support::PalletId;
use pallet_election_provider_multi_phase::OnChainConfig;
use sp_npos_elections::CompactSolution;
use sp_staking::SessionIndex;
// --- darwinia-network ---
use crate::*;
use darwinia_staking::{Config, EraIndex};

pub const MAX_NOMINATIONS: u32 = <NposCompactSolution16 as CompactSolution>::LIMIT as u32;

frame_support::parameter_types! {
	pub const StakingPalletId: PalletId = PalletId(*b"da/staki");
	pub const SessionsPerEra: SessionIndex = PANGOLIN_SESSIONS_PER_ERA;
	pub const BondingDurationInEra: EraIndex = 2;
	pub const BondingDurationInBlockNumber: BlockNumber = 2 * PANGOLIN_BLOCKS_PER_SESSION * PANGOLIN_SESSIONS_PER_ERA;
	pub const SlashDeferDuration: EraIndex = 1;
	pub const MaxNominatorRewardedPerValidator: u32 = 128;
	pub const Cap: Balance = RING_HARD_CAP;
	pub const TotalPower: Power = TOTAL_POWER;
}

impl Config for Runtime {
	const MAX_NOMINATIONS: u32 = MAX_NOMINATIONS;
	type Event = Event;
	type PalletId = StakingPalletId;
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
	type GenesisElectionProvider = OnChainSequentialPhragmen<OnChainConfig<Self>>;
	type RingCurrency = Ring;
	type RingRewardRemainder = Treasury;
	// send the slashed funds to the treasury.
	type RingSlash = Treasury;
	// rewards are minted from the void
	type RingReward = ();
	type KtonCurrency = Kton;
	// send the slashed funds to the treasury.
	type KtonSlash = KtonTreasury;
	// rewards are minted from the void
	type KtonReward = ();
	type Cap = Cap;
	type TotalPower = TotalPower;
	type WeightInfo = ();
}
