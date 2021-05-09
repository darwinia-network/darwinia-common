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

//! # Tron Backing Module

#![cfg_attr(not(feature = "std"), no_std)]

pub mod weights;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
	pub mod types {
		// --- darwinia ---
		#[cfg(feature = "std")]
		use crate::pallet::*;

		// Generic type
		pub type AccountId<T> = <T as frame_system::Config>::AccountId;
		#[cfg(feature = "std")]
		pub type RingBalance<T> = <RingCurrency<T> as Currency<AccountId<T>>>::Balance;
		#[cfg(feature = "std")]
		pub type KtonBalance<T> = <KtonCurrency<T> as Currency<AccountId<T>>>::Balance;
		#[cfg(feature = "std")]
		type RingCurrency<T> = <T as Config>::RingCurrency;
		#[cfg(feature = "std")]
		type KtonCurrency<T> = <T as Config>::KtonCurrency;
	}
	pub use types::*;

	// --- substrate ---
	use frame_support::{
		pallet_prelude::*,
		traits::{Currency, Get},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::AccountIdConversion;
	// --- darwinia ---
	use crate::weights::WeightInfo;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		// --- substrate ---
		type WeightInfo: WeightInfo;
		// --- darwinia ---
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		type RingCurrency: Currency<AccountId<Self>>;
		type KtonCurrency: Currency<AccountId<Self>>;
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub backed_ring: RingBalance<T>,
		pub backed_kton: KtonBalance<T>,
	}
	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig {
				backed_ring: Default::default(),
				backed_kton: Default::default(),
			}
		}
	}
	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			let module_account = <Pallet<T>>::account_id();

			let _ = T::RingCurrency::make_free_balance_be(
				&module_account,
				T::RingCurrency::minimum_balance() + self.backed_ring,
			);
			let _ = T::KtonCurrency::make_free_balance_be(&module_account, self.backed_kton);
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
			T::PalletId::get().into_account()
		}
	}
}
pub use pallet::*;
