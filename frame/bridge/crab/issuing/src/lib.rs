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

//! # Crab Issuing Module

#![cfg_attr(not(feature = "std"), no_std)]

pub mod weights;
// --- darwinia ---
pub use weights::WeightInfo;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

mod types {
	// --- darwinia ---
	use crate::*;

	pub type MappedRing = u128;

	pub type AccountId<T> = <T as frame_system::Config>::AccountId;

	pub type RingBalance<T> = <RingCurrency<T> as Currency<AccountId<T>>>::Balance;

	type RingCurrency<T> = <T as Config>::RingCurrency;
}

// --- substrate ---
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage,
	traits::{Currency, Get},
};
use sp_runtime::{traits::AccountIdConversion, ModuleId};
// --- darwinia ---
use types::*;

pub trait Config: frame_system::Config {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

	type ModuleId: Get<ModuleId>;

	type RingCurrency: Currency<AccountId<Self>>;

	type WeightInfo: WeightInfo;
}

decl_event! {
	pub enum Event<T>
	where
		AccountId = AccountId<T>,
		RingBalance = RingBalance<T>,
	{
		/// Dummy Event. [who, swapped *CRING*, burned Mapped *RING*]
		DummyEvent(AccountId, RingBalance, MappedRing),
	}
}

decl_error! {
	pub enum Error for Module<T: Config> {
	}
}

decl_storage! {
	trait Store for Module<T: Config> as DarwiniaCrabIssuing {
		pub TotalMappedRing get(fn total_mapped_ring) config(): MappedRing;
	}

	add_extra_genesis {
		build(|config| {
			let _ = T::RingCurrency::make_free_balance_be(
				&<Module<T>>::account_id(),
				T::RingCurrency::minimum_balance(),
			);

			TotalMappedRing::put(config.total_mapped_ring);
		});
	}
}

decl_module! {
	pub struct Module<T: Config> for enum Call
	where
		origin: T::Origin
	{
		type Error = Error<T>;

		const ModuleId: ModuleId = T::ModuleId::get();

		fn deposit_event() = default;
	}
}

impl<T: Config> Module<T> {
	pub fn account_id() -> T::AccountId {
		T::ModuleId::get().into_account()
	}
}
