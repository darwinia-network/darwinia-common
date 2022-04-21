// This file is part of Darwinia.
//
// Copyright (C) 2018-2022 Darwinia Network
// SPDX-License-Identifier: GPL-3.0
//
// Darwinia is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Darwinia is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

// --- crates.io ---
use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
// --- paritytech ---
use frame_support::{
	traits::{ConstU32, LockIdentifier},
	WeakBoundedVec,
};
use pallet_balances::Reasons;
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, Zero},
	RuntimeDebug,
};
use sp_std::prelude::*;

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
	pub fn frozen_for(self, reasons: Reasons) -> Balance {
		match reasons {
			Reasons::All => self.misc.max(self.fee),
			Reasons::Misc => self.misc,
			Reasons::Fee => self.fee,
		}
	}
}

/// A single lock on a balance. There can be many of these on an account and they "overlap", so the
/// same balance is frozen by multiple locks.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct OldBalanceLock<Balance, Moment> {
	/// An identifier for this lock. Only one lock may be in existence for each identifier.
	pub id: LockIdentifier,
	pub lock_for: LockFor<Balance, Moment>,
	/// If true, then the lock remains in effect even for payment of transaction fees.
	pub reasons: Reasons,
}
#[cfg(feature = "easy-testing")]
impl<Balance, Moment> OldBalanceLock<Balance, Moment>
where
	Balance: Copy + PartialOrd + AtLeast32BitUnsigned,
	Moment: Copy + PartialOrd,
{
	// For performance, we don't need the `at` in some cases
	// Only use for tests to avoid write a lot of matches in tests
	pub fn locked_amount(&self) -> Balance {
		match &self.lock_for {
			LockFor::Common { amount } => *amount,
			LockFor::Staking(staking_lock) => staking_lock.locked_amount(),
		}
	}
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub enum LockFor<Balance, Moment> {
	Common { amount: Balance },
	Staking(StakingLock<Balance, Moment>),
}

#[derive(Clone, Default, PartialEq, Eq, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct StakingLock<Balance, Moment> {
	/// The amount which the free balance may not drop below when this lock is in effect.
	pub staking_amount: Balance,
	pub unbondings: WeakBoundedVec<Unbonding<Balance, Moment>, ConstU32<32>>,
}
impl<Balance, Moment> StakingLock<Balance, Moment>
where
	Balance: Copy + PartialOrd + AtLeast32BitUnsigned + Zero,
	Moment: Copy + PartialOrd,
{
	#[inline]
	pub fn locked_amount(&self) -> Balance {
		self.unbondings.iter().fold(Zero::zero(), |acc, unbonding| {
			acc.saturating_add(unbonding.amount)
		})
	}

	#[inline]
	pub fn refresh(&mut self, at: Moment) {
		self.unbondings.retain(|unbonding| unbonding.valid_at(at));
	}
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
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
