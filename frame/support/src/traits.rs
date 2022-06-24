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
use codec::FullCodec;
use scale_info::TypeInfo;
// --- paritytech ---
use sp_runtime::{DispatchError, DispatchResult};
use sp_std::prelude::*;
// --- darwinia-network ---
use ethereum_primitives::receipt::EthereumTransactionIndex;

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
