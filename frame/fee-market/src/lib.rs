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
use sp_runtime::{traits::Saturating, Permill, SaturatedConversion};
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
		/// The collateral relayer need to lock for each order.
		#[pallet::constant]
		type CollateralPerOrder: Get<RingBalance<Self>>;
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
		Enroll(T::AccountId, RingBalance<T>, Fee<T>),
		/// Update relayer
		UpdateRelayer(T::AccountId, Option<RingBalance<T>>, Option<Fee<T>>),
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
		/// Update locked collateral is not allow since some orders are not confirm.
		StillHasOrdersNotConfirmed,
		/// The fee is lower than MinimumRelayFee.
		RelayFeeTooLow,
		/// The relayer is occupied, and can't cancel enrollment now.
		OccupiedRelayer,
		/// Extend lock failed.
		ExtendLockFailed,
	}

	// Enrolled relayers storage
	#[pallet::storage]
	#[pallet::getter(fn relayer)]
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
		/// the default value is MinimumRelayFee in runtime. (Update market needed)
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
			<RelayersMap<T>>::insert(&who, Relayer::new(who.clone(), lock_collateral, fee));
			<Relayers<T>>::append(&who);

			Self::update_market();
			Self::deposit_event(Event::<T>::Enroll(who, lock_collateral, fee));
			Ok(().into())
		}

		/// Update locked collateral for enrolled relayer, only supporting lock more. (Update market needed)
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

			// Increase the locked collateral
			if new_collateral >= Self::relayer(&who).collateral {
				let _ = T::RingCurrency::extend_lock(
					T::LockId::get(),
					&who,
					new_collateral,
					WithdrawReasons::all(),
				)
				.map_err(|_| <Error<T>>::ExtendLockFailed);
			} else {
				// Decrease the locked collateral
				if let Some((_, orders_locked_collateral)) = Self::occupied(&who) {
					ensure!(
						new_collateral >= orders_locked_collateral,
						<Error<T>>::StillHasOrdersNotConfirmed
					);

					T::RingCurrency::remove_lock(T::LockId::get(), &who);
					T::RingCurrency::set_lock(
						T::LockId::get(),
						&who,
						LockFor::Common {
							amount: new_collateral,
						},
						WithdrawReasons::all(),
					);
				}
			}

			<RelayersMap<T>>::mutate(who.clone(), |relayer| {
				relayer.collateral = new_collateral;
			});
			Self::update_market();
			Self::deposit_event(Event::<T>::UpdateRelayer(
				who.clone(),
				Some(new_collateral),
				None,
			));
			Ok(().into())
		}

		/// Update relay fee for enrolled relayer. (Update market needed)
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
			Self::deposit_event(Event::<T>::UpdateRelayer(who, None, Some(new_fee)));
			Ok(().into())
		}

		/// Cancel enrolled relayer(Update market needed)
		#[pallet::weight(<T as Config>::WeightInfo::cancel_enrollment())]
		#[transactional]
		pub fn cancel_enrollment(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_enrolled(&who), <Error<T>>::NotEnrolled);
			ensure!(Self::occupied(&who).is_none(), <Error<T>>::OccupiedRelayer);

			Self::remove_enrolled_relayer(&who);
			Self::deposit_event(Event::<T>::CancelEnrollment(who));
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// An important update in this pallet, need to update market information in the following cases:
	///
	/// - New relayer enroll.
	/// - The enrolled relayer wants to update fee or order capacity.
	/// - The enrolled relayer wants to cancel enrollment.
	/// - The order didn't confirm in-time, slash occurred.
	pub(crate) fn update_market() {
		// Sort all enrolled relayers who are able to accept orders.
		let mut relayers: Vec<Relayer<T::AccountId, RingBalance<T>>> = <Relayers<T>>::get()
			.iter()
			.map(RelayersMap::<T>::get)
			.filter(|r| Self::usable_order_capacity(&r.id) >= 1)
			.collect();

		// Select the first `AssignedRelayersNumber` relayers as AssignedRelayer.
		let assigned_relayers_len = T::AssignedRelayersNumber::get() as usize;
		if relayers.len() >= assigned_relayers_len {
			relayers.sort();

			let assigned_relayers: Vec<_> = relayers.iter().take(assigned_relayers_len).collect();
			<AssignedRelayers<T>>::put(assigned_relayers);
		} else {
			// The market fee comes from the last item in AssignedRelayers,
			// It's would be essential to wipe this storage if relayers not enough.
			<AssignedRelayers<T>>::kill();
		}
	}

	/// Update relayer after slash occurred, this will changes RelayersMap storage. (Update market needed)
	pub(crate) fn update_relayer_after_slash(who: &T::AccountId, new_collateral: RingBalance<T>) {
		T::RingCurrency::set_lock(
			T::LockId::get(),
			&who,
			LockFor::Common {
				amount: new_collateral,
			},
			WithdrawReasons::all(),
		);
		<RelayersMap<T>>::mutate(who.clone(), |relayer| {
			relayer.collateral = new_collateral;
		});

		if Self::usable_order_capacity(&who) == 0 {
			Self::remove_enrolled_relayer(who);
			return;
		}

		Self::update_market();
	}

	/// Remove enrolled relayer, then update market fee. (Update market needed)
	pub(crate) fn remove_enrolled_relayer(who: &T::AccountId) {
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

	/// Whether the relayer has enrolled
	pub(crate) fn is_enrolled(who: &T::AccountId) -> bool {
		<Relayers<T>>::get().iter().any(|r| *r == *who)
	}

	/// Get market fee, If there is not enough relayers have order capacity to accept new order, return None.
	pub fn market_fee() -> Option<Fee<T>> {
		Self::assigned_relayers().and_then(|relayers| relayers.last().map(|r| r.fee))
	}

	/// Whether the enrolled relayer is occupied, If occupied, return the number of orders and orders locked collateral, otherwise, return None.
	pub(crate) fn occupied(who: &T::AccountId) -> Option<(u32, RingBalance<T>)> {
		let mut count = 0u32;
		let mut orders_locked_collateral = RingBalance::<T>::zero();
		for (_, order) in <Orders<T>>::iter() {
			if order.relayers_slice().iter().any(|r| r.id == *who) && !order.is_confirmed() {
				count += 1;
				orders_locked_collateral =
					orders_locked_collateral.saturating_add(order.locked_collateral);
			}
		}

		if count == 0 {
			return None;
		}
		Some((count, orders_locked_collateral))
	}

	/// The relayer collateral is composed of two part: fee_collateral and orders_locked_collateral.
	/// Calculate the order capacity with fee_collateral
	pub(crate) fn usable_order_capacity(who: &T::AccountId) -> u32 {
		if let Some((_, orders_locked_collateral)) = Self::occupied(&who) {
			let free_collateral = Self::relayer(who)
				.collateral
				.saturating_sub(orders_locked_collateral);
			return Self::collateral_to_order_capacity(free_collateral);
		}
		Self::collateral_to_order_capacity(Self::relayer(who).collateral)
	}

	fn collateral_to_order_capacity(collateral: RingBalance<T>) -> u32 {
		(collateral / T::CollateralPerOrder::get()).saturated_into::<u32>()
	}
}
