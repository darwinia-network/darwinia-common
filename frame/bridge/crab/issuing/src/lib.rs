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

pub use pallet::*;

pub mod weights;
// --- darwinia ---
pub use weights::WeightInfo;
// --- substrate ---
use frame_support::traits::Currency;

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

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_support::traits::{Currency, Get};
	use frame_system::pallet_prelude::*;
	use sp_runtime::{traits::AccountIdConversion, ModuleId};
	use types::*;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		#[pallet::constant]
		type ModuleId: Get<ModuleId>;

		type RingCurrency: Currency<AccountId<Self>>;

		type WeightInfo: WeightInfo;

		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
	}

	// Define the pallet struct placeholder, various pallet function are implemented on it.
	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::error]
	pub enum Error<T> {}

	#[pallet::event]
	pub enum Event<T: Config> {
		/// Dummy Event. [who, swapped *CRING*, burned Mapped *RING*]
		DummyEvent(AccountId<T>, RingBalance<T>, MappedRing),
	}

	#[pallet::storage]
	#[pallet::getter(fn total_mapped_ring)]
	pub(super) type TotalMappedRing<T: Config> = StorageValue<_, MappedRing>;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T> {
		pub total_mapped_ring: MappedRing,
		pub phantom: std::marker::PhantomData<T>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self {
				total_mapped_ring: Default::default(),
				phantom: Default::default(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			let _ = T::RingCurrency::make_free_balance_be(
				&T::ModuleId::get().into_account(),
				T::RingCurrency::minimum_balance(),
			);

			<TotalMappedRing<T>>::put(self.total_mapped_ring);
		}
	}
}
