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
		type MinimumPrice: Get<Price>;
		type CandidatePriceNumber: Get<u64>;

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
		/// Cancel relayer register
		CancelRelayerRegister(T::AccountId),
		/// Update relayer price
		SubmitRelayerPrice(T::AccountId, Price),
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
		/// Only Relayer can submit price
		InvalidSubmitPriceOrigin,
		/// The price is lower than MinimumPrice
		TooLowPrice,
	}

	// Relayer Storage
	#[pallet::storage]
	#[pallet::getter(fn get_locked_ring)]
	pub type LockedRing<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, RingBalance<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn relayers)]
	pub type Relayers<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

	// Price Storage
	#[pallet::storage]
	#[pallet::getter(fn get_relayer_prices)]
	pub type RelayerPrices<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, Price, ValueQuery>;

	/// The Target price given to relayer
	#[pallet::storage]
	#[pallet::getter(fn get_target_price)]
	pub type TargetPrice<T: Config> = StorageValue<_, Price, ValueQuery>;

	/// The lowest three prices, p.0 < p.1 < p.2
	#[pallet::storage]
	#[pallet::getter(fn get_candidate_prices)]
	pub type CandidatePrices<T: Config> = StorageValue<_, Vec<(T::AccountId, Price)>, ValueQuery>;

	/// The prices list
	#[pallet::storage]
	#[pallet::getter(fn get_prices)]
	pub type Prices<T: Config> = StorageValue<_, Vec<Price>, ValueQuery>;

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
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

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
			Self::deposit_event(Event::<T>::CancelRelayerRegister(who));
			Ok(().into())
		}

		/// The relayer submit price
		#[pallet::weight(10000)]
		#[transactional]
		pub fn submit_price(origin: OriginFor<T>, p: Price) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			ensure!(
				<Relayers<T>>::get().contains(&who),
				<Error<T>>::InvalidSubmitPriceOrigin
			);
			ensure!(p >= T::MinimumPrice::get(), <Error<T>>::TooLowPrice);

			Self::handle_price(&who, p);
			Self::deposit_event(Event::<T>::SubmitRelayerPrice(who, p));
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn handle_price(who: &T::AccountId, p: Price) {
		<RelayerPrices<T>>::insert(&who, p);
		<Prices<T>>::append(&p);

		// let mut candidate_relayers = Vec::with_capacity(T::CandidatePriceNumber::get() as usize);
		let mut prices = Self::get_prices();
		prices.sort();
		if prices.len() >= T::CandidatePriceNumber::get() as usize {
			<TargetPrice<T>>::put(prices[T::CandidatePriceNumber::get() as usize - 1]);
			// something need to check again and again
			for (id, p) in <RelayerPrices<T>>::iter() {
				for i in 0..T::CandidatePriceNumber::get() as usize {}
				// if p == prices[0] {
				// 	res.insert(0, (key, item));
				// } else if item == prices[1] {
				// 	res.insert(1, (key, item));
				// } else if item == prices[2] {
				// 	res.insert(2, (key, item));
				// }
			}
		}

		// <CandidatePrices<T>>::append(res.get(0).unwrap());
		// <CandidatePrices<T>>::append(res.get(1).unwrap());
		// <CandidatePrices<T>>::append(res.get(2).unwrap());
	}

	pub fn slash_relayer() {
		// slash relayers

		// if the lock ring lower than limit, remove it auto
		todo!()
	}
}

use sp_std::cmp::Ordering;

pub struct RelayerPrice<T: Config> {
	id: T::AccountId,
	price: Price,
}

impl<T: Config> PartialOrd for RelayerPrice<T> {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		self.price.partial_cmp(&other.price)
	}
}

impl<T: Config> PartialEq for RelayerPrice<T> {
	fn eq(&self, other: &Self) -> bool {
		self.price == other.price
	}
}
