// This file is part of Darwinia.
//
// Copyright (C) 2018-2021 Darwinia Network
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

use crate::{Config, KtonBalance, RemainingKtonBalance, RemainingRingBalance, RingBalance};
use darwinia_evm::{Account as EVMAccount, AccountBasic, AddressMapping};
use darwinia_support::evm::POW_9;
use evm::ExitError;
use frame_support::{ensure, traits::Currency};
use sp_core::{H160, U256};
use sp_runtime::{
	traits::{Saturating, UniqueSaturatedInto},
	SaturatedConversion,
};

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

pub struct RingRemainBalance;
impl<T: Config> RemainBalanceOp<T, RingBalance<T>> for RingRemainBalance {
	/// Get the remaining balance
	fn remaining_balance(account_id: &T::AccountId) -> RingBalance<T> {
		<RemainingRingBalance<T>>::get(account_id)
	}
	/// Set the remaining balance
	fn set_remaining_balance(account_id: &T::AccountId, value: RingBalance<T>) {
		<RemainingRingBalance<T>>::insert(account_id, value)
	}
	/// Remove the remaining balance
	fn remove_remaining_balance(account_id: &T::AccountId) {
		<RemainingRingBalance<T>>::remove(account_id)
	}
	/// Inc remaining balance
	fn inc_remaining_balance(account_id: &T::AccountId, value: RingBalance<T>) {
		let remain_balance =
			<Self as RemainBalanceOp<T, RingBalance<T>>>::remaining_balance(account_id);
		let updated_balance = remain_balance.saturating_add(value);
		<RemainingRingBalance<T>>::insert(account_id, updated_balance);
	}
	/// Dec remaining balance
	fn dec_remaining_balance(account_id: &T::AccountId, value: RingBalance<T>) {
		let remain_balance =
			<Self as RemainBalanceOp<T, RingBalance<T>>>::remaining_balance(account_id);
		let updated_balance = remain_balance.saturating_sub(value);
		<RemainingRingBalance<T>>::insert(account_id, updated_balance);
	}
}

pub struct KtonRemainBalance;
impl<T: Config> RemainBalanceOp<T, KtonBalance<T>> for KtonRemainBalance {
	/// Get the remaining balance
	fn remaining_balance(account_id: &T::AccountId) -> KtonBalance<T> {
		<RemainingKtonBalance<T>>::get(account_id)
	}
	/// Set the remaining balance
	fn set_remaining_balance(account_id: &T::AccountId, value: KtonBalance<T>) {
		<RemainingKtonBalance<T>>::insert(account_id, value)
	}
	/// Remove the remaining balance
	fn remove_remaining_balance(account_id: &T::AccountId) {
		<RemainingKtonBalance<T>>::remove(account_id)
	}
	/// Inc remaining balance
	fn inc_remaining_balance(account_id: &T::AccountId, value: KtonBalance<T>) {
		let remain_balance =
			<Self as RemainBalanceOp<T, KtonBalance<T>>>::remaining_balance(account_id);
		let updated_balance = remain_balance.saturating_add(value);
		<RemainingKtonBalance<T>>::insert(account_id, updated_balance);
	}
	/// Dec remaining balance
	fn dec_remaining_balance(account_id: &T::AccountId, value: KtonBalance<T>) {
		let remain_balance =
			<Self as RemainBalanceOp<T, KtonBalance<T>>>::remaining_balance(account_id);
		let updated_balance = remain_balance.saturating_sub(value);
		<RemainingKtonBalance<T>>::insert(account_id, updated_balance);
	}
}

pub struct DvmAccountBasic<T, C, RB>(sp_std::marker::PhantomData<(T, C, RB)>);
impl<T: Config, C, RB> AccountBasic for DvmAccountBasic<T, C, RB>
where
	RB: RemainBalanceOp<T, C::Balance>,
	C: Currency<T::AccountId>,
{
	/// Get the account basic in EVM format.
	fn account_basic(address: &H160) -> EVMAccount {
		let account_id = <T as darwinia_evm::Config>::AddressMapping::into_account_id(*address);
		let nonce = <frame_system::Pallet<T>>::account_nonce(&account_id);
		let helper = U256::from(POW_9);

		// Get balance from Currency
		let balance: U256 = C::free_balance(&account_id).saturated_into::<u128>().into();

		// Get remaining balance from dvm
		let remaining_balance: U256 = RB::remaining_balance(&account_id)
			.saturated_into::<u128>()
			.into();

		// Final balance = balance * 10^9 + remaining_balance
		let final_balance = (balance * helper)
			.checked_add(remaining_balance)
			.unwrap_or_default();

		EVMAccount {
			nonce: nonce.saturated_into::<u128>().into(),
			balance: final_balance,
		}
	}

	/// Mutate the basic account
	fn mutate_account_basic(address: &H160, new: EVMAccount) {
		let helper = U256::from(POW_9);

		let account_id = <T as darwinia_evm::Config>::AddressMapping::into_account_id(*address);
		let current = Self::account_basic(address);
		let dvm_balance: U256 = RB::remaining_balance(&account_id)
			.saturated_into::<u128>()
			.into();

		let nb = new.balance;
		match current.balance {
			cb if cb > nb => {
				let diff = cb - nb;
				let (diff_balance, diff_remaining_balance) = diff.div_mod(helper);
				// If the dvm storage < diff remaining balance, we can not do sub operation directly.
				// Otherwise, slash Currency, dec dvm storage balance directly.
				if dvm_balance < diff_remaining_balance {
					let remaining_balance = dvm_balance
						.saturating_add(U256::from(1) * helper)
						.saturating_sub(diff_remaining_balance);

					C::slash(
						&account_id,
						(diff_balance + 1).low_u128().unique_saturated_into(),
					);
					RB::set_remaining_balance(
						&account_id,
						remaining_balance.low_u128().saturated_into(),
					);
				} else {
					C::slash(&account_id, diff_balance.low_u128().unique_saturated_into());
					RB::dec_remaining_balance(
						&account_id,
						diff_remaining_balance.low_u128().saturated_into(),
					);
				}
			}
			cb if cb < nb => {
				let diff = nb - cb;
				let (diff_balance, diff_remaining_balance) = diff.div_mod(helper);

				// If dvm storage balance + diff remaining balance > helper, we must update Currency balance.
				if dvm_balance + diff_remaining_balance >= helper {
					let remaining_balance = dvm_balance + diff_remaining_balance - helper;

					C::deposit_creating(
						&account_id,
						(diff_balance + 1).low_u128().unique_saturated_into(),
					);
					RB::set_remaining_balance(
						&account_id,
						remaining_balance.low_u128().saturated_into(),
					);
				} else {
					C::deposit_creating(
						&account_id,
						diff_balance.low_u128().unique_saturated_into(),
					);
					RB::inc_remaining_balance(
						&account_id,
						diff_remaining_balance.low_u128().saturated_into(),
					);
				}
			}
			_ => return,
		}

		// Handle existential deposit
		let ring_existential_deposit: u128 =
			<T as Config>::RingCurrency::minimum_balance().saturated_into::<u128>();
		let kton_existential_deposit: u128 =
			<T as Config>::KtonCurrency::minimum_balance().saturated_into::<u128>();
		let ring_existential_deposit = U256::from(ring_existential_deposit) * helper;
		let kton_existential_deposit = U256::from(kton_existential_deposit) * helper;

		let ring_account = T::RingAccountBasic::account_basic(address);
		let kton_account = T::KtonAccountBasic::account_basic(address);
		if ring_account.balance < ring_existential_deposit
			&& kton_account.balance < kton_existential_deposit
		{
			<RingRemainBalance as RemainBalanceOp<T, RingBalance<T>>>::remove_remaining_balance(
				&account_id,
			);
			<KtonRemainBalance as RemainBalanceOp<T, KtonBalance<T>>>::remove_remaining_balance(
				&account_id,
			);
		}
	}

	fn transfer(source: &H160, target: &H160, value: U256) -> Result<(), ExitError> {
		let source_account = Self::account_basic(source);
		ensure!(source_account.balance >= value, ExitError::OutOfGas);
		let new_source_balance = source_account.balance.saturating_sub(value);
		Self::mutate_account_basic(
			source,
			EVMAccount {
				nonce: source_account.nonce,
				balance: new_source_balance,
			},
		);

		let target_account = Self::account_basic(target);
		let new_target_balance = target_account.balance.saturating_add(value);
		Self::mutate_account_basic(
			target,
			EVMAccount {
				nonce: target_account.nonce,
				balance: new_target_balance,
			},
		);

		Ok(())
	}
}
