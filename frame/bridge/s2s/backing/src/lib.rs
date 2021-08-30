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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

//! Prototype module for s2s cross chain assets backing.

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "128"]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod tests;

pub mod weight;
pub use weight::WeightInfo;

// --- crates.io ---
use ethereum_primitives::EthereumAddress;
use ethereum_types::{H256, U256};
// --- paritytech ---
use bp_runtime::{ChainId, Size};
use frame_support::{
	ensure,
	pallet_prelude::*,
	traits::{Currency, ExistenceRequirement::*, Get},
	transactional, PalletId,
};
use frame_system::{ensure_signed, pallet_prelude::*};
use sp_runtime::{
	traits::{AccountIdConversion, Convert, UniqueSaturatedInto, Zero},
	SaturatedConversion,
};
use sp_std::prelude::*;
// --- darwinia-network ---
use darwinia_support::{
	evm::IntoDvmAddress,
	s2s::{
		ensure_source_root, to_bytes32, RelayMessageCaller, RING_DECIMAL, RING_NAME, RING_SYMBOL,
	},
};
use dp_asset::{
	token::{Token, TokenInfo, TokenOption},
	RecipientAccount,
};

const BLOCKS_PER_DAY: u128 = 14_400;

pub type AccountId<T> = <T as frame_system::Config>::AccountId;
pub type Balance = u128;
pub type RingBalance<T> = <<T as Config>::RingCurrency as Currency<AccountId<T>>>::Balance;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_timestamp::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type WeightInfo: WeightInfo;

		#[pallet::constant]
		type PalletId: Get<PalletId>;
		#[pallet::constant]
		type RingPalletId: Get<PalletId>;
		#[pallet::constant]
		type RingLockMaxLimit: Get<RingBalance<Self>>;
		#[pallet::constant]
		type DailyMaxTransferLimit: Get<U256>;
		type RingCurrency: Currency<AccountId<Self>>;

		type BridgedAccountIdConverter: Convert<H256, Self::AccountId>;
		type BridgedChainId: Get<ChainId>;

		type OutboundPayload: Parameter + Size;
		type CallEncoder: EncodeCall<Self::AccountId, Self::OutboundPayload>;

		type FeeAccount: Get<Option<Self::AccountId>>;
		type MessageSender: RelayMessageCaller<Self::OutboundPayload, RingBalance<Self>>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	#[pallet::metadata(T::AccountId = "AccountId")]
	pub enum Event<T: Config> {
		/// Token registered [token address, sender]
		TokenRegistered(Token, AccountId<T>),
		/// Token locked [token address, sender, recipient, amount]
		TokenLocked(Token, AccountId<T>, EthereumAddress, U256),
		/// Token unlocked [token, recipient, value]
		TokenUnlocked(Token, AccountId<T>, U256),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Currently we only support native token transfer comes from s2s bridge
		Erc20NotSupported,
		/// Invalid token type
		InvalidTokenType,
		/// Invalid token value
		InvalidTokenAmount,
		/// Insufficient balance
		InsufficientBalance,
		/// Ring Lock LIMITED
		RingLockLimited,
		/// Redeem Daily Limited
		RedeemDailyLimited,
		/// invalid source origin
		InvalidOrigin,
		/// encode dispatch call failed
		EncodeInvalid,
		/// send relay message failed
		SendMessageFailed,
	}

	#[pallet::storage]
	#[pallet::getter(fn daily_unlocked)]
	pub type DailyUnlocked<T: Config> =
		StorageMap<_, Blake2_128Concat, EthereumAddress, Option<(u128, U256)>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn daily_limited)]
	pub type DailyLimited<T: Config> =
		StorageMap<_, Blake2_128Concat, EthereumAddress, U256, ValueQuery>;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(<T as Config>::WeightInfo::register_and_remote_create())]
		#[transactional]
		pub fn register_and_remote_create(
			origin: OriginFor<T>,
			spec_version: u32,
			weight: u64,
			fee: RingBalance<T>,
		) -> DispatchResultWithPostInfo {
			let user = ensure_signed(origin)?;

			if let Some(fee_account) = T::FeeAccount::get() {
				T::RingCurrency::transfer(&user, &fee_account, fee, KeepAlive)?;
			}
			let token = Token::Native(TokenInfo {
				address: T::RingPalletId::get().into_dvm_address(),
				value: None,
				option: Some(TokenOption {
					name: to_bytes32(RING_NAME),
					symbol: to_bytes32(RING_SYMBOL),
					decimal: RING_DECIMAL,
				}),
			});
			let payload =
				T::CallEncoder::encode_remote_register(spec_version, weight, token.clone());
			T::MessageSender::send_message(payload, fee).map_err(|e| {
				log::info!("s2s-backing: register token failed {:?}", e);
				Error::<T>::SendMessageFailed
			})?;
			Self::deposit_event(Event::TokenRegistered(token, user));
			Ok(().into())
		}

		/// Lock token in this chain and cross transfer to the target chain
		///
		/// Target is the id of the target chain defined in s2s_chain pallet
		#[pallet::weight(<T as Config>::WeightInfo::lock_and_remote_issue())]
		#[transactional]
		pub fn lock_and_remote_issue(
			origin: OriginFor<T>,
			spec_version: u32,
			weight: u64,
			#[pallet::compact] value: RingBalance<T>,
			#[pallet::compact] fee: RingBalance<T>,
			recipient: EthereumAddress,
		) -> DispatchResultWithPostInfo {
			let user = ensure_signed(origin)?;

			// Make sure the locked value is less than the max lock limited
			ensure!(
				value < T::RingLockMaxLimit::get() && !value.is_zero(),
				<Error<T>>::RingLockLimited
			);
			// Make sure the user's balance is enough to lock
			ensure!(
				T::RingCurrency::free_balance(&user) > value + fee,
				<Error<T>>::InsufficientBalance
			);

			if let Some(fee_account) = T::FeeAccount::get() {
				T::RingCurrency::transfer(&user, &fee_account, fee, KeepAlive)?;
			}
			T::RingCurrency::transfer(&user, &Self::pallet_account_id(), value, AllowDeath)?;

			// Send to the target chain
			let amount: U256 = value.saturated_into::<u128>().into();
			let token = Token::Native(TokenInfo {
				// The native mapped RING token as a special ERC20 address
				address: T::RingPalletId::get().into_dvm_address(),
				value: Some(amount),
				option: None,
			});

			let account = RecipientAccount::EthereumAccount(recipient);
			let payload =
				T::CallEncoder::encode_remote_issue(spec_version, weight, token.clone(), account)
					.map_err(|_| Error::<T>::EncodeInvalid)?;
			T::MessageSender::send_message(payload, fee)
				.map_err(|_| Error::<T>::SendMessageFailed)?;
			Self::deposit_event(Event::TokenLocked(token, user, recipient, amount));
			Ok(().into())
		}

		/// Receive target chain locked message and unlock token in this chain.
		#[pallet::weight(<T as Config>::WeightInfo::unlock_from_remote())]
		pub fn unlock_from_remote(
			origin: OriginFor<T>,
			token: Token,
			recipient: AccountId<T>,
		) -> DispatchResultWithPostInfo {
			let user = ensure_signed(origin)?;

			// the s2s message relay has been verified the message comes from the issuing pallet with the
			// chainID and issuing sender address.
			// here only we need is to check the sender is root account
			ensure_source_root::<T::AccountId, T::BridgedAccountIdConverter>(
				T::BridgedChainId::get(),
				&user,
			)?;

			let token_info = match &token {
				Token::Native(info) => {
					log::debug!("cross receive native token {:?}", info);
					info
				}
				Token::Erc20(info) => {
					log::debug!("cross receive erc20 token {:?}", info);
					return Err(Error::<T>::Erc20NotSupported.into());
				}
				_ => return Err(Error::<T>::InvalidTokenType.into()),
			};
			let amount = token_info.value.ok_or(<Error<T>>::InvalidTokenAmount)?;

			// Make sure the total transfered is less than daily limited
			let unlocked = Self::get_daily_unlocked(token_info.address);
			let daily_limited = <DailyLimited<T>>::get(token_info.address);
			ensure!(
				daily_limited.is_zero() || daily_limited.saturating_sub(unlocked) >= amount,
				<Error<T>>::RedeemDailyLimited
			);

			// Make sure the user's balance is enough to lock
			ensure!(
				T::RingCurrency::free_balance(&Self::pallet_account_id())
					> amount.low_u128().unique_saturated_into(),
				<Error<T>>::InsufficientBalance
			);
			T::RingCurrency::transfer(
				&Self::pallet_account_id(),
				&recipient,
				amount.low_u128().unique_saturated_into(),
				KeepAlive,
			)?;

			Self::update_daily_unlocked(token_info.address, amount.saturating_add(unlocked));
			Self::deposit_event(Event::TokenUnlocked(token.clone(), recipient, amount));
			Ok(().into())
		}

		#[pallet::weight(92_000_000)]
		pub fn update_daily_limited(
			origin: OriginFor<T>,
			token: EthereumAddress,
			limited: U256,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			DailyLimited::<T>::mutate(token, |old| {
				*old = limited;
			});
			Ok(().into())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn pallet_account_id() -> T::AccountId {
			T::PalletId::get().into_account()
		}

		// if this is the first time to transfer, then the daily_transfer storage is none, other
		// wise, it is <timestamp, transfered>, where timestamp is 00:00:00, the start of the day
		fn get_daily_unlocked(token: EthereumAddress) -> U256 {
			let now = UniqueSaturatedInto::<u128>::unique_saturated_into(
				frame_system::Pallet::<T>::block_number(),
			);
			DailyUnlocked::<T>::get(token).map_or_else(
				|| U256::zero(),
				|v| {
					if now - now % BLOCKS_PER_DAY > v.0 {
						U256::zero()
					} else {
						v.1
					}
				},
			)
		}

		fn update_daily_unlocked(token: EthereumAddress, balance: U256) {
			let now = UniqueSaturatedInto::<u128>::unique_saturated_into(
				frame_system::Pallet::<T>::block_number(),
			);
			DailyUnlocked::<T>::mutate(token, |v| {
				*v = Some((now - now % BLOCKS_PER_DAY, balance));
			});
		}
	}
}

/// Encode call
pub trait EncodeCall<AccountId, MessagePayload> {
	/// Encode issuing pallet remote_register call
	fn encode_remote_register(spec_version: u32, weight: u64, token: Token) -> MessagePayload;
	/// Encode issuing pallet remote_issue call
	fn encode_remote_issue(
		spec_version: u32,
		weight: u64,
		token: Token,
		recipient: RecipientAccount<AccountId>,
	) -> Result<MessagePayload, ()>;
}
