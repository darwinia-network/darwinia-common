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

pub mod weights;
pub use weights::WeightInfo;

pub type AccountId<T> = <T as frame_system::Config>::AccountId;

// --- crates ---
use ethereum_primitives::EthereumAddress;
use ethereum_types::{H160, U256};
use sha3::Digest;
// --- substrate ---
use frame_support::{ensure, traits::Get, PalletId};
use frame_system::ensure_signed;
use sp_runtime::{DispatchError, DispatchResult};
use sp_std::vec::Vec;
// --- darwinia ---
use darwinia_evm::AddressMapping;
use darwinia_relay_primitives::{Relay, RelayAccount, RelayDigest};
use dp_asset::token::{Token, TokenInfo};
use dp_contract::mapping_token_factory::{MappingTokenFactory as mtf, TokenBurnInfo};

const REGISTERD_ACTION: &[u8] = b"registered(address,address,address)";
const BURN_ACTION: &[u8] = b"burned(address,address,address,address,uint256)";

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::config]
	#[pallet::disable_frame_system_supertrait_check]
	pub trait Config: dvm_ethereum::Config {
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type WeightInfo: WeightInfo;

		type ReceiverAccountId: From<[u8; 32]> + Into<Self::AccountId>;
		type BackingRelay: Relay<
			RelayOrigin = AccountId<Self>,
			RelayMessage = (u32, Token, RelayAccount<Self::AccountId>),
		>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// handle from contract call
		/// when user burn their tokens, this handler will receive the event from dispatch
		/// precompile contract, and relay this event to the target chain to unlock asset
		#[pallet::weight(0)]
		pub fn dispatch_handle(origin: OriginFor<T>, input: Vec<u8>) -> DispatchResultWithPostInfo {
			let user = ensure_signed(origin)?;
			// we must check this user comes from mapping token factory contract address with
			// precompile dispatch contract
			let factory_address = MappingFactoryAddress::<T>::get();
			let caller =
				<T as darwinia_evm::Config>::AddressMapping::into_account_id(factory_address);
			ensure!(caller == user, <Error<T>>::AssetAR);
			let register_action = &sha3::Keccak256::digest(&REGISTERD_ACTION)[0..4];
			let burn_action = &sha3::Keccak256::digest(&BURN_ACTION)[0..4];
			if &input[4..8] == register_action {
				//register response
				log::info!("new s2s token has been registered, ingore response");
			} else if &input[4..8] == burn_action {
				//burn action
				let burn_info =
					TokenBurnInfo::decode(&input[8..]).map_err(|_| Error::<T>::InvalidDecoding)?;
				let recipient = Self::account_id_try_from_bytes(burn_info.recipient.as_slice())?;
				Self::burn_and_remote_unlock(
					burn_info.spec_version,
					burn_info.token_type,
					burn_info.source,
					recipient,
					burn_info.amount,
				)?;
			}
			Ok(().into())
		}

		/// this is a remote call from the source backing pallet with relay message
		/// only source backing pallet address is accepted
		/// receive token transfer from the source chain, if the mapped token is not created, then
		/// create first
		#[pallet::weight(0)]
		pub fn cross_receive_and_redeem(
			origin: OriginFor<T>,
			message: (Token, EthereumAddress),
		) -> DispatchResultWithPostInfo {
			let user = ensure_signed(origin)?;
			// the s2s message relay has been verified that the message comes from the backing chain with the
			// chainID and backing sender address.
			// here only we need is to check the sender is in whitelist
			let backing = T::BackingRelay::verify_origin(&user)?;
			let (token, recipient) = message;

			let (token_type, token_info) = token
				.token_info()
				.map_err(|_| Error::<T>::InvalidTokenType)?;

			let mut mapped_address = Self::mapped_token_address(backing, token_info.address)?;
			// if the mapped token address has not been created, create it first
			if mapped_address == H160::zero() {
				// create
				match token_info.option {
					Some(option) => {
						let name = sp_std::str::from_utf8(&option.name[..])
							.map_err(|_| Error::<T>::StringCF)?;
						let symbol = sp_std::str::from_utf8(&option.symbol[..])
							.map_err(|_| Error::<T>::StringCF)?;
						let input = Self::abi_encode_token_creation(
							backing,
							token_info.address,
							token_type,
							&name,
							&symbol,
							option.decimal,
						)?;
						Self::transact_mapping_factory(input)?;
						mapped_address = Self::mapped_token_address(backing, token_info.address)?;
						Self::deposit_event(Event::NewTokenCreated(
							user,
							backing,
							token_info.address,
							mapped_address,
						));
					}
					_ => return Err(Error::<T>::InvalidTokenOption.into()),
				}
			}
			// redeem
			if let Some(value) = token_info.value {
				let input = Self::abi_encode_token_redeem(mapped_address, recipient, value)?;
				Self::transact_mapping_factory(input)?;
				Self::deposit_event(Event::TokenRedeemed(
					backing,
					mapped_address,
					recipient,
					value,
				));
			}
			Ok(().into())
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	#[pallet::metadata(T::AccountId = "AccountId")]
	pub enum Event<T: Config> {
		/// new erc20 token created [user, backing, tokenaddress, mappedaddress]
		NewTokenCreated(
			AccountId<T>,
			EthereumAddress,
			EthereumAddress,
			EthereumAddress,
		),
		/// token redeemed [backing, mappedaddress, recipient, value]
		TokenRedeemed(EthereumAddress, EthereumAddress, EthereumAddress, U256),
	}

	#[pallet::error]
	/// Issuing pallet errors.
	pub enum Error<T> {
		/// assert ar
		AssetAR,
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
	}

	#[pallet::storage]
	#[pallet::getter(fn mapping_factory_address)]
	pub type MappingFactoryAddress<T: Config> = StorageValue<_, EthereumAddress, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig {
		pub mapping_factory_address: EthereumAddress,
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
			{
				let data = &self.mapping_factory_address;
				let v: &EthereumAddress = data;
				<MappingFactoryAddress<T> as frame_support::storage::StorageValue<
					EthereumAddress,
				>>::put::<&EthereumAddress>(v);
			}
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn relay_digest() -> RelayDigest {
		return T::BackingRelay::digest();
	}

	fn abi_encode_token_creation(
		backing: EthereumAddress,
		address: EthereumAddress,
		token_type: u32,
		name: &str,
		symbol: &str,
		decimal: u8,
	) -> Result<Vec<u8>, DispatchError> {
		let callback_processor = Self::relay_digest();
		let input = mtf::encode_create_erc20(
			callback_processor,
			token_type,
			name,
			symbol,
			decimal,
			backing,
			address,
		)
		.map_err(|_| Error::<T>::InvalidEncodeERC20)?;
		Ok(input)
	}

	fn abi_encode_token_redeem(
		dtoken_address: EthereumAddress,
		recipient: EthereumAddress,
		amount: U256,
	) -> Result<Vec<u8>, DispatchError> {
		let input = mtf::encode_cross_receive(dtoken_address, recipient, amount)
			.map_err(|_| Error::<T>::InvalidMintEncoding)?;

		Ok(input)
	}

	pub fn mapped_token_address(
		backing: EthereumAddress,
		source: EthereumAddress,
	) -> Result<H160, DispatchError> {
		let factory_address = MappingFactoryAddress::<T>::get();
		let bytes = mtf::encode_mapping_token(backing, source)
			.map_err(|_| Error::<T>::InvalidIssuingAccount)?;
		let mapped_address = dvm_ethereum::Pallet::<T>::do_call(factory_address, bytes)
			.map_err(|e| -> &'static str { e.into() })?;
		if mapped_address.len() != 32 {
			return Err(Error::<T>::InvalidAddressLen.into());
		}
		Ok(H160::from_slice(&mapped_address.as_slice()[12..]))
	}

	pub fn transact_mapping_factory(input: Vec<u8>) -> DispatchResult {
		let contract = MappingFactoryAddress::<T>::get();
		let result = dvm_ethereum::Pallet::<T>::internal_transact(contract, input).map_err(
			|e| -> &'static str {
				log::info!("call mapping factory contract error {:?}", &e);
				e.into()
			},
		)?;
		Ok(())
	}

	pub fn account_id_try_from_bytes(bytes: &[u8]) -> Result<T::AccountId, DispatchError> {
		if bytes.len() != 32 {
			return Err(Error::<T>::InvalidAddressLen.into());
		}

		let account_id: T::ReceiverAccountId = array_bytes::dyn2array!(bytes, 32).into();

		Ok(account_id.into())
	}

	pub fn burn_and_remote_unlock(
		spec_version: u32,
		token_type: u32,
		token: EthereumAddress,
		recipient: AccountId<T>,
		amount: U256,
	) -> Result<(), DispatchError> {
		let message = (
			spec_version,
			(
				token_type,
				TokenInfo {
					address: token,
					value: Some(amount),
					option: None,
				},
			)
				.into(),
			RelayAccount::DarwiniaAccount(recipient),
		);
		T::BackingRelay::relay_message(&message)
	}
}
