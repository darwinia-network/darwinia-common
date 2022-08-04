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

//! The rs-module that manages the basic account info in dvm.

// --- crates.io ---
use evm::ExitError;
// --- paritytech ---
use frame_support::{
	ensure,
	traits::{Currency, WithdrawReasons},
};
use sp_core::U256;
use sp_runtime::{traits::UniqueSaturatedInto, SaturatedConversion};
// --- darwinia-network ---
use crate::{Config, Event, Pallet, RemainingKtonBalance, RemainingRingBalance};
use darwinia_evm::CurrencyAdapt;
use darwinia_support::evm::{decimal_convert, POW_9};

/// The operations for the remaining balance.
pub trait RemainBalanceOp<T: Config> {
	/// Get the remaining balance
	fn remaining_balance(account_id: &T::AccountId) -> u128;
	/// Set the remaining balance
	fn set_remaining_balance(account_id: &T::AccountId, value: u128);
	/// Remove the remaining balance
	fn remove_remaining_balance(account_id: &T::AccountId);
	/// Inc remaining balance
	fn inc_remaining_balance(account_id: &T::AccountId, value: u128);
	/// Dec remaining balance
	fn dec_remaining_balance(account_id: &T::AccountId, value: u128);
	/// Deposit dvm related transfer events
	fn deposit_dvm_transfer_event(source: &T::AccountId, target: &T::AccountId, value: U256);
}

/// The Remaining *RING* balance.
pub struct RingRemainBalance;
impl<T: Config> RemainBalanceOp<T> for RingRemainBalance {
	/// Get the remaining balance.
	fn remaining_balance(account_id: &T::AccountId) -> u128 {
		<RemainingRingBalance<T>>::get(account_id)
	}

	/// Set the remaining balance.
	fn set_remaining_balance(account_id: &T::AccountId, value: u128) {
		<RemainingRingBalance<T>>::insert(account_id, value)
	}

	/// Remove the remaining balance.
	fn remove_remaining_balance(account_id: &T::AccountId) {
		<RemainingRingBalance<T>>::remove(account_id)
	}

	/// Inc remaining balance.
	fn inc_remaining_balance(account_id: &T::AccountId, value: u128) {
		let remain_balance = <Self as RemainBalanceOp<T>>::remaining_balance(account_id);
		let updated_balance = remain_balance.saturating_add(value);
		<RemainingRingBalance<T>>::insert(account_id, updated_balance);
	}

	/// Dec remaining balance.
	fn dec_remaining_balance(account_id: &T::AccountId, value: u128) {
		let remain_balance = <Self as RemainBalanceOp<T>>::remaining_balance(account_id);
		let updated_balance = remain_balance.saturating_sub(value);
		<RemainingRingBalance<T>>::insert(account_id, updated_balance);
	}

	/// Deposit dvm transfer event
	fn deposit_dvm_transfer_event(source: &T::AccountId, target: &T::AccountId, value: U256) {
		Pallet::<T>::deposit_event(Event::DVMTransfer(source.clone(), target.clone(), value));
	}
}

/// The Remaining *KTON* balance.
pub struct KtonRemainBalance;
impl<T: Config> RemainBalanceOp<T> for KtonRemainBalance {
	/// Get the remaining balance.
	fn remaining_balance(account_id: &T::AccountId) -> u128 {
		<RemainingKtonBalance<T>>::get(account_id)
	}

	/// Set the remaining balance.
	fn set_remaining_balance(account_id: &T::AccountId, value: u128) {
		<RemainingKtonBalance<T>>::insert(account_id, value)
	}

	/// Remove the remaining balance.
	fn remove_remaining_balance(account_id: &T::AccountId) {
		<RemainingKtonBalance<T>>::remove(account_id)
	}

	/// Inc remaining balance.
	fn inc_remaining_balance(account_id: &T::AccountId, value: u128) {
		let remain_balance = <Self as RemainBalanceOp<T>>::remaining_balance(account_id);
		let updated_balance = remain_balance.saturating_add(value);
		<RemainingKtonBalance<T>>::insert(account_id, updated_balance);
	}

	/// Dec remaining balance.
	fn dec_remaining_balance(account_id: &T::AccountId, value: u128) {
		let remain_balance = <Self as RemainBalanceOp<T>>::remaining_balance(account_id);
		let updated_balance = remain_balance.saturating_sub(value);
		<RemainingKtonBalance<T>>::insert(account_id, updated_balance);
	}

	/// Deposit dvm transfer event
	fn deposit_dvm_transfer_event(source: &T::AccountId, target: &T::AccountId, value: U256) {
		Pallet::<T>::deposit_event(Event::KtonDVMTransfer(source.clone(), target.clone(), value));
	}
}

/// A currency adapter to deal with different decimal between native and evm tokens.
pub struct CurrencyAdapter<T, C, RB>(sp_std::marker::PhantomData<(T, C, RB)>);
impl<T: Config, C, RB> CurrencyAdapt<T> for CurrencyAdapter<T, C, RB>
where
	RB: RemainBalanceOp<T>,
	C: Currency<T::AccountId>,
{
	/// Get account balance, the decimal of the returned result is consistent with Ethereum.
	fn account_balance(account_id: &T::AccountId) -> U256 {
		// Get main balance from Currency.
		let main_balance = C::free_balance(&account_id).saturated_into::<u128>();
		// Get remaining balance from Dvm.
		let remaining_balance = RB::remaining_balance(&account_id).saturated_into::<u128>();
		// final_balance = balance * 10^9 + remaining_balance.
		decimal_convert(main_balance, Some(remaining_balance))
	}

	/// Get the total supply of token in Ethereum decimal.
	fn evm_total_supply() -> U256 {
		let main_balance = C::total_issuance().saturated_into::<u128>();
		decimal_convert(main_balance, None)
	}

	/// Transfer value. the value's decimal should be the same as Ethereum.
	fn evm_transfer(
		source: &T::AccountId,
		target: &T::AccountId,
		value: U256,
	) -> Result<(), ExitError> {
		if value == U256::zero() || source == target {
			return Ok(());
		}
		let source_balance = Self::account_balance(source);
		ensure!(source_balance >= value, ExitError::OutOfFund);
		let new_source_balance = source_balance.saturating_sub(value);
		Self::mutate_account_balance(source, new_source_balance);

		let target_balance = Self::account_balance(target);
		let new_target_balance = target_balance.saturating_add(value);
		Self::mutate_account_balance(target, new_target_balance);

		RB::deposit_dvm_transfer_event(source, target, value);
		Ok(())
	}

	/// Mutate account balance, the new_balance's decimal should be the same as Ethereum.
	fn mutate_account_balance(account_id: &T::AccountId, new_balance: U256) {
		debug_assert_eq!(C::minimum_balance().saturated_into::<u128>(), 0, "The Ed must be zero!");
		let helper = U256::from(POW_9);

		let current = Self::account_balance(account_id);
		let dvm_balance: U256 = RB::remaining_balance(&account_id).saturated_into::<u128>().into();

		let nb = new_balance;
		match current {
			cb if cb > nb => {
				let diff = cb.saturating_sub(nb);
				let (diff_main, diff_remaining) = diff.div_mod(helper);

				// If the dvm storage < diff remaining balance, we can not do sub operation
				// directly. Otherwise, slash Currency, dec dvm storage balance directly.
				if dvm_balance < diff_remaining {
					let remaining_balance = dvm_balance
						.saturating_add(decimal_convert(1, None))
						.saturating_sub(diff_remaining);

					C::slash(&account_id, (diff_main + 1).low_u128().unique_saturated_into());
					RB::set_remaining_balance(&account_id, remaining_balance.low_u128());
				} else {
					C::slash(&account_id, diff_main.low_u128().unique_saturated_into());
					RB::dec_remaining_balance(&account_id, diff_remaining.low_u128());
				}
			},
			cb if cb < nb => {
				let diff = nb.saturating_sub(cb);
				let (diff_main, diff_remaining) = diff.div_mod(helper);

				// If dvm storage `balance + diff remaining balance > helper`, we must update
				// Currency balance.
				if dvm_balance + diff_remaining >= helper {
					let remaining_balance =
						dvm_balance.saturating_add(diff_remaining).saturating_sub(helper);

					C::deposit_creating(
						&account_id,
						(diff_main + 1).low_u128().unique_saturated_into(),
					);
					RB::set_remaining_balance(&account_id, remaining_balance.low_u128());
				} else {
					C::deposit_creating(&account_id, diff_main.low_u128().unique_saturated_into());
					RB::inc_remaining_balance(&account_id, diff_remaining.low_u128());
				}
			},
			_ => return,
		}
	}

	/// Ensure that an account can withdraw from their fee balance.The account's decimal is the same
	/// as Ethereum.
	fn ensure_can_withdraw(
		who: &T::AccountId,
		amount: U256,
		reasons: WithdrawReasons,
	) -> Result<(), ExitError> {
		// Ensure the evm balance of the account large than the withdraw amount
		let old_evm_balance = Self::account_balance(who);
		let (_old_sub, old_remaining) = old_evm_balance.div_mod(U256::from(POW_9));
		ensure!(old_evm_balance >= amount, ExitError::OutOfFund);

		// Because of precision difference, the amount also needs to convert.
		let (mut amount_sub, amount_remaining) = amount.div_mod(U256::from(POW_9));
		if old_remaining < amount_remaining {
			// Add 1, if the substrate balance part touched
			amount_sub = amount_sub.saturating_add(U256::from(1));
		}

		// Calculate the new substrate balance part
		let new_evm_balance = old_evm_balance.saturating_sub(amount);
		let (new_sub, _new_remaining) = new_evm_balance.div_mod(U256::from(POW_9));

		// Ensure the account underlying substrate account has no liquidity restrictions.
		C::ensure_can_withdraw(
			who,
			amount_sub.low_u128().unique_saturated_into(),
			reasons,
			new_sub.low_u128().unique_saturated_into(),
		)
		.map_err(|_| ExitError::OutOfFund)
	}
}
