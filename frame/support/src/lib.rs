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

#![cfg_attr(not(feature = "std"), no_std)]

pub mod macros;
pub mod structs;
pub mod testing;
pub mod traits;

pub mod balance {
	pub mod lock {
		// --- darwinia ---
		pub use crate::structs::{BalanceLock, LockFor, LockReasons, StakingLock, Unbonding};
		pub use crate::traits::{
			LockIdentifier, LockableCurrency, VestingSchedule, WithdrawReasons,
		};
	}

	// --- darwinia ---
	pub use crate::structs::FrozenBalance;
	pub use crate::traits::{BalanceInfo, DustCollector, OnUnbalancedKton};
}

pub mod utilities {
	// --- substrate ---
	use frame_support::storage::{self, TransactionOutcome};
	use sp_runtime::DispatchError;

	// Due to substrate version
	// Copy from https://github.com/open-web3-stack/open-runtime-module-library/blob/master/utilities/src/lib.rs#L22
	/// Execute the supplied function in a new storage transaction.
	///
	/// All changes to storage performed by the supplied function are discarded if
	/// the returned outcome is `Result::Err`.
	///
	/// Transactions can be nested to any depth. Commits happen to the parent
	/// transaction.
	pub fn with_transaction_result<R>(
		f: impl FnOnce() -> Result<R, DispatchError>,
	) -> Result<R, DispatchError> {
		storage::with_transaction(|| {
			let res = f();
			if res.is_ok() {
				TransactionOutcome::Commit(res)
			} else {
				TransactionOutcome::Rollback(res)
			}
		})
	}
}

#[cfg(test)]
mod tests;
