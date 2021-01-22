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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

//! # Crab Backing Module

#![cfg_attr(not(feature = "std"), no_std)]

mod types {
	// --- darwinia ---
	#[cfg(feature = "std")]
	use crate::*;

	pub type AccountId<T> = <T as frame_system::Trait>::AccountId;

	#[cfg(feature = "std")]
	pub type RingBalance<T> = <RingCurrency<T> as Currency<AccountId<T>>>::Balance;

	#[cfg(feature = "std")]
	type RingCurrency<T> = <T as Trait>::RingCurrency;
}

// --- substrate ---
use frame_support::{
	decl_module, decl_storage,
	traits::{Currency, Get},
};
use sp_runtime::{traits::AccountIdConversion, ModuleId};
// --- darwinia ---
use types::*;

pub trait Trait: frame_system::Trait {
	type ModuleId: Get<ModuleId>;

	type RingCurrency: Currency<AccountId<Self>>;

	type WeightInfo: WeightInfo;
}

pub trait WeightInfo {}
impl WeightInfo for () {}

decl_storage! {
	trait Store for Module<T: Trait> as DarwiniaCrabBacking {}

	add_extra_genesis {
		config(backed_ring): RingBalance<T>;
		build(|config| {
			let _ = T::RingCurrency::make_free_balance_be(
				&<Module<T>>::account_id(),
				T::RingCurrency::minimum_balance() + config.backed_ring
			);
		});
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call
	where
		origin: T::Origin
	{
		const ModuleId: ModuleId = T::ModuleId::get();
	}
}

impl<T: Trait> Module<T> {
	pub fn account_id() -> T::AccountId {
		T::ModuleId::get().into_account()
	}
}
