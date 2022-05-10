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
use pallet_balances::Reasons;
use sp_runtime::{DispatchError, DispatchResult};
use sp_std::prelude::*;
// --- darwinia-network ---
use crate::structs::FrozenBalance;
use ethereum_primitives::receipt::EthereumTransactionIndex;

pub trait BalanceInfo<Balance, Module>: MaxEncodedLen {
	fn free(&self) -> Balance;
	fn set_free(&mut self, new_free: Balance);

	fn reserved(&self) -> Balance;
	fn set_reserved(&mut self, new_reserved: Balance);

	/// The total balance in this account including any that is reserved and ignoring any frozen.
	fn total(&self) -> Balance;

	/// How much this account's balance can be reduced for the given `reasons`.
	fn usable(&self, reasons: Reasons, frozen_balance: FrozenBalance<Balance>) -> Balance;
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
