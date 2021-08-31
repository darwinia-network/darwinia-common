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
use sp_std::{
	cmp::{Ord, Ordering},
	default::Default,
	vec::Vec,
};

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
		type PriorRelayersNumber: Get<u64>;

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
	#[pallet::getter(fn get_relayer)]
	pub type RelayersMap<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, Relayer<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn relayers)]
	pub type Relayers<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

	// Price Storage
	/// The lowest n prices, p.0 < p.1 < p.2 ... < p.n
	#[pallet::storage]
	#[pallet::getter(fn prior_relayers)]
	pub type PriorRelayers<T: Config> = StorageValue<_, Vec<(T::AccountId, Price)>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn top_relayer)]
	pub type TopRelayer<T: Config> = StorageValue<_, (T::AccountId, Price), ValueQuery>;

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
		/// Register to be a relayer
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
			ensure!(!Self::is_registered(&who), <Error<T>>::AlreadyRegistered);
			if let Some(p) = price {
				ensure!(p >= T::MinimumPrice::get(), <Error<T>>::TooLowPrice);
			}

			let price = price.unwrap_or_else(T::MinimumPrice::get);
			T::RingCurrency::set_lock(
				T::LockId::get(),
				&who,
				LockFor::Common { amount: lock_value },
				WithdrawReasons::all(),
			);

			<RelayersMap<T>>::insert(&who, Relayer::new(who.clone(), lock_value, price));
			<Relayers<T>>::append(who.clone());

			Self::update_relayer_prices()?;
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
				Self::is_registered(&who),
				<Error<T>>::RegisterBeforeUpdateLock
			);
			ensure!(
				T::RingCurrency::free_balance(&who) >= new_lock,
				<Error<T>>::InsufficientBalance
			);
			ensure!(
				new_lock > Self::get_relayer(&who).lock_balance,
				<Error<T>>::InvalidNewLockValue
			);

			T::RingCurrency::extend_lock(T::LockId::get(), &who, new_lock, WithdrawReasons::all())?;
			<RelayersMap<T>>::mutate(who.clone(), |relayer| {
				relayer.lock_balance = new_lock;
			});
			Self::deposit_event(Event::<T>::UpdateLockedRing(who, new_lock));
			Ok(().into())
		}

		/// Provide a way to cancel the registation and unlock asset
		#[pallet::weight(10000)]
		#[transactional]
		pub fn cancel_register(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			ensure!(
				Self::is_registered(&who),
				<Error<T>>::RegisterBeforeUpdateLock
			);

			T::RingCurrency::remove_lock(T::LockId::get(), &who);
			RelayersMap::<T>::remove(who.clone());
			Relayers::<T>::mutate(|relayers| relayers.retain(|x| x != &who));

			Self::update_relayer_prices()?;
			Self::deposit_event(Event::<T>::CancelRelayerRegister(who));
			Ok(().into())
		}

		/// The relayer submit price
		#[pallet::weight(10000)]
		#[transactional]
		pub fn update_price(origin: OriginFor<T>, p: Price) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			ensure!(
				Self::is_registered(&who),
				<Error<T>>::InvalidSubmitPriceOrigin
			);
			ensure!(p >= T::MinimumPrice::get(), <Error<T>>::TooLowPrice);

			<RelayersMap<T>>::mutate(who.clone(), |relayer| {
				relayer.price = p;
			});

			Self::update_relayer_prices()?;
			Self::deposit_event(Event::<T>::SubmitRelayerPrice(who, p));
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn update_relayer_prices() -> Result<(), DispatchError> {
		// Update candidate price list when relayers submit new price
		<PriorRelayers<T>>::kill();

		let mut relayers: Vec<Relayer<T>> = <Relayers<T>>::get()
			.iter()
			.map(RelayersMap::<T>::get)
			.collect();
		relayers.sort();

		// If the registered relayers number >= the PriorRelayersNumber,
		// append the lowest PriorRelayersNumber relayers to PriorRelayers and choose the last one as TopRelayer.
		if relayers.len() >= T::PriorRelayersNumber::get() as usize {
			for i in 0..T::PriorRelayersNumber::get() as usize {
				let r = &relayers[i];
				<PriorRelayers<T>>::append((r.id.clone(), r.price));
			}
		} else {
			// If the registered relayers number < the PriorRelayersNumber,
			// append all submit price to PriorRelayers and choose the last one as TopRelayer
			for r in relayers.iter() {
				<PriorRelayers<T>>::append((r.id.clone(), r.price));
			}
		}
		<TopRelayer<T>>::put(
			<PriorRelayers<T>>::get()
				.iter()
				.last()
				.map(|(r, p)| ((*r).clone(), *p))
				.unwrap_or_default(),
		);
		Ok(())
	}

	/// Whether the account is a registered relayer
	pub fn is_registered(who: &T::AccountId) -> bool {
		<Relayers<T>>::get().iter().any(|r| *r == *who)
	}

	// Get relayer price
	pub fn relayer_price(who: &T::AccountId) -> Price {
		Self::get_relayer(who).price
	}

	// Get relayer locked balance
	pub fn relayer_locked_balance(who: &T::AccountId) -> RingBalance<T> {
		Self::get_relayer(who).lock_balance
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
	lock_balance: RingBalance<T>,
	price: Price,
}

impl<T: Config> Relayer<T> {
	pub fn new(id: T::AccountId, lock_balance: RingBalance<T>, price: Price) -> Relayer<T> {
		Relayer {
			id,
			lock_balance,
			price,
		}
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
		self.price == other.price && self.id == other.id && self.lock_balance == other.lock_balance
	}
}

impl<T: Config> Default for Relayer<T> {
	fn default() -> Self {
		Relayer {
			id: T::AccountId::default(),
			lock_balance: RingBalance::<T>::default(),
			price: 0,
		}
	}
}
