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

#![allow(unused)]
#![cfg_attr(not(feature = "std"), no_std)]

pub mod weights;
pub use weights::WeightInfo;

use darwinia_evm::AddressMapping;
use darwinia_relay_primitives::{Relay, RelayAccount};
use darwinia_s2s_chain::ChainSelector as TargetChain;

use sp_runtime::traits::Dispatchable;

mod types {
	use crate::*;

	pub type BlockNumber<T> = <T as frame_system::Config>::BlockNumber;
	pub type AccountId<T> = <T as frame_system::Config>::AccountId;
}

// --- crates ---
use ethereum_types::{Address, H160, H256, U256};
// --- substrate ---
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage, ensure, parameter_types,
	traits::{Currency, ExistenceRequirement::*, Get},
	weights::Weight,
	PalletId,
};
use frame_system::ensure_signed;
use sp_runtime::{DispatchError, DispatchResult};
use sp_std::vec::Vec;
// --- darwinia ---
use darwinia_asset_primitives::token::{Token, TokenInfo};
use darwinia_evm::GasWeightMapping;
use darwinia_primitives_contract::mapping_token_factory::MappingTokenFactory as mtf;
use darwinia_primitives_contract::mapping_token_factory::TokenBurnInfo;
use ethereum_primitives::EthereumAddress;
use sha3::Digest;
use types::*;

const REGISTERD_ACTION: &[u8] = b"registered(address,address,address)";
const BURN_ACTION: &[u8] = b"burned(address,address,address,address,uint256)";

pub trait Config: dvm_ethereum::Config {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

	type PalletId: Get<PalletId>;

	type ReceiverAccountId: From<[u8; 32]> + Into<Self::AccountId>;

	type WeightInfo: WeightInfo;
	type BackingRelay: Relay<
		RelayProof = AccountId<Self>,
		VerifiedResult = Result<(EthereumAddress, TargetChain), DispatchError>,
		RelayMessage = (TargetChain, Token, RelayAccount<Self::AccountId>),
		RelayMessageResult = Result<(), DispatchError>,
	>;
}

decl_error! {
	/// Issuing pallet errors.
	pub enum Error for Module<T: Config> {
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
}

decl_event! {
	pub enum Event<T>
	where
		AccountId = AccountId<T>,
	{
		/// new erc20 token created [user, backing, tokenaddress, mappedaddress]
		NewTokenCreated(AccountId, EthereumAddress, EthereumAddress, EthereumAddress),
		/// token redeemed [backing, mappedaddress, recipient, value]
		TokenRedeemed(EthereumAddress, EthereumAddress, EthereumAddress, U256),
	}
}

decl_storage! {
	trait Store for Module<T: Config> as Substrate2SubstrateIssuing {
		pub MappingFactoryAddress get(fn mapping_factory_address) config(): EthereumAddress;
	}
}

decl_module! {
	pub struct Module<T: Config> for enum Call
	where
		origin: T::Origin
	{
		fn deposit_event() = default;

		fn on_initialize(_n: BlockNumber<T>) -> Weight {
			0
		}

        /// handle from contract call
        /// when user burn their tokens, this handler will receive the event from dispatch
        /// precompile contract, and relay this event to the target chain to unlock asset
        #[weight = 0]
        pub fn dispatch_handle(origin, input: Vec<u8>) {
            let user = ensure_signed(origin)?;
            // we must check this user comes from mapping token factory contract address with
            // precompile dispatch contract
            let factory_address = MappingFactoryAddress::get();
            let caller = <T as darwinia_evm::Config>::AddressMapping::into_account_id(factory_address);
            ensure!(caller == user, <Error<T>>::AssetAR);
            let register_action = &sha3::Keccak256::digest(&REGISTERD_ACTION)[0..4];
            let burn_action = &sha3::Keccak256::digest(&BURN_ACTION)[0..4];
            if &input[4..8] == register_action {
                //register response
                log::info!("new s2s token has been registered, ingore response");
            } else if &input[4..8] == burn_action {
                //burn action
                let burn_info = TokenBurnInfo::decode(&input[8..])
                    .map_err(|_| Error::<T>::InvalidDecoding)?;
                let recipient = Self::account_id_try_from_bytes(burn_info.recipient.as_slice())?;
                let mut target: [u8;4] = Default::default();
                target.copy_from_slice(&input[..4]);
                Self::cross_send(target, burn_info.source, recipient, burn_info.amount)?;
            }
        }

		#[weight = 0]
		pub fn cross_receive(origin, message: (Token, EthereumAddress)) {
			let user = ensure_signed(origin)?;
			// the s2s message relay has been verified this comes from the backing chain with the
			// chainID and backing sender address.
			// here only we need is to check the sender is in whitelist
			let (backing, target) = T::BackingRelay::verify(&user)?;
			let (token, recipient) = message;

			let token_info = match token {
				Token::Native(info) => {
					log::debug!("cross receive native token {:?}", info);
					info
				}
				Token::Erc20(info) => {
					log::debug!("cross receive erc20 token {:?}", info);
					info
				}
				_ => {
					return Err(Error::<T>::InvalidTokenType.into())
				}
			};

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
						let input = Self::abi_encode_token_creation(target, backing, token_info.address, &name, &symbol, option.decimal)?;
						Self::transact_mapping_factory(input)?;
						// TODO check if we can get this address after create immediately
						mapped_address = Self::mapped_token_address(backing, token_info.address)?;
						Self::deposit_event(RawEvent::NewTokenCreated(user, backing, token_info.address, mapped_address));
					}
					_ => return Err(Error::<T>::InvalidTokenOption.into())
				}
			}
			// redeem
			if let Some(value) = token_info.value {
				let input = Self::abi_encode_token_redeem(mapped_address, recipient, value)?;
				Self::transact_mapping_factory(input)?;
				Self::deposit_event(RawEvent::TokenRedeemed(backing, mapped_address, recipient, value));
			}
		}
    }
}

impl<T: Config> Module<T> {
	fn abi_encode_token_creation(
		target: TargetChain,
		backing: EthereumAddress,
		address: EthereumAddress,
		name: &str,
		symbol: &str,
		decimal: u8,
	) -> Result<Vec<u8>, DispatchError> {
		let input = mtf::encode_create_erc20(target, name, symbol, decimal, backing, address)
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
		let factory_address = MappingFactoryAddress::get();
		let bytes = mtf::encode_mapping_token(backing, source)
			.map_err(|_| Error::<T>::InvalidIssuingAccount)?;
		let mapped_address = dvm_ethereum::Module::<T>::do_call(factory_address, bytes)
			.map_err(|e| -> &'static str { e.into() })?;
		if mapped_address.len() != 32 {
			return Err(Error::<T>::InvalidAddressLen.into());
		}
		Ok(H160::from_slice(&mapped_address.as_slice()[12..]))
	}

	pub fn transact_mapping_factory(input: Vec<u8>) -> DispatchResult {
		let contract = MappingFactoryAddress::get();
		let result = dvm_ethereum::Module::<T>::internal_transact(contract, input).map_err(
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

    pub fn cross_send(target: TargetChain, token: EthereumAddress, recipient: AccountId<T>, amount: U256) -> Result<(), DispatchError> {
        let message = (
            target,
            Token::Native(TokenInfo {
                address: token,
                value: Some(amount),
                option: None,
            }),
            RelayAccount::DarwiniaAccount(recipient)
        );
        T::BackingRelay::relay_message(&message)
    }
}
