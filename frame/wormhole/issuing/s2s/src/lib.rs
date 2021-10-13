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

//! Prototype module for s2s cross chain assets issuing.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod tests;

pub mod weight;
pub use weight::WeightInfo;

// --- crates.io ---
use ethereum_types::{H160, H256, U256};
// --- paritytech ---
use frame_support::{
	ensure,
	pallet_prelude::*,
	traits::{Currency, Get},
	transactional, PalletId,
};
use frame_system::ensure_signed;
use sp_runtime::{traits::Convert, DispatchError};
use sp_std::{str, vec::Vec};
// --- darwinia-network ---
use bp_runtime::{ChainId, Size};
use darwinia_evm::AddressMapping;
use darwinia_support::{
	mapping_token::*,
	s2s::{ensure_source_root, MessageConfirmer, RelayMessageCaller, ToEthAddress, TokenMessageId},
	ChainName,
};
use dp_asset::token::Token;
use dp_contract::mapping_token_factory::{
	basic::BasicMappingTokenFactory as bmtf,
	s2s::{S2sRemoteUnlockInfo, Sub2SubMappingTokenFactory as smtf},
};
use dvm_ethereum::InternalTransactHandler;

pub use pallet::*;
pub type AccountId<T> = <T as frame_system::Config>::AccountId;
pub type RingBalance<T> = <<T as Config>::RingCurrency as Currency<AccountId<T>>>::Balance;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_system::pallet_prelude::*;

	#[pallet::config]
	#[pallet::disable_frame_system_supertrait_check]
	pub trait Config: frame_system::Config + darwinia_evm::Config {
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type WeightInfo: WeightInfo;
		type RingCurrency: Currency<AccountId<Self>>;

		type BridgedAccountIdConverter: Convert<H256, Self::AccountId>;
		type BridgedChainId: Get<ChainId>;
		type ToEthAddressT: ToEthAddress<Self::AccountId>;
		type OutboundPayload: Parameter + Size;
		type CallEncoder: EncodeCall<Self::AccountId, Self::OutboundPayload>;
		type MessageSender: RelayMessageCaller<Self::OutboundPayload, RingBalance<Self>>;
		type InternalTransactHandler: InternalTransactHandler;
		type BackingChainName: Get<ChainName>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// send s2s message to remote backing module
		/// this only can be called by mapping-token-factory
		#[pallet::weight(
			<T as Config>::WeightInfo::send_message()
		)]
		#[transactional]
		pub fn send_message(
			origin: OriginFor<T>,
			payload: T::OutboundPayload,
			fee: RingBalance<T>,
		) -> DispatchResultWithPostInfo {
			let caller = ensure_signed(origin)?;
			// Ensure that the user is mapping token factory contract
			let factory = MappingFactoryAddress::<T>::get();
			let factory_id = <T as darwinia_evm::Config>::AddressMapping::into_account_id(factory);
			ensure!(caller == factory_id, <Error<T>>::NotFactoryContract);
			T::MessageSender::send_message(payload, fee)
				.map_err(|_| Error::<T>::SendMessageFailed)?;
			Ok(().into())
		}

		/// Handle remote register relay message
		/// Before the token transfer, token should be created first
		#[pallet::weight(
			<T as Config>::WeightInfo::register_from_remote()
			.saturating_add(2_000_000 * 3)
		)]
		#[transactional]
		pub fn register_from_remote(
			origin: OriginFor<T>,
			token: Token,
		) -> DispatchResultWithPostInfo {
			let user = ensure_signed(origin)?;
			ensure_source_root::<T::AccountId, T::BridgedAccountIdConverter>(
				T::BridgedChainId::get(),
				&user,
			)?;

			let backing_address = T::ToEthAddressT::into_ethereum_id(&user);
			let (token_type, token_info) = token
				.token_info()
				.map_err(|_| Error::<T>::InvalidTokenType)?;
			let mut mapping_token =
				Self::mapped_token_address(backing_address, token_info.address)?;
			ensure!(mapping_token == H160::zero(), "asset has been registered");

			match token_info.option {
				Some(option) => {
					let name = mapping_token_name(option.name, T::BackingChainName::get());
					let symbol = mapping_token_symbol(option.symbol);
					let input = bmtf::encode_create_erc20(
						token_type,
						&str::from_utf8(name.as_slice()).map_err(|_| Error::<T>::StringCF)?,
						&str::from_utf8(symbol.as_slice()).map_err(|_| Error::<T>::StringCF)?,
						option.decimal,
						backing_address,
						token_info.address,
					)
					.map_err(|_| Error::<T>::InvalidEncodeERC20)?;

					Self::transact_mapping_factory(input)?;
					mapping_token =
						Self::mapped_token_address(backing_address, token_info.address)?;
					Self::deposit_event(Event::TokenRegistered(
						user,
						backing_address,
						token_info.address,
						mapping_token,
					));
				}
				_ => return Err(Error::<T>::InvalidTokenOption.into()),
			}
			Ok(().into())
		}

		/// Handle relay message sent from the source backing pallet with relay message
		#[pallet::weight(
			<T as Config>::WeightInfo::issue_from_remote()
			.saturating_add(2_000_000 * 2)
		)]
		#[transactional]
		pub fn issue_from_remote(
			origin: OriginFor<T>,
			token: Token,
			recipient: H160,
		) -> DispatchResultWithPostInfo {
			let user = ensure_signed(origin)?;
			// the s2s message relay has been verified that the message comes from the backing chain with the
			// chainID and backing sender address.
			// here only we need is to check the sender is root
			ensure_source_root::<T::AccountId, T::BridgedAccountIdConverter>(
				T::BridgedChainId::get(),
				&user,
			)?;

			let backing_address = T::ToEthAddressT::into_ethereum_id(&user);
			let (_, token_info) = token
				.token_info()
				.map_err(|_| Error::<T>::InvalidTokenType)?;

			let mapping_token = Self::mapped_token_address(backing_address, token_info.address)?;
			ensure!(
				mapping_token != H160::zero(),
				"asset has not been registered"
			);

			// Redeem process
			if let Some(value) = token_info.value {
				let input = bmtf::encode_issue_erc20(mapping_token, recipient, value)
					.map_err(|_| Error::<T>::InvalidMintEncoding)?;
				Self::transact_mapping_factory(input)?;
				Self::deposit_event(Event::TokenIssued(
					backing_address,
					mapping_token,
					recipient,
					value,
				));
			}
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
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	#[pallet::metadata(AccountId<T> = "AccountId")]
	pub enum Event<T: Config> {
		/// Create new token
		/// [user, backing_address, original_token, mapping_token]
		TokenRegistered(AccountId<T>, H160, H160, H160),
		/// Redeem Token
		/// [backing_address, mapping_token, recipient, amount]
		TokenIssued(H160, H160, H160, U256),
		/// Token Burned and request Remote unlock
		/// [spec_version, weight, tokenType, original_token, amount, recipient, fee]
		TokenBurned(u32, u64, u32, H160, U256, AccountId<T>, U256),
		/// Set mapping token factory address
		/// [old, new]
		MappingFactoryAddressUpdated(H160, H160),
	}

	#[pallet::error]
	/// Issuing pallet errors.
	pub enum Error<T> {
		/// The address is not from mapping factory contract address
		NotFactoryContract,
		/// Invalid Issuing System Account
		InvalidIssuingAccount,
		/// StringCF
		StringCF,
		/// encode erc20 tx failed
		InvalidEncodeERC20,
		/// encode mint tx failed
		InvalidMintEncoding,
		/// invalid ethereum address length
		InvalidAddressLen,
		/// invalid token type
		InvalidTokenType,
		/// invalid token option
		InvalidTokenOption,
		/// decode event failed
		InvalidDecoding,
		/// invalid source origin
		InvalidOrigin,
		/// encode dispatch call failed
		EncodeInvalid,
		/// send relay message failed
		SendMessageFailed,
		/// call mapping factory failed
		MappingFactoryCallFailed,
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
			Self {
				mapping_factory_address: Default::default(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {
			<MappingFactoryAddress<T>>::put(&self.mapping_factory_address);
		}
	}

	impl<T: Config> MessageConfirmer for Pallet<T> {
		fn on_messages_confirmed(message_id: TokenMessageId, result: bool) -> Weight {
			if let Ok(input) =
				smtf::encode_confirm_burn_and_remote_unlock(message_id.to_vec(), result)
			{
				let _ = Self::transact_mapping_factory(input);
			}
			return 1;
		}
	}
}

impl<T: Config> Pallet<T> {
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

pub trait EncodeCall<AccountId, Payload> {
	fn encode_remote_unlock(remote_unlock_info: S2sRemoteUnlockInfo) -> Result<Payload, ()>;
}
