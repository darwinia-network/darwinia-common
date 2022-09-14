// --- paritytech ---
use frame_support::traits::{Currency, LockIdentifier};
use frame_system::pallet_prelude::*;
// --- darwinia-network ---
use crate::*;

/// Counter for the number of "reward" points earned by a given validator.
pub type RewardPoint = u32;

/// Balance of an account.
pub type Balance = u128;
/// Power of an account.
pub type Power = u32;
/// A timestamp: milliseconds since the unix epoch.
/// `u64` is enough to represent a duration of half a billion years, when the
/// time scale is milliseconds.
pub type TsInMs = u64;

pub type StakingLedgerT<T> =
	StakingLedger<AccountId<T>, RingBalance<T>, KtonBalance<T>, BlockNumberFor<T>>;
pub type StakingBalanceT<T> = StakingBalance<RingBalance<T>, KtonBalance<T>>;
pub type ExposureT<T> = Exposure<AccountId<T>, RingBalance<T>, KtonBalance<T>>;

pub type AccountId<T> = <T as frame_system::Config>::AccountId;

pub type RingBalance<T> = <RingCurrency<T> as Currency<AccountId<T>>>::Balance;
pub type RingPositiveImbalance<T> = <RingCurrency<T> as Currency<AccountId<T>>>::PositiveImbalance;
pub type RingNegativeImbalance<T> = <RingCurrency<T> as Currency<AccountId<T>>>::NegativeImbalance;

pub type KtonBalance<T> = <KtonCurrency<T> as Currency<AccountId<T>>>::Balance;
pub type KtonPositiveImbalance<T> = <KtonCurrency<T> as Currency<AccountId<T>>>::PositiveImbalance;
pub type KtonNegativeImbalance<T> = <KtonCurrency<T> as Currency<AccountId<T>>>::NegativeImbalance;

type RingCurrency<T> = <T as Config>::RingCurrency;
type KtonCurrency<T> = <T as Config>::KtonCurrency;

pub const LOG_TARGET: &'static str = "runtime::staking";

pub const STAKING_ID: LockIdentifier = *b"da/staki";

// TODO: Limited in frame/support/src/lib.rs `StakingLock`
pub const MAX_UNLOCKING_CHUNKS: usize = 32;

pub const MONTH_IN_MINUTES: TsInMs = 30 * 24 * 60;
pub const MONTH_IN_MILLISECONDS: TsInMs = MONTH_IN_MINUTES * 60 * 1000;
