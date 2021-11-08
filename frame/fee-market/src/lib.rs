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
use num_traits::Zero;
use sp_runtime::{
	traits::{Saturating, UniqueSaturatedInto},
	Permill, SaturatedConversion,
};
use sp_std::{default::Default, vec::Vec};
// --- darwinia-network ---
use darwinia_support::{
	balance::{LockFor, LockableCurrency},
	AccountId,
};
use dp_fee::{Order, Relayer};

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

		/// The minimum fee for relaying.
		#[pallet::constant]
		type MinimumRelayFee: Get<Fee<Self>>;
		/// The assigned relayers number for each order.
		#[pallet::constant]
		type AssignedRelayersNumber: Get<u64>;
		/// The slot times set
		#[pallet::constant]
		type Slot: Get<Self::BlockNumber>;

		/// Reward parameters
		#[pallet::constant]
		type AssignedRelayersRewardRatio: Get<Permill>;
		#[pallet::constant]
		type MessageRelayersRewardRatio: Get<Permill>;
		#[pallet::constant]
		type ConfirmRelayersRewardRatio: Get<Permill>;

		/// The slash rule
		type SlashForEachBlock: Get<RingBalance<Self>>;
		/// The collateral relayer need to lock for each order.
		#[pallet::constant]
		type CollateralEachOrder: Get<RingBalance<Self>>;

		type RingCurrency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>
			+ Currency<Self::AccountId>;
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type WeightInfo: WeightInfo;
	}

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	#[pallet::metadata(
		T::AccountId = "AccountId",
		RingBalance<T> = "RingBalance",
		Fee<T> = "Fee"
	)]
	pub enum Event<T: Config> {
		/// Relayer enrollment
		Enroll(T::AccountId, RingBalance<T>, Fee<T>, u32),
		/// Update relayer
		UpdateRelayer(
			T::AccountId,
			Option<RingBalance<T>>,
			Option<u32>,
			Option<Fee<T>>,
		),
		/// Relayer cancel enrollment
		CancelEnrollment(T::AccountId),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Insufficient balance.
		InsufficientBalance,
		/// The relayer has been enrolled.
		AlreadyEnrolled,
		/// This relayer doesn't enroll ever.
		NotEnrolled,
		/// Only increase lock collateral is allowed when update_locked_balance.
		OnlyIncCollateralAllowed,
		/// The fee is lower than MinimumRelayFee.
		RelayFeeTooLow,
		/// The relayer is occupied, and can't cancel enrollment now.
		OccupiedRelayer,
		/// Extend lock failed.
		ExtendLockFailed,
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
	pub type AssignedRelayers<T: Config> =
		StorageValue<_, Vec<Relayer<T::AccountId, RingBalance<T>>>, OptionQuery>;

	// Order storage
	#[pallet::storage]
	#[pallet::getter(fn order)]
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
			ensure!(!Self::is_enrolled(&who), <Error<T>>::AlreadyEnrolled);

			ensure!(
				T::RingCurrency::free_balance(&who) >= lock_collateral,
				<Error<T>>::InsufficientBalance
			);
			let order_capacity: u32 =
				(lock_collateral / T::CollateralEachOrder::get()).saturated_into::<u32>();

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
			// Store enrollment detail information.
			<RelayersMap<T>>::insert(
				&who,
				Relayer::new(who.clone(), lock_collateral, fee, order_capacity),
			);
			<Relayers<T>>::append(&who);

			Self::update_market();
			Self::deposit_event(Event::<T>::Enroll(
				who,
				lock_collateral,
				fee,
				order_capacity,
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

			let collat_diff = new_collateral.saturating_sub(Self::get_relayer(&who).collateral);
			ensure!(collat_diff > <RingBalance<T>>::zero(), <Error<T>>::OnlyIncCollateralAllowed);

			let _ = T::RingCurrency::extend_lock(
				T::LockId::get(),
				&who,
				new_collateral,
				WithdrawReasons::all(),
			)
			.map_err(|_| <Error<T>>::ExtendLockFailed);
			<RelayersMap<T>>::mutate(who.clone(), |relayer| {
				relayer.collateral = new_collateral;
				relayer.order_capacity +=
					(collat_diff / T::CollateralEachOrder::get()).saturated_into::<u32>();
			});

			Self::update_market();
			Self::deposit_event(Event::<T>::UpdateRelayer(
				who.clone(),
				Some(new_collateral),
				Some(Self::get_relayer(&who).order_capacity),
				None,
			));
			Ok(().into())
		}

		/// Update relay fee for enrolled relayer
		#[pallet::weight(<T as Config>::WeightInfo::update_relay_fee())]
		#[transactional]
		pub fn update_relay_fee(
			origin: OriginFor<T>,
			new_fee: Fee<T>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_enrolled(&who), <Error<T>>::NotEnrolled);
			ensure!(
				new_fee >= T::MinimumRelayFee::get(),
				<Error<T>>::RelayFeeTooLow
			);

			<RelayersMap<T>>::mutate(who.clone(), |relayer| {
				relayer.fee = new_fee;
			});

			Self::update_market();
			Self::deposit_event(Event::<T>::UpdateRelayer(who, None, None, Some(new_fee)));
			Ok(().into())
		}

		/// Cancel enrolled relayer
		#[pallet::weight(<T as Config>::WeightInfo::cancel_enrollment())]
		#[transactional]
		pub fn cancel_enrollment(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_enrolled(&who), <Error<T>>::NotEnrolled);
			ensure!(!Self::is_occupied(&who), <Error<T>>::OccupiedRelayer);

			Self::remove_enrolled_relayer(&who);
			Self::deposit_event(Event::<T>::CancelEnrollment(who));
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// An important update in this pallet, need to update market information in the following cases:
	///
	/// - When new relayer enroll.
	/// - When enrolled relayer wants to update fee or order capacity.
	/// - When enrolled relayer wants to cancel enrollment.
	pub fn update_market() {
		// Sort all enrolled relayers firstly.
		let mut relayers: Vec<Relayer<T::AccountId, RingBalance<T>>> = <Relayers<T>>::get()
			.iter()
			.map(RelayersMap::<T>::get)
			.collect();

		// Select the first `AssignedRelayersNumber` relayers as AssignedRelayer.
		// Only when total relayer's number greater than `AssignedRelayersNumber`, selection happens.
		let assigned_relayers_len = T::AssignedRelayersNumber::get() as usize;
		if relayers.len() >= assigned_relayers_len {
			relayers.sort();

			let mut assigned_relayers = Vec::with_capacity(assigned_relayers_len);
			// todo: need more tests.
			while let Some(r) = relayers.iter().next() {
				if assigned_relayers.len() == assigned_relayers_len {
					break;
				}

				if r.order_capacity >= 1 {
					assigned_relayers.push(r);
				}
			}
			<AssignedRelayers<T>>::put(assigned_relayers);
		} else {
			// The enrolled relayers not enough, pallet can't provide any fee advice.
			<AssignedRelayers<T>>::kill();
		}
	}

	/// Update relayer after slash occurred, this will changes RelayersMap storage.
	pub fn update_relayer_after_slash(who: &T::AccountId, new_collateral: RingBalance<T>) {
		if new_collateral == RingBalance::<T>::zero() {
			Self::remove_enrolled_relayer(who);
			return;
		}

		// Update locked collateral
		let _ = T::RingCurrency::extend_lock(
			T::LockId::get(),
			who,
			new_collateral,
			WithdrawReasons::all(),
		)
		.map_err(|_| <Error<T>>::ExtendLockFailed);
		// Update order capacity
		let new_capacity: u32 =
			(new_collateral / T::CollateralEachOrder::get()).saturated_into::<u32>();
		<RelayersMap<T>>::mutate(who.clone(), |relayer| {
			relayer.collateral = new_collateral;
			relayer.order_capacity = new_capacity;
		});

		Self::update_market();
	}

	/// Remove enrolled relayer, then update market fee.
	pub fn remove_enrolled_relayer(who: &T::AccountId) {
		T::RingCurrency::remove_lock(T::LockId::get(), who);

		<RelayersMap<T>>::remove(who.clone());
		<Relayers<T>>::mutate(|relayers| relayers.retain(|x| x != who));
		<AssignedRelayers<T>>::mutate(|assigned_relayers| {
			if let Some(relayers) = assigned_relayers {
				relayers.retain(|x| x.id != *who);
			}
		});
		Self::update_market();
	}

	/// Decrease relayer order capacity by 1 after message order created.
	pub fn dec_relayer_order_capacity(relayers: &[T::AccountId]) {
		// todo: need to check 0-1 case.
		for who in relayers {
			<RelayersMap<T>>::mutate(who.clone(), |r| {
				r.order_capacity -= 1;
			});
		}
		Self::update_market();
	}

	/// Increase relayer order capacity by 1 after message order confirmed.
	pub fn inc_relayer_order_capacity(relayers: &[T::AccountId]) {
		for who in relayers {
			<RelayersMap<T>>::mutate(who.clone(), |r| {
				r.order_capacity += 1;
			});
		}
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
