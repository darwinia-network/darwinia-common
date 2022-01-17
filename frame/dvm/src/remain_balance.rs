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

// --- paritytech ---
use frame_support::{ensure, traits::Currency};
use sp_core::{H160, U256};
use sp_runtime::{
	traits::{Saturating, UniqueSaturatedInto},
	SaturatedConversion,
};
// --- darwinia-network ---
use crate::{Config, KtonBalance, RemainingKtonBalance, RemainingRingBalance, RingBalance};

/// The operations for the remaining balance.
pub trait RemainBalanceOp<T: Config, B> {
	/// Get the remaining balance
	fn remaining_balance(account_id: &T::AccountId) -> B;
	/// Set the remaining balance
	fn set_remaining_balance(account_id: &T::AccountId, value: B);
	/// Remove the remaining balance
	fn remove_remaining_balance(account_id: &T::AccountId);
	/// Inc remaining balance
	fn inc_remaining_balance(account_id: &T::AccountId, value: B);
	/// Dec remaining balance
	fn dec_remaining_balance(account_id: &T::AccountId, value: B);
}

/// The Remaining *RING* balance.
pub struct RingRemainBalance;
impl<T: Config> RemainBalanceOp<T, RingBalance<T>> for RingRemainBalance {
	/// Get the remaining balance.
	fn remaining_balance(account_id: &T::AccountId) -> RingBalance<T> {
		<RemainingRingBalance<T>>::get(account_id)
	}
	/// Set the remaining balance.
	fn set_remaining_balance(account_id: &T::AccountId, value: RingBalance<T>) {
		<RemainingRingBalance<T>>::insert(account_id, value)
	}
	/// Remove the remaining balance.
	fn remove_remaining_balance(account_id: &T::AccountId) {
		<RemainingRingBalance<T>>::remove(account_id)
	}
	/// Inc remaining balance.
	fn inc_remaining_balance(account_id: &T::AccountId, value: RingBalance<T>) {
		let remain_balance =
			<Self as RemainBalanceOp<T, RingBalance<T>>>::remaining_balance(account_id);
		let updated_balance = remain_balance.saturating_add(value);
		<RemainingRingBalance<T>>::insert(account_id, updated_balance);
	}
	/// Dec remaining balance.
	fn dec_remaining_balance(account_id: &T::AccountId, value: RingBalance<T>) {
		let remain_balance =
			<Self as RemainBalanceOp<T, RingBalance<T>>>::remaining_balance(account_id);
		let updated_balance = remain_balance.saturating_sub(value);
		<RemainingRingBalance<T>>::insert(account_id, updated_balance);
	}
}

/// The Remaining *KTON* balance.
pub struct KtonRemainBalance;
impl<T: Config> RemainBalanceOp<T, KtonBalance<T>> for KtonRemainBalance {
	/// Get the remaining balance.
	fn remaining_balance(account_id: &T::AccountId) -> KtonBalance<T> {
		<RemainingKtonBalance<T>>::get(account_id)
	}
	/// Set the remaining balance.
	fn set_remaining_balance(account_id: &T::AccountId, value: KtonBalance<T>) {
		<RemainingKtonBalance<T>>::insert(account_id, value)
	}
	/// Remove the remaining balance.
	fn remove_remaining_balance(account_id: &T::AccountId) {
		<RemainingKtonBalance<T>>::remove(account_id)
	}
	/// Inc remaining balance.
	fn inc_remaining_balance(account_id: &T::AccountId, value: KtonBalance<T>) {
		let remain_balance =
			<Self as RemainBalanceOp<T, KtonBalance<T>>>::remaining_balance(account_id);
		let updated_balance = remain_balance.saturating_add(value);
		<RemainingKtonBalance<T>>::insert(account_id, updated_balance);
	}
	/// Dec remaining balance.
	fn dec_remaining_balance(account_id: &T::AccountId, value: KtonBalance<T>) {
		let remain_balance =
			<Self as RemainBalanceOp<T, KtonBalance<T>>>::remaining_balance(account_id);
		let updated_balance = remain_balance.saturating_sub(value);
		<RemainingKtonBalance<T>>::insert(account_id, updated_balance);
	}
}
