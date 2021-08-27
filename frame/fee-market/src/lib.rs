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

//! # Fee Market Module

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "128"]

#[cfg(test)]
mod tests;

pub mod weights;
use crate::weights::WeightInfo;

use frame_support::{
	ensure,
	pallet_prelude::*,
	traits::{Currency, Get},
	transactional, PalletId,
};
use sp_runtime::traits::AccountIdConversion;

use frame_system::{ensure_signed, pallet_prelude::*};
pub type AccountId<T> = <T as frame_system::Config>::AccountId;
pub type Balance = u128;
pub type RingBalance<T> = <<T as Config>::RingCurrency as Currency<AccountId<T>>>::Balance;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type WeightInfo: WeightInfo;
		type RingCurrency: Currency<AccountId<Self>>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	#[pallet::metadata(T::AccountId = "AccountId")]
	pub enum Event<T: Config> {}

	#[pallet::error]
	pub enum Error<T> {}

	#[pallet::storage]
	#[pallet::getter(fn get_locked_ring)]
	pub type RegisterLocked<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, RingBalance<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_expected_price)]
	pub type SubmitedPrice<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, u64, ValueQuery>;

	#[pallet::storage]
	pub type TargetRelayPrice<T: Config> = StorageValue<_, u64>;

	/// p1, p2, p3, TODO: A better comments
	#[pallet::storage]
	pub type LowestPrices<T: Config> = StorageValue<_, (u64, u64, u64)>;

	#[pallet::genesis_config]
	pub struct GenesisConfig {}
	#[cfg(feature = "std")]
	impl Default for GenesisConfig {
		fn default() -> Self {
			Self {}
		}
	}
	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(_: T::BlockNumber) {
			// update the latest target price
		}
	}
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Before the relayer transfer msgs, they need lock asset.
		#[pallet::weight(10000)]
		#[transactional]
		pub fn register_and_lock_asset(
			origin: OriginFor<T>,
			// TODO: do we need to add lock time
			lock_amount: RingBalance<T>,
		) -> DispatchResultWithPostInfo {
			let user = ensure_signed(origin)?;

			// lock user lock amount
			Ok(().into())
		}
		// TODO: how to deal with the case, when relayer wants to inc/dec lock asset

		/// Provide a way to cancel the registation and unlock asset
		#[pallet::weight(10000)]
		#[transactional]
		pub fn cancel_register_and_unlock(
			origin: OriginFor<T>,
			lock_amount: RingBalance<T>,
		) -> DispatchResultWithPostInfo {
			let user = ensure_signed(origin)?;

			// lock user lock amount
			Ok(().into())
		}

		/// The relayers can submit a expect fee price
		#[pallet::weight(10000)]
		#[transactional]
		pub fn submit_expected_price(origin: OriginFor<T>, fee: u64) -> DispatchResultWithPostInfo {
			let user = ensure_signed(origin)?;

			// update fee list and do sort work

			Ok(().into())
		}

		/// Allow relayers to update price while relaying
		#[pallet::weight(10000)]
		#[transactional]
		pub fn update_expected_price(origin: OriginFor<T>, fee: u64) -> DispatchResultWithPostInfo {
			let user = ensure_signed(origin)?;

			// update fee list and do sort work

			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn pallet_account_id() -> T::AccountId {
		T::PalletId::get().into_account()
	}

	pub fn slash_relayer() {
		todo!()
	}
}

// TODO:
// 1. expose rpc to a estimate price
// 2. S(t) function
