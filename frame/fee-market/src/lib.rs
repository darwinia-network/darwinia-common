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

//! # Fee Market Pallet

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "128"]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod tests;

pub mod s2s;
pub mod weight;
pub use weight::WeightInfo;

// --- substrate ---
use bp_messages::{LaneId, MessageNonce};
use frame_support::{
	ensure,
	pallet_prelude::*,
	traits::{Currency, Get, LockIdentifier, WithdrawReasons},
	transactional, PalletId,
};
use frame_system::{ensure_signed, pallet_prelude::*};
use sp_runtime::{
	traits::{Saturating, UniqueSaturatedInto},
	Permill,
};
use sp_std::{default::Default, vec::Vec};
// --- darwinia-network ---
use darwinia_support::balance::{LockFor, LockableCurrency};
use dp_fee::{Order, Relayer};

pub type AccountId<T> = <T as frame_system::Config>::AccountId;
pub type RingBalance<T> = <<T as Config>::RingCurrency as Currency<AccountId<T>>>::Balance;
pub type Fee<T> = RingBalance<T>;

pub use pallet::*;
#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		/// Some reward goes to Treasury.
		#[pallet::constant]
		type TreasuryPalletId: Get<PalletId>;
		#[pallet::constant]
		type LockId: Get<LockIdentifier>;
		/// The minimum locked collateral for a fee market relayer, also represented as the maximum value for slash.
		#[pallet::constant]
		type MiniumLockCollateral: Get<RingBalance<Self>>;
		/// The minimum fee for relaying.
		#[pallet::constant]
		type MinimumRelayFee: Get<Fee<Self>>;
		#[pallet::constant]
		type MinRelayersNumber: Get<u64>;
		/// The slot times set
		#[pallet::constant]
		type SlotTime: Get<Self::BlockNumber>;

		/// Reward parameters
		#[pallet::constant]
		type AssignedRelayersRewardRatio: Get<Permill>;
		#[pallet::constant]
		type MessageRelayersRewardRatio: Get<Permill>;
		#[pallet::constant]
		type ConfirmRelayersRewardRatio: Get<Permill>;

		/// The slash rule
		type Slasher: Slasher<Self>;
		type RingCurrency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>
			+ Currency<Self::AccountId>;
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type WeightInfo: WeightInfo;
	}

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	#[pallet::metadata(T::AccountId = "AccountId")]
	pub enum Event<T: Config> {
		/// Relayer enrollment
		EnrollAndLockCollateral(T::AccountId, RingBalance<T>, Fee<T>),
		/// Update relayer locked collateral
		UpdateLockedCollateral(T::AccountId, RingBalance<T>),
		/// Update relayer fee
		UpdateRelayFee(T::AccountId, Fee<T>),
		/// Relayer cancel enrollment
		CancelEnrollment(T::AccountId),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Insufficient balance
		InsufficientBalance,
		/// The locked collateral is lower than MiniumLockLimit
		LockCollateralTooLow,
		/// The relayer has been enrolled
		AlreadyEnrolled,
		/// This relayer doesn't enroll ever
		NotEnrolled,
		/// Only increase lock collateral is allowed when update_locked_balance
		OnlyIncreaseLockedCollateralAllowed,
		/// The fee is lower than MinimumRelayFee
		RelayFeeTooLow,
		/// The enrolled relayers less than MIN_RELAYERS_NUMBER
		TooFewEnrolledRelayers,
		/// The relayer is occupied, and can't cancel enrollment now.
		OccupiedRelayer,
	}

	// Enrolled relayers storage
	#[pallet::storage]
	#[pallet::getter(fn get_relayer)]
	pub type RelayersMap<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Relayer<T::AccountId, RingBalance<T>>,
		ValueQuery,
	>;
	#[pallet::storage]
	#[pallet::getter(fn relayers)]
	pub type Relayers<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

	// Priority relayers storage
	#[pallet::storage]
	#[pallet::getter(fn assigned_relayers)]
	pub type AssignedRelayersStorage<T: Config> =
		StorageValue<_, Vec<Relayer<T::AccountId, RingBalance<T>>>, OptionQuery>;

	// Order storage
	#[pallet::storage]
	pub type Orders<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		(LaneId, MessageNonce),
		Order<T::AccountId, T::BlockNumber, Fee<T>>,
		OptionQuery,
	>;
	#[pallet::storage]
	pub type ConfirmedMessagesThisBlock<T: Config> =
		StorageValue<_, Vec<(LaneId, MessageNonce)>, ValueQuery>;

	#[pallet::pallet]
	pub struct Pallet<T>(_);
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(_: BlockNumberFor<T>) {
			// Clean the order's storage when the rewards has been paid off
			for (lane_id, message_nonce) in <ConfirmedMessagesThisBlock<T>>::get() {
				<Orders<T>>::remove((lane_id, message_nonce));
			}
			<ConfirmedMessagesThisBlock<T>>::kill();
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Any accounts can enroll to be a relayer by lock collateral. The relay fee is optional,
		/// the default value is MinimumRelayFee in runtime.
		/// Note: One account can enroll only once.
		#[pallet::weight(<T as Config>::WeightInfo::enroll_and_lock_collateral())]
		#[transactional]
		pub fn enroll_and_lock_collateral(
			origin: OriginFor<T>,
			lock_collateral: RingBalance<T>,
			relay_fee: Option<Fee<T>>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			ensure!(
				lock_collateral >= T::MiniumLockCollateral::get(),
				<Error<T>>::LockCollateralTooLow
			);
			ensure!(
				T::RingCurrency::free_balance(&who) >= lock_collateral,
				<Error<T>>::InsufficientBalance
			);
			ensure!(!Self::is_enrolled(&who), <Error<T>>::AlreadyEnrolled);
			if let Some(fee) = relay_fee {
				ensure!(fee >= T::MinimumRelayFee::get(), <Error<T>>::RelayFeeTooLow);
			}

			let fee = relay_fee.unwrap_or_else(T::MinimumRelayFee::get);
			T::RingCurrency::set_lock(
				T::LockId::get(),
				&who,
				LockFor::Common {
					amount: lock_collateral,
				},
				WithdrawReasons::all(),
			);

			<RelayersMap<T>>::insert(&who, Relayer::new(who.clone(), lock_collateral, fee));
			<Relayers<T>>::append(who.clone());

			Self::update_market();
			Self::deposit_event(Event::<T>::EnrollAndLockCollateral(
				who,
				lock_collateral,
				fee,
			));
			Ok(().into())
		}

		/// Update locked collateral for enrolled relayer, only supporting lock more.
		#[pallet::weight(<T as Config>::WeightInfo::update_locked_collateral())]
		#[transactional]
		pub fn update_locked_collateral(
			origin: OriginFor<T>,
			new_collateral: RingBalance<T>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_enrolled(&who), <Error<T>>::NotEnrolled);
			ensure!(
				T::RingCurrency::free_balance(&who) >= new_collateral,
				<Error<T>>::InsufficientBalance
			);
			ensure!(
				new_collateral > Self::get_relayer(&who).collateral,
				<Error<T>>::OnlyIncreaseLockedCollateralAllowed
			);

			Self::update_collateral(&who, new_collateral);
			Self::deposit_event(Event::<T>::UpdateLockedCollateral(who, new_collateral));
			Ok(().into())
		}

		/// Cancel enrolled relayer
		#[pallet::weight(<T as Config>::WeightInfo::cancel_enrollment())]
		#[transactional]
		pub fn cancel_enrollment(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_enrolled(&who), <Error<T>>::NotEnrolled);
			ensure!(
				<Relayers<T>>::get().len() > T::MinRelayersNumber::get() as usize,
				<Error<T>>::TooFewEnrolledRelayers
			);
			ensure!(!Self::is_occupied(&who), <Error<T>>::OccupiedRelayer);

			Self::remove_enrolled_relayer(&who);
			Self::deposit_event(Event::<T>::CancelEnrollment(who));
			Ok(().into())
		}

		/// Update relay fee for enrolled relayer
		#[pallet::weight(<T as Config>::WeightInfo::update_relay_fee())]
		#[transactional]
		pub fn update_relay_fee(
			origin: OriginFor<T>,
			relay_fee: Fee<T>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_enrolled(&who), <Error<T>>::NotEnrolled);
			ensure!(
				relay_fee >= T::MinimumRelayFee::get(),
				<Error<T>>::RelayFeeTooLow
			);

			<RelayersMap<T>>::mutate(who.clone(), |relayer| {
				relayer.fee = relay_fee;
			});

			Self::update_market();
			Self::deposit_event(Event::<T>::UpdateRelayFee(who, relay_fee));
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// An important update in this pallet, need to update market information in the following cases:
	///
	/// - When new relayer enroll
	/// - When enrolled relayer wants to update relaying fee
	/// - When enrolled relayer wants to cancel enrollment
	/// - When some enrolled relayer's collateral below MiniumLockCollateral, might trigger market update
	pub fn update_market() {
		let mut relayers: Vec<Relayer<T::AccountId, RingBalance<T>>> = <Relayers<T>>::get()
			.iter()
			.map(RelayersMap::<T>::get)
			.collect();
		relayers.sort();

		let prior_relayers_len = T::MinRelayersNumber::get() as usize;
		if relayers.len() >= prior_relayers_len {
			<AssignedRelayersStorage<T>>::kill();

			let mut prior_relayers = Vec::with_capacity(prior_relayers_len);
			for i in 0..prior_relayers_len {
				if let Some(r) = relayers.get(i) {
					prior_relayers.push(r);
				}
			}
			<AssignedRelayersStorage<T>>::put(prior_relayers);
		}
	}

	/// Update relayer locked collateral, it will changes RelayersMap storage
	pub fn update_collateral(who: &T::AccountId, new_collateral: RingBalance<T>) {
		if new_collateral < T::MiniumLockCollateral::get()
			&& <Relayers<T>>::get().len() > T::MinRelayersNumber::get() as usize
		{
			Self::remove_enrolled_relayer(who);
			return;
		}
		let _ = T::RingCurrency::extend_lock(
			T::LockId::get(),
			who,
			new_collateral,
			WithdrawReasons::all(),
		);
		<RelayersMap<T>>::mutate(who.clone(), |relayer| {
			relayer.collateral = new_collateral;
		});
		Self::update_market();
	}

	/// Remove enrolled relayer
	pub fn remove_enrolled_relayer(who: &T::AccountId) {
		T::RingCurrency::remove_lock(T::LockId::get(), who);
		<RelayersMap<T>>::remove(who.clone());
		<Relayers<T>>::mutate(|relayers| relayers.retain(|x| x != who));
		Self::update_market();
	}

	/// Whether the relayer has enrolled
	pub fn is_enrolled(who: &T::AccountId) -> bool {
		<Relayers<T>>::get().iter().any(|r| *r == *who)
	}

	/// Get relayer fee
	pub fn relayer_fee(who: &T::AccountId) -> Fee<T> {
		Self::get_relayer(who).fee
	}

	/// Get relayer locked collateral
	pub fn relayer_locked_collateral(who: &T::AccountId) -> RingBalance<T> {
		Self::get_relayer(who).collateral
	}

	/// Get market fee(P3), If the enrolled relayers less then MIN_RELAYERS_NUMBER, return NONE.
	pub fn market_fee() -> Option<Fee<T>> {
		Self::assigned_relayers().and_then(|relayers| relayers.last().map(|r| r.fee))
	}

	/// Get order info
	pub fn order(
		lane_id: &LaneId,
		message: &MessageNonce,
	) -> Option<Order<T::AccountId, T::BlockNumber, Fee<T>>> {
		<Orders<T>>::get((lane_id, message))
	}

	/// Whether the enrolled relayer is occupied(Responsible for order relaying)
	pub fn is_occupied(who: &T::AccountId) -> bool {
		for (_, order) in <Orders<T>>::iter() {
			if order.relayers_slice().iter().any(|r| r.id == *who) && !order.is_confirmed() {
				return true;
			}
		}
		false
	}
}

pub trait Slasher<T: Config> {
	fn slash(base: RingBalance<T>, _timeout: T::BlockNumber) -> RingBalance<T>;
}

impl<T: Config> Slasher<T> for () {
	// The slash result = base(p3 fee) + slash_each_block * timeout
	// Note: The maximum slash result is the MiniumLockCollateral. We mush ensures that all enrolled
	// relayers have ability to pay this slash result.
	fn slash(base: Fee<T>, timeout: T::BlockNumber) -> RingBalance<T> {
		// Slash 20 RING for each delay block until the maximum slash value
		let slash_each_block = 20_000_000_000u128;
		let timeout_u128: u128 = timeout.unique_saturated_into();
		let mut slash = base.saturating_add(
			timeout_u128
				.saturating_mul(slash_each_block)
				.unique_saturated_into(),
		);

		if slash >= T::MiniumLockCollateral::get() {
			slash = T::MiniumLockCollateral::get();
		}
		slash
	}
}
