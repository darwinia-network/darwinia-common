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

use codec::{Decode, Encode};
use darwinia_support::balance::{LockFor, LockableCurrency};
use frame_support::{
	dispatch::DispatchError,
	ensure,
	pallet_prelude::*,
	traits::{Currency, Get, LockIdentifier, WithdrawReasons},
	transactional, PalletId,
};
use frame_system::{ensure_signed, pallet_prelude::*};
use sp_std::cmp::{Ord, Ordering};

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
	pub type Relayers<T: Config> = StorageValue<_, Vec<Relayer<T>>, ValueQuery>;

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
		pub fn register(
			origin: OriginFor<T>,
			lock_value: RingBalance<T>,
			price: Option<Price>,
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
				<Relayers<T>>::get().iter().find(|r| r.id == who).is_none(),
				<Error<T>>::AlreadyRegistered
			);
			if let Some(p) = price {
				ensure!(p >= T::MinimumPrice::get(), <Error<T>>::TooLowPrice);
			}

			let price = price.unwrap_or(T::MinimumPrice::get());
			T::RingCurrency::set_lock(
				T::LockId::get(),
				&who,
				LockFor::Common { amount: lock_value },
				WithdrawReasons::all(),
			);
			<LockedRing<T>>::insert(&who, lock_value);
			<Relayers<T>>::append(Relayer::new(who.clone(), price));
			Self::update_relayer_prices(who.clone(), price)?;
			Self::deposit_event(Event::<T>::RegisterAndLockRing(who, lock_value));
			Ok(().into())
		}

		#[pallet::weight(10000)]
		#[transactional]
		pub fn update_locked_balance(
			origin: OriginFor<T>,
			new_lock: RingBalance<T>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			ensure!(
				<Relayers<T>>::get().iter().find(|r| r.id == who).is_some(),
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
				<Relayers<T>>::get().iter().find(|r| r.id == who).is_some(),
				<Error<T>>::RegisterBeforeUpdateLock
			);

			T::RingCurrency::remove_lock(T::LockId::get(), &who);
			LockedRing::<T>::remove(who.clone());
			Relayers::<T>::mutate(|relayers| relayers.retain(|x| x.id != who));
			Self::deposit_event(Event::<T>::CancelRelayerRegister(who));
			Ok(().into())
		}

		/// The relayer submit price
		#[pallet::weight(10000)]
		#[transactional]
		pub fn submit_price(origin: OriginFor<T>, p: Price) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			ensure!(
				<Relayers<T>>::get().iter().find(|r| r.id == who).is_some(),
				<Error<T>>::InvalidSubmitPriceOrigin
			);
			ensure!(p >= T::MinimumPrice::get(), <Error<T>>::TooLowPrice);

			Self::update_relayer_prices(who.clone(), p)?;
			Self::deposit_event(Event::<T>::SubmitRelayerPrice(who, p));
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn update_relayer_prices(who: T::AccountId, p: Price) -> Result<(), DispatchError> {
		<RelayerPrices<T>>::insert(&who, p);
		<Relayers<T>>::mutate(|relayers| {
			relayers
				.into_iter()
				.find(|relayer| relayer.id == who)
				.map(|r| r.price = p)
				.ok_or_else(|| <Error<T>>::InvalidSubmitPriceOrigin)
		})?;
		// Update candidate price list when relayers submit new price
		<CandidatePrices<T>>::kill();

		let mut relayers = Self::relayers();
		relayers.sort();
		println!("bear: relayers = {:?}", relayers.len());
		// If the submit price relayer number is larger than the CandidatePriceNumber,
		// append the lowest candidate number to CandidatePrices and choose the last one as TargetPrice.
		if relayers.len() >= T::CandidatePriceNumber::get() as usize {
			for i in 0..T::CandidatePriceNumber::get() as usize {
				let r = &relayers[i];
				<CandidatePrices<T>>::append((r.id.clone(), r.price));
			}
			<TargetPrice<T>>::put(relayers[(T::CandidatePriceNumber::get() - 1) as usize].price);
		} else {
			// If the submit price relayer number lower than the CandidatePriceNumber,
			// append all submit price to CandidatePrices and choose the last one as TargetPrice
			for i in 0..relayers.len() {
				let r = &relayers[i];
				<CandidatePrices<T>>::append((r.id.clone(), r.price));
			}
			<TargetPrice<T>>::put(relayers[relayers.len() - 1].price);
		}
		Ok(())
	}

	/// Whether the account is a registered relayer
	pub fn is_relayer(who: T::AccountId) -> bool {
		<Relayers<T>>::get().iter().find(|r| r.id == who).is_some()
	}

	pub fn slash_relayer() {
		// slash relayers
		// if the lock ring lower than limit, remove it auto
		todo!()
	}
}
#[derive(Encode, Decode, Clone, Eq, Debug)]
pub struct Relayer<T: Config> {
	id: T::AccountId,
	price: Price,
}

impl<T: Config> Relayer<T> {
	pub fn new(id: T::AccountId, price: Price) -> Relayer<T> {
		Relayer { id, price }
	}
}

impl<T: Config> PartialOrd for Relayer<T> {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		self.price.partial_cmp(&other.price)
	}
}

impl<T: Config> Ord for Relayer<T> {
	fn cmp(&self, other: &Self) -> Ordering {
		self.price.cmp(&other.price)
	}
}

impl<T: Config> PartialEq for Relayer<T> {
	fn eq(&self, other: &Self) -> bool {
		self.price == other.price
	}
}
