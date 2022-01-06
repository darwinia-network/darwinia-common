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

// --- core ---
use core::fmt::Debug;
// --- crates.io ---
use codec::{FullCodec, MaxEncodedLen};
use impl_trait_for_tuples::impl_for_tuples;
use scale_info::TypeInfo;
// --- paritytech ---
use frame_support::traits::{Currency, Get, LockIdentifier, WithdrawReasons};
use sp_runtime::{DispatchError, DispatchResult};
use sp_std::prelude::*;
// --- darwinia-network ---
use crate::structs::{FrozenBalance, LockFor, LockReasons};
use ethereum_primitives::receipt::EthereumTransactionIndex;

pub trait BalanceInfo<Balance, Module>: MaxEncodedLen {
	fn free(&self) -> Balance;
	fn set_free(&mut self, new_free: Balance);

	fn reserved(&self) -> Balance;
	fn set_reserved(&mut self, new_reserved: Balance);

	/// The total balance in this account including any that is reserved and ignoring any frozen.
	fn total(&self) -> Balance;

	/// How much this account's balance can be reduced for the given `reasons`.
	fn usable(&self, reasons: LockReasons, frozen_balance: FrozenBalance<Balance>) -> Balance;
}

/// A currency whose accounts can have liquidity restrictions.
pub trait LockableCurrency<AccountId>: Currency<AccountId> {
	/// The quantity used to denote time; usually just a `BlockNumber`.
	type Moment;

	/// The maximum number of locks a user should have on their account.
	type MaxLocks: Get<u32>;

	/// Create a new balance lock on account `who`.
	///
	/// If the new lock is valid (i.e. not already expired), it will push the struct to
	/// the `Locks` vec in storage. Note that you can lock more funds than a user has.
	///
	/// If the lock `id` already exists, this will update it.
	fn set_lock(
		id: LockIdentifier,
		who: &AccountId,
		lock_for: LockFor<Self::Balance, Self::Moment>,
		reasons: WithdrawReasons,
	);

	/// Changes a balance lock (selected by `id`) so that it becomes less liquid in all
	/// parameters or creates a new one if it does not exist.
	///
	/// Calling `extend_lock` on an existing lock `id` differs from `set_lock` in that it
	/// applies the most severe constraints of the two, while `set_lock` replaces the lock
	/// with the new parameters. As in, `extend_lock` will set:
	/// - maximum `amount`
	/// - bitwise mask of all `reasons`
	fn extend_lock(
		id: LockIdentifier,
		who: &AccountId,
		amount: Self::Balance,
		reasons: WithdrawReasons,
	) -> DispatchResult;

	/// Remove an existing lock.
	fn remove_lock(id: LockIdentifier, who: &AccountId);

	/// Get the balance of an account that can be used for transfers, reservations, or any other
	/// non-locking, non-transaction-fee activity. Will be at most `free_balance`.
	fn usable_balance(who: &AccountId) -> Self::Balance;

	/// Get the balance of an account that can be used for paying transaction fees (not tipping,
	/// or any other kind of fees, though). Will be at most `free_balance`.
	fn usable_balance_for_fees(who: &AccountId) -> Self::Balance;
}

pub trait DustCollector<AccountId> {
	fn is_dust(who: &AccountId) -> bool;

	fn collect(who: &AccountId);
}
#[impl_for_tuples(30)]
impl<AccountId> DustCollector<AccountId> for Currencies {
	fn is_dust(who: &AccountId) -> bool {
		for_tuples!( #(
			if !Currencies::is_dust(who) {
				return false;
			}
		)* );

		true
	}

	fn collect(who: &AccountId) {
		for_tuples!( #( Currencies::collect(who); )* );
	}
}

/// Callback on ethereum-backing module
pub trait OnDepositRedeem<AccountId, Balance> {
	fn on_deposit_redeem(
		backing: &AccountId,
		stash: &AccountId,
		amount: Balance,
		start_at: u64,
		months: u8,
	) -> DispatchResult;
}

pub trait EthereumReceipt<AccountId, Balance> {
	type EthereumReceiptProofThing: Clone + Debug + PartialEq + FullCodec + TypeInfo;

	fn account_id() -> AccountId;

	fn receipt_verify_fee() -> Balance;

	fn verify_receipt(
		proof: &Self::EthereumReceiptProofThing,
	) -> Result<ethereum_primitives::receipt::EthereumReceipt, DispatchError>;

	fn gen_receipt_index(proof: &Self::EthereumReceiptProofThing) -> EthereumTransactionIndex;
}
