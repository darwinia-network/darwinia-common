// This file is part of Darwinia.
//
// Copyright (C) 2018-2022 Darwinia Network
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

//! Prototype module for s2s cross chain assets issuing.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod weight;
pub use weight::WeightInfo;

// --- crates.io ---
use ethereum_types::{H160, H256, U256};
// --- paritytech ---
use bp_messages::{source_chain::OnDeliveryConfirmed, DeliveredMessages, LaneId};
use frame_support::{
	ensure, log,
	pallet_prelude::*,
	traits::{Currency, Get},
	transactional, PalletId,
};
use frame_system::ensure_signed;
use sp_runtime::{traits::Convert, DispatchError};
use sp_std::{str, vec::Vec};
// --- darwinia-network ---
use bp_runtime::ChainId;
use darwinia_ethereum::InternalTransactHandler;
use darwinia_support::{
	mapping_token::*,
	s2s::{ensure_source_account, ToEthAddress},
	ChainName,
};
use dp_asset::TokenMetadata;
use dp_contract::mapping_token_factory::{
	basic::BasicMappingTokenFactory as bmtf, s2s::Sub2SubMappingTokenFactory as smtf,
};

pub type AccountId<T> = <T as frame_system::Config>::AccountId;
pub type RingBalance<T> = <<T as Config>::RingCurrency as Currency<AccountId<T>>>::Balance;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_system::pallet_prelude::*;

	#[pallet::config]
	#[pallet::disable_frame_system_supertrait_check]
	pub trait Config: frame_system::Config + darwinia_evm::Config {
		/// The pallet id of this pallet
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;

		/// The *RING* currency.
		type RingCurrency: Currency<AccountId<Self>>;

		/// The bridge account id converter.
		/// `remote account` + `remote chain id` derive the new account
		type BridgedAccountIdConverter: Convert<H256, Self::AccountId>;

		/// The bridged chain id
		type BridgedChainId: Get<ChainId>;

		/// Convert the substrate account to ethereum account
		type ToEthAddressT: ToEthAddress<Self::AccountId>;

		/// The handler for internal transaction.
		type InternalTransactHandler: InternalTransactHandler;

		/// The remote chain name where the backing module in
		type BackingChainName: Get<ChainName>;

		/// The lane id of the s2s bridge
		type MessageLaneId: Get<LaneId>;
	}

	/// Remote Backing Address, this used to verify the remote caller
	#[pallet::storage]
	#[pallet::getter(fn remote_backing_account)]
	pub type RemoteBackingAccount<T: Config> = StorageValue<_, AccountId<T>, ValueQuery>;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Handle remote register relay message
		/// Before the token transfer, token should be created first
		#[pallet::weight(
			<T as Config>::WeightInfo::register_from_remote()
		)]
		#[transactional]
		pub fn register_from_remote(
			origin: OriginFor<T>,
			token_metadata: TokenMetadata,
		) -> DispatchResultWithPostInfo {
			let user = ensure_signed(origin)?;
			ensure_source_account::<T::AccountId, T::BridgedAccountIdConverter>(
				T::BridgedChainId::get(),
				<RemoteBackingAccount<T>>::get(),
				&user,
			)?;

			let backing_address = T::ToEthAddressT::into_ethereum_id(&user);
			let mut mapping_token =
				Self::mapped_token_address(backing_address, token_metadata.address)?;
			ensure!(mapping_token == H160::zero(), "asset has been registered");

			let name = mapping_token_name(token_metadata.name, T::BackingChainName::get());
			let symbol = mapping_token_symbol(token_metadata.symbol);
			let input = bmtf::encode_create_erc20(
				token_metadata.token_type,
				&str::from_utf8(name.as_slice()).map_err(|_| Error::<T>::StringCF)?,
				&str::from_utf8(symbol.as_slice()).map_err(|_| Error::<T>::StringCF)?,
				token_metadata.decimal,
				backing_address,
				token_metadata.address,
			)
			.map_err(|_| Error::<T>::InvalidEncodeERC20)?;

			Self::transact_mapping_factory(input)?;
			mapping_token = Self::mapped_token_address(backing_address, token_metadata.address)?;
			Self::deposit_event(Event::TokenRegistered(
				user,
				backing_address,
				token_metadata.address,
				mapping_token,
			));
			Ok(().into())
		}

		/// Handle relay message sent from the source backing pallet with relay message
		#[pallet::weight(
			<T as Config>::WeightInfo::issue_from_remote()
		)]
		#[transactional]
		pub fn issue_from_remote(
			origin: OriginFor<T>,
			token_address: H160,
			amount: U256,
			recipient: Vec<u8>,
		) -> DispatchResultWithPostInfo {
			let user = ensure_signed(origin)?;
			// the s2s message relay has been verified that the message comes from the backing chain
			// with the chainID and backing sender address.
			// here only we need is to check the sender is root
			ensure_source_account::<T::AccountId, T::BridgedAccountIdConverter>(
				T::BridgedChainId::get(),
				<RemoteBackingAccount<T>>::get(),
				&user,
			)?;

			ensure!(recipient.len() == 20, Error::<T>::InvalidRecipient);
			let recipient = H160::from_slice(&recipient.as_slice()[..]);

			let backing_address = T::ToEthAddressT::into_ethereum_id(&user);
			let mapping_token = Self::mapped_token_address(backing_address, token_address)?;
			ensure!(mapping_token != H160::zero(), Error::<T>::TokenUnregistered);

			// issue erc20 tokens
			let input = bmtf::encode_issue_erc20(mapping_token, recipient, amount)
				.map_err(|_| Error::<T>::InvalidIssueEncoding)?;
			Self::transact_mapping_factory(input)?;
			Self::deposit_event(Event::TokenIssued(
				backing_address,
				mapping_token,
				recipient,
				amount,
			));
			Ok(().into())
		}

		/// Set mapping token factory address, root account required
		#[pallet::weight(
			<T as Config>::WeightInfo::set_mapping_factory_address()
		)]
		pub fn set_mapping_factory_address(
			origin: OriginFor<T>,
			address: H160,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			let old_address = <MappingFactoryAddress<T>>::get();
			<MappingFactoryAddress<T>>::put(address);
			Self::deposit_event(Event::MappingFactoryAddressUpdated(old_address, address));
			Ok(().into())
		}

        #[pallet::weight(<T as Config>::WeightInfo::set_remote_backing_account())]
        pub fn set_remote_backing_account(
            origin: OriginFor<T>,
            account: AccountId<T>,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            <RemoteBackingAccount<T>>::put(account.clone());
            Self::deposit_event(Event::RemoteBackingAccountUpdated(account));
            Ok(().into())
        }
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Create new token
		/// [user, backing_address, original_token, mapping_token]
		TokenRegistered(AccountId<T>, H160, H160, H160),
		/// Redeem Token
		/// [backing_address, mapping_token, recipient, amount]
		TokenIssued(H160, H160, H160, U256),
		/// Set mapping token factory address
		/// [old, new]
		MappingFactoryAddressUpdated(H160, H160),
		/// Update remote backing address \[account\]
		RemoteBackingAccountUpdated(AccountId<T>),
	}

	#[pallet::error]
	/// Issuing pallet errors.
	pub enum Error<T> {
		/// Token unregistered when issuing
		TokenUnregistered,
		/// Invalid Issuing System Account
		InvalidIssuingAccount,
		/// StringCF
		StringCF,
		/// encode erc20 tx failed
		InvalidEncodeERC20,
		/// encode issue tx failed
		InvalidIssueEncoding,
		/// invalid ethereum address length
		InvalidAddressLen,
		/// Invalid recipient
		InvalidRecipient,
	}

	#[pallet::storage]
	#[pallet::getter(fn mapping_factory_address)]
	pub type MappingFactoryAddress<T: Config> = StorageValue<_, H160, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig {
		pub mapping_factory_address: H160,
	}

	#[cfg(feature = "std")]
	impl Default for GenesisConfig {
		fn default() -> Self {
			Self { mapping_factory_address: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {
			<MappingFactoryAddress<T>>::put(&self.mapping_factory_address);
		}
	}

	impl<T: Config> OnDeliveryConfirmed for Pallet<T> {
		fn on_messages_delivered(lane: &LaneId, messages: &DeliveredMessages) -> Weight {
			if *lane != T::MessageLaneId::get() {
				return 0;
			}
			for nonce in messages.begin..=messages.end {
				let result = messages.message_dispatch_result(nonce);
				if let Ok(input) = smtf::encode_confirm_burn_and_remote_unlock(lane, nonce, result)
				{
					if let Err(e) = Self::transact_mapping_factory(input) {
						log::error!("confirm sub<>sub message failed, err {:?}", e);
					}
				}
			}
			// TODO: The returned weight should be more accurately. See: https://github.com/darwinia-network/darwinia-common/issues/911
			<T as frame_system::Config>::DbWeight::get().reads_writes(1, 1)
		}
	}
}
pub use pallet::*;

impl<T: Config> Pallet<T> {
	/// Get mapping token address from contract
	///
	/// Note: The result is padded as 32 bytes, but the address in contract is 20 bytes, we need to
	/// truncate the prefix(12 bytes) off
	pub fn mapped_token_address(
		backing_address: H160,
		original_token: H160,
	) -> Result<H160, DispatchError> {
		let factory_address = <MappingFactoryAddress<T>>::get();
		let bytes = bmtf::encode_mapping_token(backing_address, original_token)
			.map_err(|_| Error::<T>::InvalidIssuingAccount)?;
		let mapping_token = T::InternalTransactHandler::read_only_call(factory_address, bytes)?;
		if mapping_token.len() != 32 {
			return Err(Error::<T>::InvalidAddressLen.into());
		}
		Ok(H160::from_slice(&mapping_token.as_slice()[12..]))
	}

	/// Make a transaction call to mapping token factory sol contract
	///
	/// Note: this a internal transaction
	pub fn transact_mapping_factory(input: Vec<u8>) -> DispatchResultWithPostInfo {
		let contract = MappingFactoryAddress::<T>::get();
		T::InternalTransactHandler::internal_transact(contract, input)
	}
}
