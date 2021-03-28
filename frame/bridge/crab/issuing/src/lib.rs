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

pub use pallet::*;
pub use weights::WeightInfo;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	pub mod types {
		// --- darwinia ---
		use super::*;

		// Generic type
		pub type AccountId<T> = <T as frame_system::Config>::AccountId;
		pub type RingBalance<T> = <RingCurrency<T> as Currency<AccountId<T>>>::Balance;
		type RingCurrency<T> = <T as Config>::RingCurrency;
		// Simple type
		pub type MappedRing = u128;
	}
	pub use types::*;

	// --- substrate ---
	use frame_support::{
		pallet_prelude::*,
		traits::{Currency, Get},
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::{traits::AccountIdConversion, ModuleId};
	// --- darwinia ---
	use crate::weights::WeightInfo;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		// --- substrate ---
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type WeightInfo: WeightInfo;
		// --- darwinia ---
		#[pallet::constant]
		type ModuleId: Get<ModuleId>;
		type RingCurrency: Currency<AccountId<Self>>;
	}

	#[pallet::event]
	pub enum Event<T: Config> {
		/// Dummy Event. \[who, swapped *CRING*, burned Mapped *RING*\]
		DummyEvent(AccountId<T>, RingBalance<T>, MappedRing),
	}

	#[pallet::error]
	pub enum Error<T> {}

	#[pallet::storage]
	#[pallet::getter(fn total_mapped_ring)]
	pub type TotalMappedRing<T: Config> = StorageValue<_, MappedRing>;

	#[cfg_attr(feature = "std", derive(Default))]
	#[pallet::genesis_config]
	pub struct GenesisConfig {
		pub total_mapped_ring: MappedRing,
	}
	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {
			let _ = T::RingCurrency::make_free_balance_be(
				&T::ModuleId::get().into_account(),
				T::RingCurrency::minimum_balance(),
			);

			<TotalMappedRing<T>>::put(self.total_mapped_ring);
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}
	#[pallet::call]
	impl<T: Config> Pallet<T> {}
}

pub mod migration {
	const OLD_PALLET_NAME: &[u8] = b"DarwiniaCrabIssuing";

	pub fn migrate(new_pallet_name: &[u8]) {
		frame_support::migration::move_pallet(OLD_PALLET_NAME, new_pallet_name);
	}
}
