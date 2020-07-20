// --- crates ---
use codec::{Decode, Encode};
use num_traits::Zero;
// --- substrate ---
use sp_runtime::{traits::AtLeast32Bit, RuntimeDebug};
use sp_std::{ops::BitOr, prelude::*};
// --- darwinia ---
use crate::balance::lock::{LockIdentifier, WithdrawReason, WithdrawReasons};

/// Frozen balance information for an account.
pub struct FrozenBalance<Balance> {
	/// The amount that `free` may not drop below when withdrawing specifically for transaction
	/// fee payment.
	pub fee: Balance,
	/// The amount that `free` may not drop below when withdrawing for *anything except transaction
	/// fee payment*.
	pub misc: Balance,
}

impl<Balance> FrozenBalance<Balance>
where
	Balance: Copy + Ord + Zero,
{
	pub fn zero() -> Self {
		Self {
			fee: Zero::zero(),
			misc: Zero::zero(),
		}
	}

	/// The amount that this account's free balance may not be reduced beyond for the given
	/// `reasons`.
	pub fn frozen_for(self, reasons: LockReasons) -> Balance {
		match reasons {
			LockReasons::All => self.misc.max(self.fee),
			LockReasons::Misc => self.misc,
			LockReasons::Fee => self.fee,
		}
	}
}

/// Simplified reasons for withdrawing balance.
#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug)]
pub enum LockReasons {
	/// Paying system transaction fees.
	Fee = 0,
	/// Any reason other than paying system transaction fees.
	Misc = 1,
	/// Any reason at all.
	All = 2,
}

impl From<WithdrawReasons> for LockReasons {
	fn from(r: WithdrawReasons) -> LockReasons {
		if r == WithdrawReasons::from(WithdrawReason::TransactionPayment) {
			LockReasons::Fee
		} else if r.contains(WithdrawReason::TransactionPayment) {
			LockReasons::All
		} else {
			LockReasons::Misc
		}
	}
}

impl BitOr for LockReasons {
	type Output = LockReasons;
	fn bitor(self, other: LockReasons) -> LockReasons {
		if self == other {
			return self;
		}
		LockReasons::All
	}
}

/// A single lock on a balance. There can be many of these on an account and they "overlap", so the
/// same balance is frozen by multiple locks.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct BalanceLock<Balance, Moment> {
	/// An identifier for this lock. Only one lock may be in existence for each identifier.
	pub id: LockIdentifier,
	pub lock_for: LockFor<Balance, Moment>,
	/// If true, then the lock remains in effect even for payment of transaction fees.
	pub lock_reasons: LockReasons,
}

#[cfg(feature = "easy-testing")]
impl<Balance, Moment> BalanceLock<Balance, Moment>
where
	Balance: Copy + PartialOrd + AtLeast32Bit,
	Moment: Copy + PartialOrd,
{
	// For performance, we don't need the `at` in some cases
	// Only use for tests to avoid write a lot of matches in tests
	pub fn locked_amount(&self, at: Option<Moment>) -> Balance {
		match &self.lock_for {
			LockFor::Common { amount } => *amount,
			LockFor::Staking(staking_lock) => staking_lock
				.locked_amount(at.expect("This's a `StakingLock`, please specify the `Moment`.")),
		}
	}
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug)]
pub enum LockFor<Balance, Moment> {
	Common { amount: Balance },
	Staking(StakingLock<Balance, Moment>),
}

#[derive(Clone, Default, PartialEq, Eq, Encode, Decode, RuntimeDebug)]
pub struct StakingLock<Balance, Moment> {
	/// The amount which the free balance may not drop below when this lock is in effect.
	pub staking_amount: Balance,
	pub unbondings: Vec<Unbonding<Balance, Moment>>,
}

impl<Balance, Moment> StakingLock<Balance, Moment>
where
	Balance: Copy + PartialOrd + AtLeast32Bit,
	Moment: Copy + PartialOrd,
{
	#[inline]
	pub fn locked_amount(&self, at: Moment) -> Balance {
		self.unbondings
			.iter()
			.fold(self.staking_amount, |acc, unbonding| {
				if unbonding.valid_at(at) {
					acc.saturating_add(unbonding.amount)
				} else {
					acc
				}
			})
	}

	#[inline]
	pub fn update(&mut self, at: Moment) {
		let mut locked_amount = self.staking_amount;

		self.unbondings.retain(|unbonding| {
			let valid = unbonding.valid_at(at);
			if valid {
				locked_amount = locked_amount.saturating_add(unbonding.amount);
			}

			valid
		});
	}
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug)]
pub struct Unbonding<Balance, Moment> {
	/// The amount which the free balance may not drop below when this lock is in effect.
	pub amount: Balance,
	pub until: Moment,
}

impl<Balance, Moment> Unbonding<Balance, Moment>
where
	Balance: Copy + PartialOrd + Zero,
	Moment: PartialOrd,
{
	#[inline]
	fn valid_at(&self, at: Moment) -> bool {
		self.until > at
	}

	#[inline]
	pub fn locked_amount(&self, at: Moment) -> Balance {
		if self.valid_at(at) {
			self.amount
		} else {
			Zero::zero()
		}
	}
}

#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug)]
pub struct TcHeaderBrief<TcBlockNumber, TcHeaderHash, TcHeaderMMR> {
	pub number: TcBlockNumber,
	pub hash: TcHeaderHash,
	pub parent_hash: TcHeaderHash,
	pub mmr: TcHeaderMMR,
	pub others: Vec<u8>,
}
