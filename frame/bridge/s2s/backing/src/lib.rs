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

pub mod weights;
pub use weights::WeightInfo;

const RING_NAME: &'static str = "0x44617277696e6961204e6574776f726b204e617469766520546f6b656e000000";
const RING_SYMBOL: &'static str = "0x52494e4700000000000000000000000000000000000000000000000000000000";

#[frame_support::pallet]
pub mod pallet {
    pub mod types {
        use crate::pallet::*;

        pub type AccountId<T> = <T as frame_system::Config>::AccountId;
        pub type Balance = u128;
        pub type BlockNumber<T> = <T as frame_system::Config>::BlockNumber;
        pub type RingBalance<T> = <<T as Config>::RingCurrency as Currency<AccountId<T>>>::Balance;
    }

	use frame_system::pallet_prelude::*;
    // --- crates ---
    use ethereum_types::{Address, H160, H256, U256};
    // --- substrate ---
    use frame_support::{
		pallet_prelude::*,
        decl_error, decl_event, decl_module, decl_storage,
        ensure, parameter_types,
        traits::{Currency, ExistenceRequirement::*, Get},
        weights::Weight,
        PalletId,
    };
    use frame_system::ensure_signed;
    use sp_runtime::{
        DispatchError, DispatchResult,
        traits::{Zero, AccountIdConversion, Saturating},
        SaturatedConversion,
    };
    use sp_std::vec::Vec;
    // --- darwinia ---
    use darwinia_primitives_contract::mapping_token_factory::MappingTokenFactory as mtf;
    use darwinia_asset_primitives::token::{Token, TokenInfo, TokenOption};
    use ethereum_primitives::EthereumAddress;
    use darwinia_relay_primitives::Relay;
    use darwinia_evm::AddressMapping;
	use darwinia_support::balance::*;
	use sp_std::{convert::TryFrom, prelude::*};

    use sp_runtime::traits::Dispatchable;

    use darwinia_s2s_chain::ChainSelector as TargetChain;

	use crate::weights::WeightInfo;

    use types::*;
    use crate::{RING_NAME, RING_SYMBOL};

	#[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		#[pallet::constant]
        type PalletId: Get<PalletId>;
		#[pallet::constant]
		type FeePalletId: Get<PalletId>;

        type WeightInfo: WeightInfo;
        type IssuingRelay: Relay<
            RelayProof = AccountId<Self>, 
            VerifiedResult = Result<EthereumAddress, DispatchError>, 
            RelayMessage=(TargetChain, Token, EthereumAddress),
            RelayMessageResult = Result<(), DispatchError>>;
		#[pallet::constant]
        type RingLockLimit: Get<RingBalance<Self>>;
		#[pallet::constant]
        type AdvancedFee: Get<RingBalance<Self>>;
		type RingCurrency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;
    }

    #[pallet::error]
	pub enum Error<T> {
        /// currently we only support native token transfered by s2s bridge
        Erc20NotSupported,
        /// invalid token type
        InvalidTokenType,
        /// invalid token option
        InvalidTokenOption,
        /// not enough ring balance
        NotEnoughRingBalance,
		/// Ring Lock - LIMITED
		RingLockLim,
	}

    #[pallet::event]
    #[pallet::generate_deposit(fn deposit_event)]
    #[pallet::metadata(
        AccountId<T> = "AccountId",
    )]
	pub enum Event<T: Config> {
        /// token locked [tokenaddress, sender, recipient, amount]
        TokenLocked(Token, AccountId<T>, EthereumAddress, U256),
        /// token unlocked [token, recipient, value]
        TokenUnlocked(Token, AccountId<T>, U256),
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumber<T>) -> Weight {
            0
		}
	}

	#[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Receive balance from issuing burn
		#[pallet::weight(0)]
        pub fn cross_receive(
            origin: OriginFor<T>,
            message: (Token, AccountId<T>)
        ) -> DispatchResultWithPostInfo {
			let user = ensure_signed(origin)?;
            // the s2s message relay has been verified this comes from the backing chain with the
            // chainID and backing sender address.
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
                    return Err(Error::<T>::Erc20NotSupported.into())
                }
                _ => {
                    return Err(Error::<T>::InvalidTokenType.into())
                }
            };
            let amount = match token_info.value {
                Some(value) => {
                    value.into()
                }
                _ => return Err(<Error<T>>::InvalidTokenType.into())
            };
            let unlock_amount = Balance::try_from(amount)?;
            Self::unlock_token_cast::<T::RingCurrency>(
                &recipient,
                unlock_amount,
            )?;
            Self::deposit_event(Event::TokenUnlocked(token, recipient, amount));
			Ok(().into())
        }

        /// lock token and cross transfer to the target chain
		#[pallet::weight(0)]
		#[frame_support::transactional]
        pub fn cross_send(
			origin: OriginFor<T>,
            target: TargetChain,
            #[pallet::compact] ring_to_lock: RingBalance<T>, 
            recipient: EthereumAddress,
		) -> DispatchResultWithPostInfo {
			let user = ensure_signed(origin)?;
			let fee_account = Self::fee_account_id();
            T::RingCurrency::transfer(
				&user,
				&fee_account,
				T::AdvancedFee::get(),
				KeepAlive,
			)?;

            ensure!(ring_to_lock < T::RingLockLimit::get()
                    && !ring_to_lock.is_zero(), <Error<T>>::RingLockLim);

            T::RingCurrency::transfer(
                &user,
                &Self::account_id(),
                ring_to_lock,
                AllowDeath,
                )?;

            let amount: Balance = ring_to_lock.saturated_into();
            let amount = U256::from(amount);

            let ring_name: [u8;32] = array_bytes::hex2array_unchecked!(RING_NAME, 32).into();
            let ring_symbol: [u8;32] = array_bytes::hex2array_unchecked!(RING_SYMBOL, 32).into();
            let token = Token::Native(TokenInfo {
                address: H160::zero(),
                value: Some(amount),
                option: Some(TokenOption {
                    name: ring_name,
                    symbol: ring_symbol,
                    decimal: 9,
                })
            });

            let message = (
                target,
                token.clone(),
                recipient);
            T::IssuingRelay::relay_message(&message);
            Self::deposit_event(Event::TokenLocked(token, user, recipient, amount));
			Ok(().into())
        }
	}

    impl<T: Config> Pallet<T> {
		pub fn account_id() -> T::AccountId {
			T::PalletId::get().into_account()
		}
		pub fn fee_account_id() -> T::AccountId {
			T::FeePalletId::get().into_account()
		}

		/// Return the amount of money in the pot.
		// The existential deposit is not part of the pot so backing account never gets deleted.
		pub fn pot<C: LockableCurrency<T::AccountId>>() -> C::Balance {
			C::usable_balance(&Self::account_id())
				// Must never be less than 0 but better be safe.
				.saturating_sub(C::minimum_balance())
		}

        fn unlock_token_cast<C: LockableCurrency<T::AccountId>>(
            recipient: &T::AccountId,
            amount: Balance,
            ) -> DispatchResult {
            let amount: C::Balance = amount.saturated_into();

            ensure!(
                Self::pot::<C>() >= amount,
                <Error<T>>::NotEnoughRingBalance
            );

            C::transfer(
                &Self::account_id(),
                &recipient,
                amount,
                KeepAlive,
                )?;
            Ok(())
        }
    }
}

