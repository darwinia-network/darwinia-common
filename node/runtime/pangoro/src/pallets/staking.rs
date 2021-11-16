// --- paritytech ---
use frame_election_provider_support::onchain::OnChainSequentialPhragmen;
use frame_support::PalletId;
use frame_system::EnsureRoot;
use pallet_election_provider_multi_phase::OnChainConfig;
use sp_npos_elections::CompactSolution;
use sp_staking::SessionIndex;
// --- darwinia-network ---
use crate::*;
use darwinia_staking::{Config, EraIndex};

pub const MAX_NOMINATIONS: u32 = <NposCompactSolution16 as CompactSolution>::LIMIT as u32;

frame_support::parameter_types! {
	pub const StakingPalletId: PalletId = PalletId(*b"da/staki");
	pub const SessionsPerEra: SessionIndex = SESSIONS_PER_ERA;
	pub const BondingDurationInEra: EraIndex = 2;
	pub const BondingDurationInBlockNumber: BlockNumber = 2 * BLOCKS_PER_SESSION * SESSIONS_PER_ERA;
	pub const SlashDeferDuration: EraIndex = 1;
	pub const MaxNominatorRewardedPerValidator: u32 = 128;
	pub const Cap: Balance = CAP;
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
	type SlashCancelOrigin = EnsureRoot<AccountId>;
	type SessionInterface = Self;
	type NextNewSession = Session;
	type MaxNominatorRewardedPerValidator = MaxNominatorRewardedPerValidator;
	type ElectionProvider = ElectionProviderMultiPhase;
	type GenesisElectionProvider = OnChainSequentialPhragmen<OnChainConfig<Self>>;
	type RingCurrency = Ring;
	type RingRewardRemainder = ();
	// send the slashed funds to the treasury.
	type RingSlash = ();
	// rewards are minted from the void
	type RingReward = ();
	type KtonCurrency = Kton;
	// send the slashed funds to the treasury.
	type KtonSlash = ();
	// rewards are minted from the void
	type KtonReward = ();
	type Cap = Cap;
	type TotalPower = TotalPower;
	type WeightInfo = ();
}
