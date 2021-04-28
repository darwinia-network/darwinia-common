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
pub use weights::WeightInfo;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	pub mod types {
		// --- darwinia ---
		use crate::pallet::*;

		// Simple type
		pub type MappedRing = u128;
		// Generic type
		pub type AccountId<T> = <T as frame_system::Config>::AccountId;
		pub type RingBalance<T> = <RingCurrency<T> as Currency<AccountId<T>>>::Balance;
		type RingCurrency<T> = <T as Config>::RingCurrency;
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
		type WeightInfo: WeightInfo;
		// --- darwinia ---
		#[pallet::constant]
		type ModuleId: Get<ModuleId>;
		type RingCurrency: Currency<AccountId<Self>>;
	}

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
				&<Pallet<T>>::account_id(),
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
	impl<T: Config> Pallet<T> {
		pub fn account_id() -> T::AccountId {
			T::ModuleId::get().into_account()
		}
	}
}
pub use pallet::*;

pub mod migration {
	const OLD_PALLET_NAME: &[u8] = b"DarwiniaCrabIssuing";

	#[cfg(feature = "try-runtime")]
	pub mod try_runtime {
		// --- substrate ---
		use frame_support::{pallet_prelude::*, traits::StorageInstance};
		// --- darwinia ---
		use crate::*;

		macro_rules! generate_storage_types {
			($prefix:expr, $name:ident => Value<$value:ty>) => {
				paste::paste! {
					type $name = StorageValue<[<$name Instance>], $value, ValueQuery>;

					struct [<$name Instance>];
					impl StorageInstance for [<$name Instance>] {
						const STORAGE_PREFIX: &'static str = "TotalMappedRing";

						fn pallet_prefix() -> &'static str { $prefix }
					}
				}
			};
		}

		generate_storage_types!("DarwiniaCrabIssuing", OldTotalMappedRing => Value<()>);
		generate_storage_types!("CrabIssuing", NewTotalMappedRing => Value<()>);

		pub fn pre_migrate<T: Config>() -> Result<(), &'static str> {
			log::info!(
				"OldTotalMappedRing.exits()? {:?}",
				OldTotalMappedRing::exists()
			);
			log::info!(
				"NewTotalMappedRing.exits()? {:?}",
				NewTotalMappedRing::exists()
			);

			assert!(OldTotalMappedRing::exists());
			assert!(!NewTotalMappedRing::exists());

			log::info!("Migrating `DarwiniaCrabIssuing` to `CrabIssuing`...");
			migration::migrate(b"CrabIssuing");

			log::info!(
				"OldTotalMappedRing.exits()? {:?}",
				OldTotalMappedRing::exists()
			);
			log::info!(
				"NewTotalMappedRing.exits()? {:?}",
				NewTotalMappedRing::exists()
			);

			assert!(!OldTotalMappedRing::exists());
			assert!(NewTotalMappedRing::exists());

			Ok(())
		}
	}

	pub fn migrate(new_pallet_name: &[u8]) {
		frame_support::migration::move_pallet(OLD_PALLET_NAME, new_pallet_name);
	}
}
