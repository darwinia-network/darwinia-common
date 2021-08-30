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

use darwinia_support::balance::{LockFor, LockableCurrency};
use frame_support::{
	ensure,
	pallet_prelude::*,
	traits::{Currency, Get, LockIdentifier, WithdrawReasons},
	transactional, PalletId,
};
use frame_system::{ensure_signed, pallet_prelude::*};
use sp_runtime::traits::AccountIdConversion;

pub type AccountId<T> = <T as frame_system::Config>::AccountId;
pub type Balance = u128;
pub type Price = u64;
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

		#[pallet::constant]
		type MiniumLockValue: Get<RingBalance<Self>>;
		#[pallet::constant]
		type MinimumPrice: Get<RingBalance<Self>>;

		type LockId: Get<LockIdentifier>;
		type RingCurrency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

		type WeightInfo: WeightInfo;
	}

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	#[pallet::metadata(T::AccountId = "AccountId")]
	pub enum Event<T: Config> {
		/// Lock some RING and register to be relayer
		RegisterAndLockRing(T::AccountId, RingBalance<T>),
		/// Update lock value
		UpdateLockedRing(T::AccountId, RingBalance<T>),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Insufficient balance
		InsufficientBalance,
		/// The lock value is lower than MiniumLockLimit
		TooLowLockValue,
		/// The relayer has been registered
		AlreadyRegistered,
		/// Register before update lock value
		RegisterBeforeUpdateLock,
		/// Invalid new lock value
		InvalidNewLockValue,
	}

	#[pallet::storage]
	#[pallet::getter(fn get_locked_ring)]
	pub type LockedRing<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, RingBalance<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn relayers)]
	pub type Relayers<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_expected_price)]
	pub type SubmitedPrice<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, Price, ValueQuery>;

	#[pallet::storage]
	pub type TargetRelayPrice<T: Config> = StorageValue<_, Price>;

	/// p1 < p2 < p3, TODO: A better comments
	#[pallet::storage]
	pub type LowestPrices<T: Config> = StorageValue<_, (Price, Price, Price)>;

	#[pallet::storage]
	pub type PricesList<T: Config> = StorageValue<_, Vec<Price>>;

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
		/// Before the relayer transfer msgs, they need lock some rings.
		#[pallet::weight(10000)]
		#[transactional]
		pub fn register_and_lock_ring(
			origin: OriginFor<T>,
			lock_value: RingBalance<T>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			ensure!(
				lock_value >= T::MiniumLockValue::get(),
				<Error<T>>::TooLowLockValue
			);
			ensure!(
				T::RingCurrency::free_balance(&who) >= lock_value,
				<Error<T>>::InsufficientBalance
			);
			ensure!(
				!<Relayers<T>>::get().contains(&who),
				<Error<T>>::AlreadyRegistered
			);

			T::RingCurrency::set_lock(
				T::LockId::get(),
				&who,
				LockFor::Common { amount: lock_value },
				WithdrawReasons::all(),
			);
			<LockedRing<T>>::insert(&who, lock_value);
			<Relayers<T>>::append(&who);
			Self::deposit_event(Event::<T>::RegisterAndLockRing(who, lock_value));
			Ok(().into())
		}

		#[pallet::weight(10000)]
		#[transactional]
		pub fn update_locked_ring(
			origin: OriginFor<T>,
			new_lock: RingBalance<T>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			ensure!(
				<Relayers<T>>::get().contains(&who),
				<Error<T>>::RegisterBeforeUpdateLock
			);
			ensure!(
				T::RingCurrency::free_balance(&who) >= new_lock,
				<Error<T>>::InsufficientBalance
			);
			ensure!(
				new_lock > Self::get_locked_ring(&who),
				<Error<T>>::InvalidNewLockValue
			);

			T::RingCurrency::extend_lock(T::LockId::get(), &who, new_lock, WithdrawReasons::all())?;
			LockedRing::<T>::insert(who.clone(), new_lock);
			Self::deposit_event(Event::<T>::UpdateLockedRing(who, new_lock));
			Ok(().into())
		}

		/// Provide a way to cancel the registation and unlock asset
		#[pallet::weight(10000)]
		#[transactional]
		pub fn cancel_register(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			ensure!(
				<Relayers<T>>::get().contains(&who),
				<Error<T>>::RegisterBeforeUpdateLock
			);

			T::RingCurrency::remove_lock(T::LockId::get(), &who);
			LockedRing::<T>::remove(who.clone());
			Relayers::<T>::mutate(|relayers| relayers.retain(|x| *x != who));
			Ok(().into())
		}

		/// The relayers can submit a expect fee price
		#[pallet::weight(10000)]
		#[transactional]
		pub fn submit_expected_price(origin: OriginFor<T>, p: Price) -> DispatchResultWithPostInfo {
			let user = ensure_signed(origin)?;

			// update fee list and do sort work

			Ok(().into())
		}

		/// Allow relayers to update price while relaying
		#[pallet::weight(10000)]
		#[transactional]
		pub fn update_expected_price(origin: OriginFor<T>, p: Price) -> DispatchResultWithPostInfo {
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
