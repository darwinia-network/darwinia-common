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

mod payment;

use bp_messages::{DeliveredMessages, LaneId, MessageNonce};
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
use sp_io::hashing::blake2_256;
use sp_std::{
	cmp::{Ord, Ordering},
	default::Default,
	ops::Range,
	vec::Vec,
};
use sp_core::H256;

pub type AccountId<T> = <T as frame_system::Config>::AccountId;
pub type RingBalance<T> = <<T as Config>::RingCurrency as Currency<AccountId<T>>>::Balance;
pub type Fee<T> = RingBalance<T>;

pub use pallet::*;

const PriorRelayersNumber: u64 = 3;

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
		type MinimumFee: Get<Fee<Self>>;
		#[pallet::constant]
		type LockId: Get<LockIdentifier>;
		#[pallet::constant]
		// todo: maybe this can change to tuple?
		type T: Get<(Self::BlockNumber, Self::BlockNumber, Self::BlockNumber)>;

		type RingCurrency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;
		type WeightInfo: WeightInfo;
	}

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	#[pallet::metadata(T::AccountId = "AccountId")]
	pub enum Event<T: Config> {
		/// Relayer register
		Register(T::AccountId, RingBalance<T>, Fee<T>),
		/// Update relayer lock balance
		UpdateLockedBalance(T::AccountId, RingBalance<T>),
		/// Update relayer fee
		UpdateFee(T::AccountId, Fee<T>),
		/// Cancel relayer register
		CancelRelayerRegister(T::AccountId),
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
		/// Only Relayer can submit fee
		InvalidSubmitPriceOrigin,
		/// The fee is lower than MinimumFee
		TooLowFee,
	}

	#[pallet::storage]
	#[pallet::getter(fn get_relayer)]
	pub type RelayersMap<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, Relayer<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn relayers)]
	pub type Relayers<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

	/// The lowest n fees, p.0 < p.1 < p.2 ... < p.n
	#[pallet::storage]
	#[pallet::getter(fn prior_relayers)]
	pub type PriorRelayers<T: Config> = StorageValue<_, Vec<(T::AccountId, Fee<T>)>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn best_relayer)]
	pub type TopRelayer<T: Config> = StorageValue<_, (T::AccountId, Fee<T>), ValueQuery>;

	#[pallet::storage]
	pub type Orders<T: Config> =
		StorageMap<_, Blake2_128Concat, H256, Order<T::AccountId, T::BlockNumber>, ValueQuery>;

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
			fee: Option<Fee<T>>,
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
			if let Some(p) = fee {
				ensure!(p >= T::MinimumFee::get(), <Error<T>>::TooLowFee);
			}

			let fee = fee.unwrap_or_else(T::MinimumFee::get);
			T::RingCurrency::set_lock(
				T::LockId::get(),
				&who,
				LockFor::Common { amount: lock_value },
				WithdrawReasons::all(),
			);

			<RelayersMap<T>>::insert(&who, Relayer::new(who.clone(), lock_value, fee));
			<Relayers<T>>::append(who.clone());

			Self::update_relayer_fees()?;
			Self::deposit_event(Event::<T>::Register(who, lock_value, fee));
			Ok(().into())
		}

		/// Relayer update locked balance
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
			Self::deposit_event(Event::<T>::UpdateLockedBalance(who, new_lock));
			Ok(().into())
		}

		/// Relayer cancel register
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

			Self::update_relayer_fees()?;
			Self::deposit_event(Event::<T>::CancelRelayerRegister(who));
			Ok(().into())
		}

		/// Relayer update fee
		#[pallet::weight(10000)]
		#[transactional]
		pub fn update_fee(origin: OriginFor<T>, p: Fee<T>) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			ensure!(
				Self::is_registered(&who),
				<Error<T>>::InvalidSubmitPriceOrigin
			);
			ensure!(p >= T::MinimumFee::get(), <Error<T>>::TooLowFee);

			<RelayersMap<T>>::mutate(who.clone(), |relayer| {
				relayer.fee = p;
			});

			Self::update_relayer_fees()?;
			Self::deposit_event(Event::<T>::UpdateFee(who, p));
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Update fees in the following cases:
	/// 1. New relayer register
	/// 2. Already registered relayer update fee
	/// 3. Cancel registered relayer
	pub fn update_relayer_fees() -> Result<(), DispatchError> {
		<PriorRelayers<T>>::kill();

		let mut relayers: Vec<Relayer<T>> = <Relayers<T>>::get()
			.iter()
			.map(RelayersMap::<T>::get)
			.collect();
		relayers.sort();

		// If the registered relayers number >= the PriorRelayersNumber,
		// append the lowest PriorRelayersNumber relayers to PriorRelayers and choose the last one as TopRelayer.
		if relayers.len() >= PriorRelayersNumber as usize {
			for i in 0..PriorRelayersNumber as usize {
				let r = &relayers[i];
				// <PriorRelayers<T>>::append((r.id.clone(), r.fee));
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

	/// Whether the relayer has registered
	pub fn is_registered(who: &T::AccountId) -> bool {
		<Relayers<T>>::get().iter().any(|r| *r == *who)
	}

	// Get relayer fee
	pub fn relayer_price(who: &T::AccountId) -> Fee<T> {
		Self::get_relayer(who).fee
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
	fee: Fee<T>,
}

impl<T: Config> Relayer<T> {
	pub fn new(id: T::AccountId, lock_balance: RingBalance<T>, fee: Fee<T>) -> Relayer<T> {
		Relayer {
			id,
			lock_balance,
			fee,
		}
	}
}

impl<T: Config> PartialOrd for Relayer<T> {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		self.fee.partial_cmp(&other.fee)
	}
}

impl<T: Config> Ord for Relayer<T> {
	fn cmp(&self, other: &Self) -> Ordering {
		self.fee.cmp(&other.fee)
	}
}

impl<T: Config> PartialEq for Relayer<T> {
	fn eq(&self, other: &Self) -> bool {
		self.fee == other.fee && self.id == other.id && self.lock_balance == other.lock_balance
	}
}

impl<T: Config> Default for Relayer<T> {
	fn default() -> Self {
		Relayer {
			id: T::AccountId::default(),
			lock_balance: RingBalance::<T>::default(),
			fee: Fee::<T>::default(),
		}
	}
}

type PriorRelayers<AccountId, BlockNumber> = (
	Option<PriorRelayer<AccountId, BlockNumber>>,
	Option<PriorRelayer<AccountId, BlockNumber>>,
	Option<PriorRelayer<AccountId, BlockNumber>>,
);

#[derive(Clone, RuntimeDebug, Encode, Decode, Default)]
pub struct Order<AccountId, BlockNumber> {
	lane: LaneId,
	message: MessageNonce,
	sent_time: BlockNumber,
	confirm_time: Option<BlockNumber>,
	prior_relayers: PriorRelayers<AccountId, BlockNumber>,
}

impl<AccountId, BlockNumber> Order<AccountId, BlockNumber> {
	pub fn new(lane: LaneId, message: MessageNonce, sent_time: BlockNumber) -> Self {
		Self {
			lane,
			message,
			sent_time,
			confirm_time: None,
			prior_relayers: (None, None, None),
		}
	}

	pub fn set_prior_relayers(&mut self, prior_relayers: PriorRelayers<AccountId, BlockNumber>) {
		self.prior_relayers = prior_relayers;
	}

	pub fn set_confirm_time(&mut self, confirm_time: Option<BlockNumber>) {
		self.confirm_time = confirm_time;
	}

	pub fn prior_relayers(&self) -> PriorRelayers<AccountId, BlockNumber> {
		self.prior_relayers
	}
}

#[derive(Clone, RuntimeDebug, Encode, Decode, Default)]
pub struct PriorRelayer<AccountId, BlockNumber> {
	id: AccountId,
	priority: Priority,
	valid_range: Range<BlockNumber>,
}

impl<AccountId, BlockNumber> PriorRelayer<AccountId, BlockNumber>
where
	BlockNumber: std::ops::Add<Output = BlockNumber>,
{
	pub fn new(id: AccountId, start_time: BlockNumber, last_time: BlockNumber) -> Self {
		Self {
			id,
			priority: Priority::NoPriority,
			valid_range: Range {
				start: start_time,
				end: start_time + last_time,
			},
		}
	}
}

#[derive(Clone, RuntimeDebug, Encode, Decode, Copy)]
pub enum Priority {
	NoPriority,
	P1,
	P2,
	P3,
}

impl Default for Priority {
	fn default() -> Self {
		Priority::NoPriority
	}
}

/// Handler for messages have been accepted
pub trait OnMessageAccepted {
	/// Called when a message has been accepted by message pallet.
	fn on_messages_accepted(lane: &LaneId, message: &MessageNonce) -> Weight;
}

pub struct MessageAcceptedHandler<T>(PhantomData<T>);

impl<T: Config> OnMessageAccepted for MessageAcceptedHandler<T> {
	// Called when the message is accepted by message pallet
	fn on_messages_accepted(lane: &LaneId, message: &MessageNonce) -> Weight {
		// create an order
		let current_block_number = frame_system::Pallet::<T>::block_number();
		let mut order: Order<T::AccountId, T::BlockNumber> =
			Order::new(*lane, *message, current_block_number);
		// TODO: get prior relayers from market
		let prior_relayers = (None, None, None);
		order.set_prior_relayers(prior_relayers);

		// store the create order
		let message_hash: H256 = (lane, message).using_encoded(blake2_256).into();
		<Orders<T>>::insert(message_hash, order);
		10000 // todo: update the weight
	}
}

/// Handler for messages delivery confirmation.
pub trait OnDeliveryConfirmed {
	fn on_messages_delivered(_lane: &LaneId, _messages: &DeliveredMessages) -> Weight;
}

pub struct MessageConfirmedHandler;

impl OnDeliveryConfirmed for MessageConfirmedHandler {
	fn on_messages_delivered(_lane: &LaneId, _messages: &DeliveredMessages) -> Weight {
		todo!()
	}
}
