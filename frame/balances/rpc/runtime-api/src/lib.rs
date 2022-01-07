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

//! Runtime API definition required by balances RPC extensions.
//!
//! This API should be imported and implemented by the runtime,
//! of a node that wants to use the custom RPC extension
//! adding balances access methods.

#![cfg_attr(not(feature = "std"), no_std)]

// --- core ---
use core::fmt::Debug;
// --- crates.io ---
use codec::{Codec, Decode, Encode};
// --- paritytech ---
use sp_api::decl_runtime_apis;
use sp_runtime::traits::{MaybeDisplay, MaybeFromStr};
// --- darwinia-network ---
use darwinia_support::impl_runtime_dispatch_info;

impl_runtime_dispatch_info! {
	struct RuntimeDispatchInfo<Balance> {
		usable_balance: Balance
	}
}

decl_runtime_apis! {
	pub trait BalancesApi<AccountId, Balance>
	where
		AccountId: Codec,
		Balance: Debug + Codec + MaybeDisplay + MaybeFromStr,
	{
		fn usable_balance(instance: u8, who: AccountId) -> RuntimeDispatchInfo<Balance>;
	}
}
