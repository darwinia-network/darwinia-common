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

//! # Balances Pallet
//!
//! The Balances pallet provides functionality for handling accounts and balances.
//!
//! - [`Config`]
//! - [`Call`]
//! - [`Pallet`]
//!
//! ## Overview
//!
//! The Balances pallet provides functions for:
//!
//! - Getting and setting free balances.
//! - Retrieving total, reserved and unreserved balances.
//! - Repatriating a reserved balance to a beneficiary account that exists.
//! - Transferring a balance between accounts (when not reserved).
//! - Slashing an account balance.
//! - Account creation and removal.
//! - Managing total issuance.
//! - Setting and managing locks.
//!
//! ### Terminology
//!
//! - **Existential Deposit:** The minimum balance required to create or keep an account open. This prevents
//! "dust accounts" from filling storage. When the free plus the reserved balance (i.e. the total balance)
//!   fall below this, then the account is said to be dead; and it loses its functionality as well as any
//!   prior history and all information on it is removed from the chain's state.
//!   No account should ever have a total balance that is strictly between 0 and the existential
//!   deposit (exclusive). If this ever happens, it indicates either a bug in this pallet or an
//!   erroneous raw mutation of storage.
//!
//! - **Total Issuance:** The total number of units in existence in a system.
//!
//! - **Reaping an account:** The act of removing an account by resetting its nonce. Happens after its
//! total balance has become zero (or, strictly speaking, less than the Existential Deposit).
//!
//! - **Free Balance:** The portion of a balance that is not reserved. The free balance is the only
//!   balance that matters for most operations.
//!
//! - **Reserved Balance:** Reserved balance still belongs to the account holder, but is suspended.
//!   Reserved balance can still be slashed, but only after all the free balance has been slashed.
//!
//! - **Imbalance:** A condition when some funds were credited or debited without equal and opposite accounting
//! (i.e. a difference between total issuance and account balances). Functions that result in an imbalance will
//! return an object of the `Imbalance` trait that can be managed within your runtime logic. (If an imbalance is
//! simply dropped, it should automatically maintain any book-keeping such as total issuance.)
//!
//! - **Lock:** A freeze on a specified amount of an account's free balance until a specified block number. Multiple
//! locks always operate over the same funds, so they "overlay" rather than "stack".
//!
//! ### Implementations
//!
//! The Balances pallet provides implementations for the following traits. If these traits provide the functionality
//! that you need, then you can avoid coupling with the Balances pallet.
//!
//! - [`Currency`](frame_support::traits::Currency): Functions for dealing with a
//! fungible assets system.
//! - [`ReservableCurrency`](frame_support::traits::ReservableCurrency):
//! Functions for dealing with assets that can be reserved from an account.
//! - [`LockableCurrency`](darwinia_support::traits::LockableCurrency): Functions for
//! dealing with accounts that allow liquidity restrictions.
//! - [`Imbalance`](frame_support::traits::Imbalance): Functions for handling
//! imbalances between total issuance in the system and account balances. Must be used when a function
//! creates new funds (e.g. a reward) or destroys some funds (e.g. a system fee).
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! - `transfer` - Transfer some liquid free balance to another account.
//! - `set_balance` - Set the balances of a given account. The origin of this call must be root.
//!
//! ## Usage
//!
//! The following examples show how to use the Balances pallet in your custom pallet.
//!
//! ### Examples from the FRAME
//!
//! The Contract pallet uses the `Currency` trait to handle gas payment, and its types inherit from `Currency`:
//!
//! ```
//! use frame_support::traits::Currency;
//! # pub trait Config: frame_system::Config {
//! # 	type Currency: Currency<Self::AccountId>;
//! # }
//!
//! pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
//! pub type NegativeImbalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::NegativeImbalance;
//!
//! # fn main() {}
//! ```
//!
//! The Staking pallet uses the `LockableCurrency` trait to lock a stash account's funds:
//!
//! ```
//! use frame_support::traits::WithdrawReasons;
//! use sp_runtime::traits::Bounded;
//! use darwinia_support::balance::*;
//! pub trait Config: frame_system::Config {
//! 	type Currency: LockableCurrency<Self::AccountId, Moment=Self::BlockNumber>;
//! }
//! # struct StakingLedger<T: Config> {
//! # 	stash: <T as frame_system::Config>::AccountId,
//! # 	total: <<T as Config>::Currency as frame_support::traits::Currency<<T as frame_system::Config>::AccountId>>::Balance,
//! # 	phantom: std::marker::PhantomData<T>,
//! # }
//! # const STAKING_ID: [u8; 8] = *b"staking ";
//!
//! fn update_ledger<T: Config>(
//! 	controller: &T::AccountId,
//! 	ledger: &StakingLedger<T>
//! ) {
//! 	T::Currency::set_lock(
//! 		STAKING_ID,
//! 		&ledger.stash,
//! 		LockFor::Common {
//! 			amount: ledger.total
//!			},
//! 		WithdrawReasons::all()
//! 	);
//! 	// <Ledger<T>>::insert(controller, ledger); // Commented out as we don't have access to Staking's storage here.
//! }
//! # fn main() {}
//! ```
//!
//! ## Genesis config
//!
//! The Balances pallet depends on the [`GenesisConfig`].
//!
//! ## Assumptions
//!
//! * Total issued balanced of all accounts should be less than `Config::Balance::max_value()`.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
#[macro_use]
mod tests;
#[cfg(test)]
mod tests_local;

pub mod weights;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
	// wrapping these imbalances in a private module is necessary to ensure absolute privacy
	// of the inner member.
	pub mod imbalances {
		// --- darwinia ---
		use crate::pallet::*;

		/// Opaque, move-only struct with private fields that serves as a token denoting that
		/// funds have been created without any equal and opposite accounting.
		#[must_use]
		#[derive(RuntimeDebug, PartialEq, Eq)]
		pub struct PositiveImbalance<T: Config<I>, I: 'static = ()>(T::Balance);

		impl<T: Config<I>, I: 'static> PositiveImbalance<T, I> {
			/// Create a new positive imbalance from a balance.
			pub fn new(amount: T::Balance) -> Self {
				PositiveImbalance(amount)
			}
		}

		/// Opaque, move-only struct with private fields that serves as a token denoting that
		/// funds have been destroyed without any equal and opposite accounting.
		#[must_use]
		#[derive(RuntimeDebug, PartialEq, Eq)]
		pub struct NegativeImbalance<T: Config<I>, I: 'static = ()>(T::Balance);

		impl<T: Config<I>, I: 'static> NegativeImbalance<T, I> {
			/// Create a new negative imbalance from a balance.
			pub fn new(amount: T::Balance) -> Self {
				NegativeImbalance(amount)
			}
		}

		impl<T: Config<I>, I: 'static> TryDrop for PositiveImbalance<T, I> {
			fn try_drop(self) -> Result<(), Self> {
				self.drop_zero()
			}
		}

		impl<T: Config<I>, I: 'static> Imbalance<T::Balance> for PositiveImbalance<T, I> {
			type Opposite = NegativeImbalance<T, I>;

			fn zero() -> Self {
				Self(Zero::zero())
			}
			fn drop_zero(self) -> Result<(), Self> {
				if self.0.is_zero() {
					Ok(())
				} else {
					Err(self)
				}
			}
			fn split(self, amount: T::Balance) -> (Self, Self) {
				let first = self.0.min(amount);
				let second = self.0 - first;

				mem::forget(self);
				(Self(first), Self(second))
			}
			fn merge(mut self, other: Self) -> Self {
				self.0 = self.0.saturating_add(other.0);
				mem::forget(other);

				self
			}
			fn subsume(&mut self, other: Self) {
				self.0 = self.0.saturating_add(other.0);
				mem::forget(other);
			}
			fn offset(self, other: Self::Opposite) -> Result<Self, Self::Opposite> {
				let (a, b) = (self.0, other.0);
				mem::forget((self, other));

				if a >= b {
					Ok(Self(a - b))
				} else {
					Err(NegativeImbalance::new(b - a))
				}
			}
			fn peek(&self) -> T::Balance {
				self.0.clone()
			}
		}

		impl<T: Config<I>, I: 'static> TryDrop for NegativeImbalance<T, I> {
			fn try_drop(self) -> Result<(), Self> {
				self.drop_zero()
			}
		}

		impl<T: Config<I>, I: 'static> Imbalance<T::Balance> for NegativeImbalance<T, I> {
			type Opposite = PositiveImbalance<T, I>;

			fn zero() -> Self {
				Self(Zero::zero())
			}
			fn drop_zero(self) -> Result<(), Self> {
				if self.0.is_zero() {
					Ok(())
				} else {
					Err(self)
				}
			}
			fn split(self, amount: T::Balance) -> (Self, Self) {
				let first = self.0.min(amount);
				let second = self.0 - first;

				mem::forget(self);
				(Self(first), Self(second))
			}
			fn merge(mut self, other: Self) -> Self {
				self.0 = self.0.saturating_add(other.0);
				mem::forget(other);

				self
			}
			fn subsume(&mut self, other: Self) {
				self.0 = self.0.saturating_add(other.0);
				mem::forget(other);
			}
			fn offset(self, other: Self::Opposite) -> Result<Self, Self::Opposite> {
				let (a, b) = (self.0, other.0);
				mem::forget((self, other));

				if a >= b {
					Ok(Self(a - b))
				} else {
					Err(PositiveImbalance::new(b - a))
				}
			}
			fn peek(&self) -> T::Balance {
				self.0.clone()
			}
		}

		impl<T: Config<I>, I: 'static> Drop for PositiveImbalance<T, I> {
			/// Basic drop handler will just square up the total issuance.
			fn drop(&mut self) {
				<TotalIssuance<T, I>>::mutate(|v| *v = v.saturating_add(self.0));
			}
		}

		impl<T: Config<I>, I: 'static> Drop for NegativeImbalance<T, I> {
			/// Basic drop handler will just square up the total issuance.
			fn drop(&mut self) {
				<TotalIssuance<T, I>>::mutate(|v| *v = v.saturating_sub(self.0));
			}
		}
	}
	pub use imbalances::{NegativeImbalance, PositiveImbalance};

	// --- crates ---
	use codec::{Codec, EncodeLike};
	// --- substrate ---
	use frame_support::{
		ensure,
		pallet_prelude::*,
		traits::{
			BalanceStatus, Currency, ExistenceRequirement, Imbalance, LockIdentifier, OnUnbalanced,
			ReservableCurrency, SignedImbalance, StoredMap, TryDrop, WithdrawReasons,
		},
	};
	use frame_system::pallet_prelude::*;
	// --- substrate ---
	use sp_runtime::{
		traits::{
			AtLeast32BitUnsigned, Bounded, CheckedAdd, CheckedSub, MaybeSerializeDeserialize,
			Saturating, StaticLookup, StoredMapError, Zero,
		},
		DispatchError, DispatchResult, RuntimeDebug,
	};
	use sp_std::{borrow::Borrow, cmp, fmt::Debug, mem, prelude::*};
	// --- darwinia ---
	use crate::weights::WeightInfo;
	use darwinia_balances_rpc_runtime_api::RuntimeDispatchInfo;
	use darwinia_support::{balance::*, impl_rpc, traits::BalanceInfo};

	#[pallet::config]
	pub trait Config<I: 'static = ()>: frame_system::Config {
		/// The balance of an account.
		type Balance: Parameter
			+ Member
			+ AtLeast32BitUnsigned
			+ Codec
			+ Default
			+ Copy
			+ MaybeSerializeDeserialize
			+ Debug;

		/// Handler for the unbalanced reduction when removing a dust account.
		type DustRemoval: OnUnbalanced<NegativeImbalance<Self, I>>;

		/// The overarching event type.
		type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;

		/// The minimum amount required to keep an account open.
		#[pallet::constant]
		type ExistentialDeposit: Get<Self::Balance>;

		/// The means of storing the balances of an account.
		type AccountStore: StoredMap<Self::AccountId, Self::BalanceInfo>;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;

		/// The maximum number of locks that should exist on an account.
		/// Not strictly enforced, but used for weight estimation.
		type MaxLocks: Get<u32>;

		type BalanceInfo: BalanceInfo<Self::Balance, I>
			+ Into<<Self as frame_system::Config>::AccountData>
			+ Member
			+ Codec
			+ Clone
			+ Default
			+ EncodeLike;

		// A handle to check if other curencies drop below existential deposit
		type OtherCurrencies: DustCollector<Self::AccountId>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	#[pallet::metadata(T::AccountId = "AccountId", T::Balance = "Balance")]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		/// An account was created with some free balance. \[account, free_balance\]
		Endowed(T::AccountId, T::Balance),
		/// An account was removed whose balance was non-zero but below ExistentialDeposit,
		/// resulting in an outright loss. \[account, balance\]
		DustLost(T::AccountId, T::Balance),
		/// Transfer succeeded. \[from, to, value\]
		Transfer(T::AccountId, T::AccountId, T::Balance),
		/// A balance was set by root. \[who, free, reserved\]
		BalanceSet(T::AccountId, T::Balance, T::Balance),
		/// Some amount was deposited (e.g. for transaction fees). \[who, deposit\]
		Deposit(T::AccountId, T::Balance),
		/// Some balance was reserved (moved from free to reserved). \[who, value\]
		Reserved(T::AccountId, T::Balance),
		/// Some balance was unreserved (moved from reserved to free). \[who, value\]
		Unreserved(T::AccountId, T::Balance),
		/// Some balance was moved from the reserve of the first account to the second account.
		/// Final argument indicates the destination balance type.
		/// \[from, to, balance, destination_status\]
		ReserveRepatriated(T::AccountId, T::AccountId, T::Balance, BalanceStatus),
	}

	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// Vesting balance too high to send value
		VestingBalance,
		/// Account liquidity restrictions prevent withdrawal
		LiquidityRestrictions,
		/// Got an overflow after adding
		Overflow,
		/// Balance too low to send value
		InsufficientBalance,
		/// Value too low to create account due to existential deposit
		ExistentialDeposit,
		/// Transfer/payment would kill account
		KeepAlive,
		/// A vesting schedule already exists for this account
		ExistingVestingSchedule,
		/// Beneficiary account must pre-exist
		DeadAccount,
		/// Lock - POISONED
		LockP,
	}

	/// The total units issued in the system.
	#[pallet::storage]
	#[pallet::getter(fn total_issuance)]
	pub type TotalIssuance<T: Config<I>, I: 'static = ()> = StorageValue<_, T::Balance, ValueQuery>;

	/// The balance of an account.
	///
	/// NOTE: This is only used in the case that this pallet is used to store balances.
	#[pallet::storage]
	pub type Account<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, T::AccountId, T::BalanceInfo, ValueQuery>;

	/// Any liquidity locks on some account balances.
	/// NOTE: Should only be accessed when setting, changing and freeing a lock.
	#[pallet::storage]
	#[pallet::getter(fn locks)]
	pub type Locks<T: Config<I>, I: 'static = ()> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Vec<BalanceLock<T::Balance, T::BlockNumber>>,
		ValueQuery,
	>;

	/// Storage version of the pallet.
	///
	/// This is set to v2.0.0 for new networks.
	#[pallet::storage]
	pub(super) type StorageVersion<T: Config<I>, I: 'static = ()> =
		StorageValue<_, Releases, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config<I>, I: 'static = ()> {
		pub balances: Vec<(T::AccountId, T::Balance)>,
	}

	#[cfg(feature = "std")]
	impl<T: Config<I>, I: 'static> Default for GenesisConfig<T, I> {
		fn default() -> Self {
			Self {
				balances: Default::default(),
			}
		}
	}
	#[pallet::genesis_build]
	impl<T: Config<I>, I: 'static> GenesisBuild<T, I> for GenesisConfig<T, I> {
		fn build(&self) {
			let total = self
				.balances
				.iter()
				.fold(Zero::zero(), |acc: T::Balance, &(_, n)| acc + n);
			<TotalIssuance<T, I>>::put(total);

			<StorageVersion<T, I>>::put(Releases::V2_0_0);

			for (_, balance) in &self.balances {
				assert!(
					*balance >= <T as Config<I>>::ExistentialDeposit::get(),
					"the balance of any account should always be at least the existential deposit.",
				)
			}

			// ensure no duplicates exist.
			let endowed_accounts = self
				.balances
				.iter()
				.map(|(x, _)| x)
				.cloned()
				.collect::<std::collections::BTreeSet<_>>();

			assert!(
				endowed_accounts.len() == self.balances.len(),
				"duplicate balances in genesis."
			);

			for &(ref who, free) in self.balances.iter() {
				let mut account_data = T::AccountStore::get(who);
				account_data.set_free(free);

				assert!(T::AccountStore::insert(who, account_data).is_ok());
			}
		}
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T, I = ()>(PhantomData<(T, I)>);
	#[pallet::hooks]
	impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {}
	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		/// Transfer some liquid free balance to another account.
		///
		/// `transfer` will set the `FreeBalance` of the sender and receiver.
		/// It will decrease the total issuance of the system by the `TransferFee`.
		/// If the sender's account is below the existential deposit as a result
		/// of the transfer, the account will be reaped.
		///
		/// The dispatch origin for this call must be `Signed` by the transactor.
		///
		/// # <weight>
		/// - Dependent on arguments but not critical, given proper implementations for
		///   input config types. See related functions below.
		/// - It contains a limited number of reads and writes internally and no complex computation.
		///
		/// Related functions:
		///
		///   - `ensure_can_withdraw` is always called internally but has a bounded complexity.
		///   - Transferring balances to accounts that did not exist before will cause
		///      `T::OnNewAccount::on_new_account` to be called.
		///   - Removing enough funds from an account will trigger `T::DustRemoval::on_unbalanced`.
		///   - `transfer_keep_alive` works the same way as `transfer`, but has an additional
		///     check that the transfer will not kill the origin account.
		///
		/// # </weight>
		#[pallet::weight(T::WeightInfo::transfer())]
		pub fn transfer(
			origin: OriginFor<T>,
			dest: <T::Lookup as StaticLookup>::Source,
			#[pallet::compact] value: T::Balance,
		) -> DispatchResultWithPostInfo {
			let transactor = ensure_signed(origin)?;
			let dest = T::Lookup::lookup(dest)?;
			<Self as Currency<_>>::transfer(
				&transactor,
				&dest,
				value,
				ExistenceRequirement::AllowDeath,
			)?;
			Ok(().into())
		}

		/// Set the balances of a given account.
		///
		/// This will alter `FreeBalance` and `ReservedBalance` in storage. it will
		/// also decrease the total issuance of the system (`TotalIssuance`).
		/// If the new free or reserved balance is below the existential deposit,
		/// it will reset the account nonce (`frame_system::AccountNonce`).
		///
		/// The dispatch origin for this call is `root`.
		///
		/// # <weight>
		/// - Independent of the arguments.
		/// - Contains a limited number of reads and writes.
		/// # </weight>
		#[pallet::weight(
			T::WeightInfo::set_balance_creating() // Creates a new account.
				.max(T::WeightInfo::set_balance_killing()) // Kills an existing account.
		)]
		pub(super) fn set_balance(
			origin: OriginFor<T>,
			who: <T::Lookup as StaticLookup>::Source,
			#[pallet::compact] new_free: T::Balance,
			#[pallet::compact] new_reserved: T::Balance,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			let who = T::Lookup::lookup(who)?;
			let existential_deposit = T::ExistentialDeposit::get();

			let wipeout = {
				let new_total = new_free + new_reserved;

				new_total < existential_deposit && T::OtherCurrencies::is_dust(&who)
			};
			let new_free = if wipeout { Zero::zero() } else { new_free };
			let new_reserved = if wipeout { Zero::zero() } else { new_reserved };

			let (free, reserved) = Self::mutate_account(&who, |account| {
				if new_free > account.free() {
					mem::drop(<PositiveImbalance<T, I>>::new(new_free - account.free()));
				} else if new_free < account.free() {
					mem::drop(NegativeImbalance::<T, I>::new(account.free() - new_free));
				}

				if new_reserved > account.reserved() {
					mem::drop(<PositiveImbalance<T, I>>::new(
						new_reserved - account.reserved(),
					));
				} else if new_reserved < account.reserved() {
					mem::drop(<NegativeImbalance<T, I>>::new(
						account.reserved() - new_reserved,
					));
				}

				account.set_free(new_free);
				account.set_reserved(new_reserved);

				(account.free(), account.reserved())
			})?;
			Self::deposit_event(Event::BalanceSet(who, free, reserved));
			Ok(().into())
		}

		/// Exactly as `transfer`, except the origin must be root and the source account may be
		/// specified.
		/// # <weight>
		/// - Same as transfer, but additional read and write because the source account is
		///   not assumed to be in the overlay.
		/// # </weight>
		#[pallet::weight(T::WeightInfo::force_transfer())]
		pub fn force_transfer(
			origin: OriginFor<T>,
			source: <T::Lookup as StaticLookup>::Source,
			dest: <T::Lookup as StaticLookup>::Source,
			#[pallet::compact] value: T::Balance,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			let source = T::Lookup::lookup(source)?;
			let dest = T::Lookup::lookup(dest)?;
			<Self as Currency<_>>::transfer(
				&source,
				&dest,
				value,
				ExistenceRequirement::AllowDeath,
			)?;
			Ok(().into())
		}

		/// Same as the [`transfer`] call, but with a check that the transfer will not kill the
		/// origin account.
		///
		/// 99% of the time you want [`transfer`] instead.
		///
		/// [`transfer`]: struct.Pallet.html#method.transfer
		#[pallet::weight(T::WeightInfo::transfer_keep_alive())]
		pub fn transfer_keep_alive(
			origin: OriginFor<T>,
			dest: <T::Lookup as StaticLookup>::Source,
			#[pallet::compact] value: T::Balance,
		) -> DispatchResultWithPostInfo {
			let transactor = ensure_signed(origin)?;
			let dest = T::Lookup::lookup(dest)?;
			<Self as Currency<_>>::transfer(
				&transactor,
				&dest,
				value,
				ExistenceRequirement::KeepAlive,
			)?;
			Ok(().into())
		}
	}

	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		// PRIVATE MUTABLES

		/// Get the free balance of an account.
		pub fn free_balance(who: impl Borrow<T::AccountId>) -> T::Balance {
			Self::account(who.borrow()).free()
		}

		/// Get the frozen balance of an account.
		fn frozen_balance(who: impl Borrow<T::AccountId>) -> FrozenBalance<T::Balance> {
			let now = <frame_system::Pallet<T>>::block_number();
			let mut frozen_balance = <FrozenBalance<T::Balance>>::zero();
			for lock in Self::locks(who.borrow()).iter() {
				let locked_amount = match &lock.lock_for {
					LockFor::Common { amount } => *amount,
					LockFor::Staking(staking_lock) => staking_lock.locked_amount(now),
				};
				if lock.lock_reasons == LockReasons::All || lock.lock_reasons == LockReasons::Misc {
					frozen_balance.misc = frozen_balance.misc.max(locked_amount);
				}
				if lock.lock_reasons == LockReasons::All || lock.lock_reasons == LockReasons::Fee {
					frozen_balance.fee = frozen_balance.fee.max(locked_amount);
				}
			}

			frozen_balance
		}

		impl_rpc! {
			fn usable_balance_rpc(who: impl Borrow<T::AccountId>) -> RuntimeDispatchInfo<T::Balance> {
				RuntimeDispatchInfo {
					usable_balance: Self::usable_balance(who.borrow()),
				}
			}
		}

		/// Get the reserved balance of an account.
		pub fn reserved_balance(who: impl Borrow<T::AccountId>) -> T::Balance {
			let account = Self::account(who.borrow());
			account.reserved()
		}

		/// Get both the free and reserved balances of an account.
		fn account(who: &T::AccountId) -> T::BalanceInfo {
			T::AccountStore::get(&who)
		}

		/// Handles any steps needed after mutating an account.
		///
		/// This includes DustRemoval unbalancing, in the case than the `new` account's total balance
		/// is non-zero but below ED.
		///
		/// Returns two values:
		/// - `Some` containing the the `new` account, iff the account has sufficient balance.
		/// - `Some` containing the dust to be dropped, iff some dust should be dropped.
		fn post_mutation(
			who: &T::AccountId,
			new: T::BalanceInfo,
		) -> (Option<T::BalanceInfo>, Option<NegativeImbalance<T, I>>) {
			let total = new.total();

			if total < T::ExistentialDeposit::get() && T::OtherCurrencies::is_dust(who) {
				if total.is_zero() {
					(None, None)
				} else {
					(None, Some(NegativeImbalance::new(total)))
				}
			} else {
				(Some(new), None)
			}
		}

		/// Mutate an account to some new value, or delete it entirely with `None`. Will enforce
		/// `ExistentialDeposit` law, annulling the account as needed.
		///
		/// NOTE: Doesn't do any preparatory work for creating a new account, so should only be used
		/// when it is known that the account already exists.
		///
		/// NOTE: LOW-LEVEL: This will not attempt to maintain total issuance. It is expected that
		/// the caller will do this.
		pub fn mutate_account<R>(
			who: &T::AccountId,
			f: impl FnOnce(&mut T::BalanceInfo) -> R,
		) -> Result<R, StoredMapError> {
			Self::try_mutate_account(who, |a, _| -> Result<R, StoredMapError> { Ok(f(a)) })
		}

		/// Mutate an account to some new value, or delete it entirely with `None`. Will enforce
		/// `ExistentialDeposit` law, annulling the account as needed. This will do nothing if the
		/// result of `f` is an `Err`.
		///
		/// NOTE: Doesn't do any preparatory work for creating a new account, so should only be used
		/// when it is known that the account already exists.
		///
		/// NOTE: LOW-LEVEL: This will not attempt to maintain total issuance. It is expected that
		/// the caller will do this.
		fn try_mutate_account<R, E: From<StoredMapError>>(
			who: &T::AccountId,
			f: impl FnOnce(&mut T::BalanceInfo, bool) -> Result<R, E>,
		) -> Result<R, E> {
			Self::try_mutate_account_with_dust(who, f).map(|(result, dust_cleaner)| {
				drop(dust_cleaner);
				result
			})
		}

		/// Mutate an account to some new value, or delete it entirely with `None`. Will enforce
		/// `ExistentialDeposit` law, annulling the account as needed. This will do nothing if the
		/// result of `f` is an `Err`.
		///
		/// It returns both the result from the closure, and an optional `DustCleaner` instance which
		/// should be dropped once it is known that all nested mutates that could affect storage items
		/// what the dust handler touches have completed.
		///
		/// NOTE: Doesn't do any preparatory work for creating a new account, so should only be used
		/// when it is known that the account already exists.
		///
		/// NOTE: LOW-LEVEL: This will not attempt to maintain total issuance. It is expected that
		/// the caller will do this.
		fn try_mutate_account_with_dust<R, E: From<StoredMapError>>(
			who: &T::AccountId,
			f: impl FnOnce(&mut T::BalanceInfo, bool) -> Result<R, E>,
		) -> Result<(R, DustCleaner<T, I>), E> {
			let result = T::AccountStore::try_mutate_exists(who, |maybe_account| {
				let is_new = maybe_account.is_none();
				let mut account = maybe_account.take().unwrap_or_default();
				f(&mut account, is_new).map(move |result| {
					let maybe_endowed = if is_new { Some(account.free()) } else { None };
					let maybe_account_maybe_dust = Self::post_mutation(who, account);
					*maybe_account = maybe_account_maybe_dust.0;
					(maybe_endowed, maybe_account_maybe_dust.1, result)
				})
			});
			result.map(|(maybe_endowed, maybe_dust, result)| {
				if let Some(endowed) = maybe_endowed {
					Self::deposit_event(Event::Endowed(who.clone(), endowed));
				}
				let dust_cleaner = DustCleaner(maybe_dust.map(|dust| (who.clone(), dust)));
				(result, dust_cleaner)
			})
		}

		/// Update the account entry for `who`, given the locks.
		fn update_locks(who: &T::AccountId, locks: &[BalanceLock<T::Balance, T::BlockNumber>]) {
			if locks.len() as u32 > T::MaxLocks::get() {
				log::warn!(
					target: "runtime::balances",
					"Warning: A user has more currency locks than expected. \
					A runtime configuration adjustment may be needed."
				);
			}

			let existed = Locks::<T, I>::contains_key(who);
			if locks.is_empty() {
				Locks::<T, I>::remove(who);
				if existed {
					// TODO: use Locks::<T, I>::hashed_key
					// https://github.com/paritytech/substrate/issues/4969
					<frame_system::Pallet<T>>::dec_consumers(who);
				}
			} else {
				Locks::<T, I>::insert(who, locks);
				if !existed {
					if <frame_system::Pallet<T>>::inc_consumers(who).is_err() {
						// No providers for the locks. This is impossible under normal circumstances
						// since the funds that are under the lock will themselves be stored in the
						// account and therefore will need a reference.
						log::warn!(
							target: "runtime::balances",
							"Warning: Attempt to introduce lock consumer reference, yet no providers. \
							This is unexpected but should be safe."
						);
					}
				}
			}
		}
	}

	impl<T: Config<I>, I: 'static> Currency<T::AccountId> for Pallet<T, I>
	where
		T::Balance: MaybeSerializeDeserialize + Debug,
	{
		type Balance = T::Balance;
		type PositiveImbalance = PositiveImbalance<T, I>;
		type NegativeImbalance = NegativeImbalance<T, I>;

		fn total_balance(who: &T::AccountId) -> Self::Balance {
			let account = Self::account(who);
			account.total()
		}

		// Check if `value` amount of free balance can be slashed from `who`.
		fn can_slash(who: &T::AccountId, value: Self::Balance) -> bool {
			if value.is_zero() {
				return true;
			}
			Self::free_balance(who) >= value
		}

		fn total_issuance() -> Self::Balance {
			<TotalIssuance<T, I>>::get()
		}

		fn minimum_balance() -> Self::Balance {
			T::ExistentialDeposit::get()
		}

		// Burn funds from the total issuance, returning a positive imbalance for the amount burned.
		// Is a no-op if amount to be burned is zero.
		fn burn(mut amount: Self::Balance) -> Self::PositiveImbalance {
			if amount.is_zero() {
				return PositiveImbalance::zero();
			}
			<TotalIssuance<T, I>>::mutate(|issued| {
				*issued = issued.checked_sub(&amount).unwrap_or_else(|| {
					amount = *issued;
					Zero::zero()
				});
			});
			PositiveImbalance::new(amount)
		}

		// Create new funds into the total issuance, returning a negative imbalance
		// for the amount issued.
		// Is a no-op if amount to be issued it zero.
		fn issue(mut amount: Self::Balance) -> Self::NegativeImbalance {
			if amount.is_zero() {
				return NegativeImbalance::zero();
			}
			<TotalIssuance<T, I>>::mutate(|issued| {
				*issued = issued.checked_add(&amount).unwrap_or_else(|| {
					amount = Self::Balance::max_value() - *issued;
					Self::Balance::max_value()
				})
			});
			NegativeImbalance::new(amount)
		}

		fn free_balance(who: &T::AccountId) -> Self::Balance {
			Self::account(who).free()
		}

		// Ensure that an account can withdraw from their free balance given any existing withdrawal
		// restrictions like locks and vesting balance.
		// Is a no-op if amount to be withdrawn is zero.
		//
		// # <weight>
		// Despite iterating over a list of locks, they are limited by the number of
		// lock IDs, which means the number of runtime modules that intend to use and create locks.
		// # </weight>
		fn ensure_can_withdraw(
			who: &T::AccountId,
			amount: T::Balance,
			reasons: WithdrawReasons,
			new_balance: T::Balance,
		) -> DispatchResult {
			if amount.is_zero() {
				return Ok(());
			}
			let min_balance = Self::frozen_balance(who.borrow()).frozen_for(reasons.into());
			ensure!(
				new_balance >= min_balance,
				<Error<T, I>>::LiquidityRestrictions
			);
			Ok(())
		}

		// Transfer some free balance from `transactor` to `dest`, respecting existence requirements.
		// Is a no-op if value to be transferred is zero or the `transactor` is the same as `dest`.
		fn transfer(
			transactor: &T::AccountId,
			dest: &T::AccountId,
			value: Self::Balance,
			existence_requirement: ExistenceRequirement,
		) -> DispatchResult {
			if value.is_zero() || transactor == dest {
				return Ok(());
			}

			Self::try_mutate_account_with_dust(
				dest,
				|to_account, _| -> Result<DustCleaner<T, I>, DispatchError> {
					Self::try_mutate_account_with_dust(
						transactor,
						|from_account, _| -> DispatchResult {
							from_account.set_free(
								from_account
									.free()
									.checked_sub(&value)
									.ok_or(<Error<T, I>>::InsufficientBalance)?,
							);

							// NOTE: total stake being stored in the same type means that this could never overflow
							// but better to be safe than sorry.
							to_account.set_free(
								to_account
									.free()
									.checked_add(&value)
									.ok_or(<Error<T, I>>::Overflow)?,
							);

							let ed = T::ExistentialDeposit::get();
							ensure!(
								to_account.total() >= ed || !T::OtherCurrencies::is_dust(dest),
								<Error<T, I>>::ExistentialDeposit
							);

							Self::ensure_can_withdraw(
								transactor,
								value,
								WithdrawReasons::TRANSFER,
								from_account.free(),
							)
							.map_err(|_| <Error<T, I>>::LiquidityRestrictions)?;

							// TODO: This is over-conservative. There may now be other providers, and this module
							//   may not even be a provider.
							let allow_death =
								existence_requirement == ExistenceRequirement::AllowDeath;
							let allow_death = allow_death
								&& !<frame_system::Pallet<T>>::is_provider_required(transactor);
							ensure!(
								allow_death
									|| from_account.free() >= ed || !T::OtherCurrencies::is_dust(
									transactor
								),
								<Error<T, I>>::KeepAlive
							);

							Ok(())
						},
					)
					.map(|(_, maybe_dust_cleaner)| maybe_dust_cleaner)
				},
			)?;

			// Emit transfer event.
			Self::deposit_event(Event::Transfer(transactor.clone(), dest.clone(), value));

			Ok(())
		}

		/// Slash a target account `who`, returning the negative imbalance created and any left over
		/// amount that could not be slashed.
		///
		/// Is a no-op if `value` to be slashed is zero or the account does not exist.
		///
		/// NOTE: `slash()` prefers free balance, but assumes that reserve balance can be drawn
		/// from in extreme circumstances. `can_slash()` should be used prior to `slash()` to avoid having
		/// to draw from reserved funds, however we err on the side of punishment if things are inconsistent
		/// or `can_slash` wasn't used appropriately.
		fn slash(
			who: &T::AccountId,
			value: Self::Balance,
		) -> (Self::NegativeImbalance, Self::Balance) {
			if value.is_zero() {
				return (NegativeImbalance::zero(), Zero::zero());
			}
			if Self::total_balance(&who).is_zero() {
				return (NegativeImbalance::zero(), value);
			}

			for attempt in 0..2 {
				match Self::try_mutate_account(
					who,
					|account,
					 _is_new|
					 -> Result<(Self::NegativeImbalance, Self::Balance), StoredMapError> {
						// Best value is the most amount we can slash following liveness rules.
						let best_value = match attempt {
							// First attempt we try to slash the full amount, and see if liveness issues happen.
							0 => value,
							// If acting as a critical provider (i.e. first attempt failed), then slash
							// as much as possible while leaving at least at ED.
							_ => value.min(
								(account.free() + account.reserved())
									.saturating_sub(T::ExistentialDeposit::get()),
							),
						};

						let free_slash = cmp::min(account.free(), best_value);
						account.set_free(account.free() - free_slash); // Safe because of above check
						let remaining_slash = best_value - free_slash; // Safe because of above check

						if !remaining_slash.is_zero() {
							// If we have remaining slash, take it from reserved balance.
							let reserved_slash = cmp::min(account.reserved(), remaining_slash);
							account.set_reserved(account.reserved() - reserved_slash); // Safe because of above check
							Ok((
								NegativeImbalance::new(free_slash + reserved_slash),
								value - free_slash - reserved_slash, // Safe because value is gt or eq total slashed
							))
						} else {
							// Else we are done!
							Ok((
								NegativeImbalance::new(free_slash),
								value - free_slash, // Safe because value is gt or eq to total slashed
							))
						}
					},
				) {
					Ok(r) => return r,
					Err(_) => (),
				}
			}

			// Should never get here. But we'll be defensive anyway.
			(Self::NegativeImbalance::zero(), value)
		}

		/// Deposit some `value` into the free balance of an existing target account `who`.
		///
		/// Is a no-op if the `value` to be deposited is zero.
		fn deposit_into_existing(
			who: &T::AccountId,
			value: Self::Balance,
		) -> Result<Self::PositiveImbalance, DispatchError> {
			if value.is_zero() {
				return Ok(PositiveImbalance::zero());
			}

			Self::try_mutate_account(
				who,
				|account, is_new| -> Result<Self::PositiveImbalance, DispatchError> {
					ensure!(
						!is_new || !T::OtherCurrencies::is_dust(who),
						<Error<T, I>>::DeadAccount
					);
					account.set_free(
						account
							.free()
							.checked_add(&value)
							.ok_or(<Error<T, I>>::Overflow)?,
					);
					Ok(PositiveImbalance::new(value))
				},
			)
		}

		/// Deposit some `value` into the free balance of `who`, possibly creating a new account.
		///
		/// This function is a no-op if:
		/// - the `value` to be deposited is zero; or
		/// - the `value` to be deposited is less than the required ED and the account does not yet exist; or
		/// - the deposit would necessitate the account to exist and there are no provider references; or
		/// - `value` is so large it would cause the balance of `who` to overflow.
		fn deposit_creating(who: &T::AccountId, value: Self::Balance) -> Self::PositiveImbalance {
			if value.is_zero() {
				return Self::PositiveImbalance::zero();
			}

			Self::try_mutate_account(
				who,
				|account, is_new| -> Result<Self::PositiveImbalance, DispatchError> {
					let ed = T::ExistentialDeposit::get();
					ensure!(
						value >= ed || !is_new || !T::OtherCurrencies::is_dust(who),
						<Error<T, I>>::ExistentialDeposit
					);

					// defensive only: overflow should never happen, however in case it does, then this
					// operation is a no-op.
					account.set_free(match account.free().checked_add(&value) {
						Some(x) => x,
						None => return Ok(Self::PositiveImbalance::zero()),
					});

					Ok(PositiveImbalance::new(value))
				},
			)
			.unwrap_or_else(|_| Self::PositiveImbalance::zero())
		}

		/// Withdraw some free balance from an account, respecting existence requirements.
		///
		/// Is a no-op if value to be withdrawn is zero.
		fn withdraw(
			who: &T::AccountId,
			value: Self::Balance,
			reasons: WithdrawReasons,
			liveness: ExistenceRequirement,
		) -> Result<Self::NegativeImbalance, DispatchError> {
			if value.is_zero() {
				return Ok(NegativeImbalance::zero());
			}

			Self::try_mutate_account(
				who,
				|account, _| -> Result<Self::NegativeImbalance, DispatchError> {
					let new_free_account = account
						.free()
						.checked_sub(&value)
						.ok_or(<Error<T, I>>::InsufficientBalance)?;

					// bail if we need to keep the account alive and this would kill it.
					let ed = T::ExistentialDeposit::get();
					let others_is_dust = T::OtherCurrencies::is_dust(who);
					let would_be_dead = {
						let new_total = new_free_account + account.reserved();
						new_total < ed && others_is_dust
					};
					let would_kill = {
						let old_total = account.free() + account.reserved();
						would_be_dead && (old_total >= ed || !others_is_dust)
					};
					ensure!(
						liveness == ExistenceRequirement::AllowDeath || !would_kill,
						<Error<T, I>>::KeepAlive
					);

					Self::ensure_can_withdraw(who, value, reasons, new_free_account)?;

					account.set_free(new_free_account);

					Ok(NegativeImbalance::new(value))
				},
			)
		}

		/// Force the new free balance of a target account `who` to some new value `balance`.
		fn make_free_balance_be(
			who: &T::AccountId,
			value: Self::Balance,
		) -> SignedImbalance<Self::Balance, Self::PositiveImbalance> {
			Self::try_mutate_account(
				who,
				|account,
				 is_new|
				 -> Result<
					SignedImbalance<Self::Balance, Self::PositiveImbalance>,
					DispatchError,
				> {
					let ed = T::ExistentialDeposit::get();
					let total = value.saturating_add(account.reserved());
					// If we're attempting to set an existing account to less than ED, then
					// bypass the entire operation. It's a no-op if you follow it through, but
					// since this is an instance where we might account for a negative imbalance
					// (in the dust cleaner of set_account) before we account for its actual
					// equal and opposite cause (returned as an Imbalance), then in the
					// instance that there's no other accounts on the system at all, we might
					// underflow the issuance and our arithmetic will be off.
					ensure!(
						total >= ed || !is_new || !T::OtherCurrencies::is_dust(who),
						<Error<T, I>>::ExistentialDeposit
					);

					let imbalance = if account.free() <= value {
						SignedImbalance::Positive(PositiveImbalance::new(value - account.free()))
					} else {
						SignedImbalance::Negative(NegativeImbalance::new(account.free() - value))
					};
					account.set_free(value);
					Ok(imbalance)
				},
			)
			.unwrap_or(SignedImbalance::Positive(Self::PositiveImbalance::zero()))
		}
	}

	impl<T: Config<I>, I: 'static> ReservableCurrency<T::AccountId> for Pallet<T, I>
	where
		T::Balance: MaybeSerializeDeserialize + Debug,
	{
		/// Check if `who` can reserve `value` from their free balance.
		///
		/// Always `true` if value to be reserved is zero.
		fn can_reserve(who: &T::AccountId, value: Self::Balance) -> bool {
			if value.is_zero() {
				return true;
			}
			Self::account(who)
				.free()
				.checked_sub(&value)
				.map_or(false, |new_balance| {
					Self::ensure_can_withdraw(who, value, WithdrawReasons::RESERVE, new_balance)
						.is_ok()
				})
		}

		/// Slash from reserved balance, returning the negative imbalance created,
		/// and any amount that was unable to be slashed.
		///
		/// Is a no-op if the value to be slashed is zero or the account does not exist.
		fn slash_reserved(
			who: &T::AccountId,
			value: Self::Balance,
		) -> (Self::NegativeImbalance, Self::Balance) {
			if value.is_zero() {
				return (NegativeImbalance::zero(), Zero::zero());
			}
			if Self::total_balance(&who).is_zero() {
				return (NegativeImbalance::zero(), value);
			}

			// NOTE: `mutate_account` may fail if it attempts to reduce the balance to the point that an
			//   account is attempted to be illegally destroyed.

			for attempt in 0..2 {
				match Self::mutate_account(who, |account| {
					let best_value = match attempt {
						0 => value,
						// If acting as a critical provider (i.e. first attempt failed), then ensure
						// slash leaves at least the ED.
						_ => value.min(
							(account.free() + account.reserved())
								.saturating_sub(T::ExistentialDeposit::get()),
						),
					};

					let actual = cmp::min(account.reserved(), best_value);
					account.set_reserved(account.reserved() - actual);

					// underflow should never happen, but it if does, there's nothing to be done here.
					(NegativeImbalance::new(actual), value - actual)
				}) {
					Ok(r) => return r,
					Err(_) => (),
				}
			}
			// Should never get here as we ensure that ED is left in the second attempt.
			// In case we do, though, then we fail gracefully.
			(Self::NegativeImbalance::zero(), value)
		}

		fn reserved_balance(who: &T::AccountId) -> Self::Balance {
			let account = Self::account(who);
			account.reserved()
		}

		/// Move `value` from the free balance from `who` to their reserved balance.
		///
		/// Is a no-op if value to be reserved is zero.
		fn reserve(who: &T::AccountId, value: Self::Balance) -> DispatchResult {
			if value.is_zero() {
				return Ok(());
			}

			Self::try_mutate_account(who, |account, _| -> DispatchResult {
				let new_free = account
					.free()
					.checked_sub(&value)
					.ok_or(<Error<T, I>>::InsufficientBalance)?;
				account.set_free(new_free);

				let new_reserved = account
					.reserved()
					.checked_add(&value)
					.ok_or(<Error<T, I>>::Overflow)?;
				account.set_reserved(new_reserved);
				Self::ensure_can_withdraw(
					&who,
					value.clone(),
					WithdrawReasons::RESERVE,
					account.free(),
				)
			})?;

			Self::deposit_event(Event::Reserved(who.clone(), value));
			Ok(())
		}

		/// Unreserve some funds, returning any amount that was unable to be unreserved.
		///
		/// Is a no-op if the value to be unreserved is zero.
		fn unreserve(who: &T::AccountId, value: Self::Balance) -> Self::Balance {
			if value.is_zero() {
				return Zero::zero();
			}
			if Self::total_balance(&who).is_zero() {
				return value;
			}

			let actual = match Self::mutate_account(who, |account| {
				let actual = cmp::min(account.reserved(), value);
				let new_reserved = account.reserved() - actual;
				account.set_reserved(new_reserved);
				// defensive only: this can never fail since total issuance which is at least free+reserved
				// fits into the same data type.
				account.set_free(account.free().saturating_add(actual));
				actual
			}) {
				Ok(x) => x,
				Err(_) => {
					// This should never happen since we don't alter the total amount in the account.
					// If it ever does, then we should fail gracefully though, indicating that nothing
					// could be done.
					return value;
				}
			};

			Self::deposit_event(Event::Unreserved(who.clone(), actual.clone()));
			value - actual
		}

		/// Move the reserved balance of one account into the balance of another, according to `status`.
		///
		/// Is a no-op if:
		/// - the value to be moved is zero; or
		/// - the `slashed` id equal to `beneficiary` and the `status` is `Reserved`.
		fn repatriate_reserved(
			slashed: &T::AccountId,
			beneficiary: &T::AccountId,
			value: Self::Balance,
			status: BalanceStatus,
		) -> Result<Self::Balance, DispatchError> {
			if value.is_zero() {
				return Ok(Zero::zero());
			}

			if slashed == beneficiary {
				return match status {
					BalanceStatus::Free => Ok(Self::unreserve(slashed, value)),
					BalanceStatus::Reserved => {
						Ok(value.saturating_sub(Self::reserved_balance(slashed)))
					}
				};
			}

			let ((actual, _maybe_one_dust), _maybe_other_dust) =
				Self::try_mutate_account_with_dust(
					beneficiary,
					|to_account,
					 is_new|
					 -> Result<(Self::Balance, DustCleaner<T, I>), DispatchError> {
						ensure!(
							!is_new || !T::OtherCurrencies::is_dust(beneficiary),
							<Error<T, I>>::DeadAccount
						);
						Self::try_mutate_account_with_dust(
							slashed,
							|from_account, _| -> Result<Self::Balance, DispatchError> {
								let actual = cmp::min(from_account.reserved(), value);
								match status {
									BalanceStatus::Free => to_account.set_free(
										to_account
											.free()
											.checked_add(&actual)
											.ok_or(<Error<T, I>>::Overflow)?,
									),
									BalanceStatus::Reserved => to_account.set_reserved(
										to_account
											.reserved()
											.checked_add(&actual)
											.ok_or(<Error<T, I>>::Overflow)?,
									),
								}
								from_account.set_reserved(from_account.reserved() - actual);
								Ok(actual)
							},
						)
					},
				)?;

			Self::deposit_event(Event::ReserveRepatriated(
				slashed.clone(),
				beneficiary.clone(),
				actual,
				status,
			));
			Ok(value - actual)
		}
	}

	impl<T: Config<I>, I: 'static> LockableCurrency<T::AccountId> for Pallet<T, I>
	where
		T::Balance: MaybeSerializeDeserialize + Debug,
	{
		type Moment = T::BlockNumber;
		type MaxLocks = T::MaxLocks;

		// Set a lock on the balance of `who`.
		// Is a no-op if lock amount is zero or `reasons` `is_none()`.
		fn set_lock(
			id: LockIdentifier,
			who: &T::AccountId,
			lock_for: LockFor<Self::Balance, Self::Moment>,
			reasons: WithdrawReasons,
		) {
			if match &lock_for {
				LockFor::Common { amount } => *amount,
				LockFor::Staking(staking_lock) => {
					staking_lock.locked_amount(<frame_system::Pallet<T>>::block_number())
				}
			}
			.is_zero() || reasons.is_empty()
			{
				return;
			}
			let mut new_lock = Some(BalanceLock {
				id,
				lock_for,
				lock_reasons: reasons.into(),
			});
			let mut locks = Self::locks(who)
				.into_iter()
				.filter_map(|l| if l.id == id { new_lock.take() } else { Some(l) })
				.collect::<Vec<_>>();
			if let Some(lock) = new_lock {
				locks.push(lock)
			}
			Self::update_locks(who, &locks);
		}

		// Extend a lock on the balance of `who`.
		// Is a no-op if lock amount is zero or `reasons` `is_none()`.
		fn extend_lock(
			id: LockIdentifier,
			who: &T::AccountId,
			amount: T::Balance,
			reasons: WithdrawReasons,
		) -> DispatchResult {
			if amount.is_zero() || reasons.is_empty() {
				return Ok(());
			}

			let mut new_lock = Some(BalanceLock {
				id,
				lock_for: LockFor::Common { amount },
				lock_reasons: reasons.into(),
			});
			let mut poisoned = false;
			let mut locks = Self::locks(who)
				.into_iter()
				.filter_map(|l| {
					if l.id == id {
						if let LockFor::Common { amount: a } = l.lock_for {
							new_lock.take().map(|nl| BalanceLock {
								id: l.id,
								lock_for: {
									match nl.lock_for {
										// Only extend common lock type
										LockFor::Common { amount: na } => LockFor::Common {
											amount: (a).max(na),
										},
										// Not allow to extend other combination/type lock
										//
										// And the lock is always with lock id
										// it's impossiable to match a (other lock, common lock)
										// under this if condition
										_ => {
											poisoned = true;

											nl.lock_for
										}
									}
								},
								lock_reasons: l.lock_reasons | nl.lock_reasons,
							})
						} else {
							Some(l)
						}
					} else {
						Some(l)
					}
				})
				.collect::<Vec<_>>();

			if poisoned {
				Err(<Error<T, I>>::LockP)?;
			}

			if let Some(lock) = new_lock {
				locks.push(lock)
			}

			Self::update_locks(who, &locks);

			Ok(())
		}

		fn remove_lock(id: LockIdentifier, who: &T::AccountId) {
			let mut locks = Self::locks(who);

			locks.retain(|l| l.id != id);

			Self::update_locks(who, &locks);
		}

		/// Get the balance of an account that can be used for transfers, reservations, or any other
		/// non-locking, non-transaction-fee activity. Will be at most `free_balance`.
		fn usable_balance(who: &T::AccountId) -> Self::Balance {
			let account = Self::account(who);

			account.usable(LockReasons::Misc, Self::frozen_balance(who))
		}

		/// Get the balance of an account that can be used for paying transaction fees (not tipping,
		/// or any other kind of fees, though). Will be at most `free_balance`.
		fn usable_balance_for_fees(who: &T::AccountId) -> Self::Balance {
			let account = Self::account(who);

			account.usable(LockReasons::Fee, Self::frozen_balance(who))
		}
	}

	impl<T: Config<I>, I: 'static> DustCollector<T::AccountId> for Pallet<T, I> {
		fn is_dust(who: &T::AccountId) -> bool {
			let total = Self::total_balance(who);

			total < T::ExistentialDeposit::get() || total.is_zero()
		}

		fn collect(who: &T::AccountId) {
			let dropped = Self::total_balance(who);

			if !dropped.is_zero() {
				T::DustRemoval::on_unbalanced(NegativeImbalance::new(dropped));
				if let Err(e) = <frame_system::Pallet<T>>::dec_providers(who) {
					log::error!("Logic error: Unexpected {:?}", e);
				}
				Self::deposit_event(Event::DustLost(who.clone(), dropped));
			}
		}
	}

	// A value placed in storage that represents the current version of the Balances storage.
	// This value is used by the `on_runtime_upgrade` logic to determine whether we run
	// storage migration logic. This should match directly with the semantic versions of the Rust crate.
	#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug)]
	pub enum Releases {
		V1_0_0,
		V2_0_0,
	}
	impl Default for Releases {
		fn default() -> Self {
			Releases::V1_0_0
		}
	}

	pub struct DustCleaner<T: Config<I>, I: 'static = ()>(
		Option<(T::AccountId, NegativeImbalance<T, I>)>,
	);
	impl<T: Config<I>, I: 'static> Drop for DustCleaner<T, I> {
		fn drop(&mut self) {
			if let Some((who, dust)) = self.0.take() {
				if T::OtherCurrencies::is_dust(&who) {
					T::OtherCurrencies::collect(&who);

					<Pallet<T, I>>::deposit_event(Event::DustLost(who, dust.peek()));
					T::DustRemoval::on_unbalanced(dust);
				}
			}
		}
	}
}
pub use pallet::{imbalances::*, *};

pub mod migration {
	#[cfg(feature = "try-runtime")]
	pub mod try_runtime {
		// --- substrate ---
		use frame_support::{pallet_prelude::*, traits::StorageInstance};
		// --- darwinia ---
		use crate::*;

		macro_rules! generate_storage_types {
			($prefix:expr, $storage:expr, $name:ident => Value<$value:ty>) => {
				paste::paste! {
					type $name = StorageValue<[<$name Instance>], $value, ValueQuery>;

					struct [<$name Instance>];
					impl StorageInstance for [<$name Instance>] {
						const STORAGE_PREFIX: &'static str = $storage;

						fn pallet_prefix() -> &'static str { $prefix }
					}
				}
			};
		}

		generate_storage_types!("Instance0DarwiniaBalances", "TotalIssuance", OldRingTotalIssuance => Value<()>);
		generate_storage_types!("Balances", "TotalIssuance", NewRingTotalIssuance => Value<()>);
		generate_storage_types!("Instance1DarwiniaBalances", "TotalIssuance", OldKtonTotalIssuance => Value<()>);
		generate_storage_types!("Kton", "TotalIssuance", NewKtonTotalIssuance => Value<()>);

		pub fn pre_migrate() -> Result<(), &'static str> {
			assert!(OldRingTotalIssuance::exists());
			assert!(!NewRingTotalIssuance::exists());
			assert!(OldKtonTotalIssuance::exists());
			assert!(!NewKtonTotalIssuance::exists());

			log::info!("Migrating `Instance0Darwinia` to `Balances`...");
			migration::migrate(b"Instance0DarwiniaBalances", b"Balances");

			log::info!("Migrating `Instance1Darwinia` to `Kton`...");
			migration::migrate(b"Instance1DarwiniaBalances", b"Kton");

			assert!(!OldRingTotalIssuance::exists());
			assert!(NewRingTotalIssuance::exists());
			assert!(!OldKtonTotalIssuance::exists());
			assert!(NewKtonTotalIssuance::exists());

			Ok(())
		}
	}

	pub fn migrate(old_pallet_name: &[u8], new_pallet_name: &[u8]) {
		// Balances: Instance0DarwiniaBalances
		// Kton: Instance1DarwiniaBalances
		frame_support::migration::move_pallet(old_pallet_name, new_pallet_name);
	}
}
