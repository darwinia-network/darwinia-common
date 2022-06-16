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
use frame_support::{traits::ConstU32, WeakBoundedVec};
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, Zero},
	RuntimeDebug,
};
use sp_std::prelude::*;

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
	// TODO: Remove this and bring `ledger.total` back.
	#[inline]
	pub fn total_unbond_at(&self, at: Moment) -> Balance {
		self.unbondings
			.iter()
			.fold(Zero::zero(), |acc, unbonding| acc.saturating_add(unbonding.locked_amount(at)))
	}

	#[inline]
	#[deprecated = "If you know what you are doing now."]
	pub fn total_unbond(&self) -> Balance {
		self.unbondings
			.iter()
			.fold(Zero::zero(), |acc, unbonding| acc.saturating_add(unbonding.amount))
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
