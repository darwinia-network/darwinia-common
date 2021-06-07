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

#![allow(unused)]
#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "128"]

pub mod weights;
pub use weights::WeightInfo;

// --- crates ---
use ethereum_primitives::EthereumAddress;
use ethereum_types::{Address, H160, H256, U256};
// --- substrate ---
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage, ensure,
	pallet_prelude::*,
	parameter_types,
	traits::{Currency, ExistenceRequirement::*, Get},
	weights::Weight,
	PalletId,
};
use frame_system::{ensure_signed, pallet_prelude::*};
use sp_runtime::traits::UniqueSaturatedInto;
use sp_runtime::{
	traits::{AccountIdConversion, Dispatchable, Saturating, Zero},
	DispatchError, SaturatedConversion,
};
use sp_std::{convert::TryFrom, prelude::*, vec::Vec};
// --- darwinia ---
use darwinia_asset_primitives::token::{Token, TokenInfo, TokenOption};
use darwinia_primitives_contract::mapping_token_factory::MappingTokenFactory as mtf;
use darwinia_relay_primitives::{Relay, RelayAccount};
use darwinia_support::{
	balance::*,
	s2s::{
		to_bytes32, ChainSelector as TargetChain, BACK_ERC20_RING, RING_DECIMAL, RING_NAME,
		RING_SYMBOL,
	},
};

pub type AccountId<T> = <T as frame_system::Config>::AccountId;
pub type Balance = u128;
pub type RingBalance<T> = <<T as Config>::RingCurrency as Currency<AccountId<T>>>::Balance;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type WeightInfo: WeightInfo;
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		#[pallet::constant]
		type FeePalletId: Get<PalletId>;
		#[pallet::constant]
		type RingLockMaxLimit: Get<RingBalance<Self>>;
		#[pallet::constant]
		type AdvancedFee: Get<RingBalance<Self>>;
		type RingCurrency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;
		// now we have only one relay,
		// in future, we modify it to multi-instance to interact with multi-target chains
		type IssuingRelay: Relay<
			RelayProof = AccountId<Self>,
			VerifiedResult = Result<(EthereumAddress, TargetChain), DispatchError>,
			RelayMessage = (TargetChain, Token, RelayAccount<AccountId<Self>>),
			RelayMessageResult = Result<(), DispatchError>,
		>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	#[pallet::metadata(T::AccountId = "AccountId")]
	pub enum Event<T: Config> {
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
		/// Invalid token option
		InvalidTokenOption,
		/// Insufficient balance
		InsufficientBalance,
		/// Ring Lock LIMITED
		RingLockLimited,
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub backed_ring: RingBalance<T>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self {
				backed_ring: Default::default(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Lock token in this chain and cross transfer to the target chain
		///
		/// Target is the id of the target chain defined in s2s_chain pallet
		// TODO: update the weight
		#[pallet::weight(0)]
		#[frame_support::transactional]
		pub fn lock_and_cross_send(
			origin: OriginFor<T>,
			target: TargetChain,
			#[pallet::compact] value: RingBalance<T>,
			recipient: EthereumAddress,
		) -> DispatchResultWithPostInfo {
			// TODO: Do we need to check the target chain? and make sure this id is a truly bridged chain
			let user = ensure_signed(origin)?;

			// Make sure the locked value is less than the max lock limited
			ensure!(
				value < T::RingLockMaxLimit::get() && !value.is_zero(),
				<Error<T>>::RingLockLimited
			);
			// Make sure the user's balance is enough to lock
			ensure!(
				T::RingCurrency::free_balance(&user) > value + T::AdvancedFee::get(),
				<Error<T>>::InsufficientBalance
			);

			// Pay some fee and lock token
			let fee_account = Self::fee_account_id();
			T::RingCurrency::transfer(&user, &fee_account, T::AdvancedFee::get(), KeepAlive)?;
			T::RingCurrency::transfer(&user, &Self::pallet_account_id(), value, AllowDeath)?;

			// Send to the target chain
			let amount: U256 = value.saturated_into::<u128>().into();
			let token = Token::Native(TokenInfo {
				// The native mapped RING token as a special ERC20 address
				address: array_bytes::hex2array_unchecked!(BACK_ERC20_RING, 20).into(),
				value: Some(amount),
				option: Some(TokenOption {
					name: to_bytes32(RING_NAME),
					symbol: to_bytes32(RING_SYMBOL),
					decimal: RING_DECIMAL,
				}),
			});

			let message = (
				target,
				token.clone(),
				RelayAccount::EthereumAccount(recipient),
			);
			T::IssuingRelay::relay_message(&message);
			Self::deposit_event(Event::TokenLocked(token, user, recipient, amount));
			Ok(().into())
		}

		/// Receive target chain locked message and unlock token in this chain.
		// TODO: update the weight
		#[pallet::weight(0)]
		pub fn cross_receive_and_unlock(
			origin: OriginFor<T>,
			message: (Token, AccountId<T>),
		) -> DispatchResultWithPostInfo {
			let user = ensure_signed(origin)?;

			// the s2s message relay has been verified the message comes from the issuing pallet with the
			// chainID and issuing sender address.
			// here only we need is to check the sender is in whitelist
			let backing = T::IssuingRelay::verify(&user)?;
			let (token, recipient) = message;

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
			let amount = match token_info.value {
				Some(value) => value,
				_ => return Err(<Error<T>>::InvalidTokenType.into()),
			};

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

			Self::deposit_event(Event::TokenUnlocked(token, recipient, amount));
			Ok(().into())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn pallet_account_id() -> T::AccountId {
			T::PalletId::get().into_account()
		}

		pub fn fee_account_id() -> T::AccountId {
			T::FeePalletId::get().into_account()
		}
	}
}
