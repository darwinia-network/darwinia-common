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

//! Prototype module for cross chain assets issuing.

#![allow(unused)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[cfg(feature = "runtime-benchmarks")]
mod mock_header;

pub mod weight;
pub use weight::WeightInfo;

// --- paritytech ---
use frame_support::{
	ensure, log,
	pallet_prelude::*,
	parameter_types,
	traits::{Currency, ExistenceRequirement::*, Get, LockableCurrency},
	transactional, PalletId,
};
use frame_system::{ensure_root, ensure_signed, pallet_prelude::*};
use sp_runtime::{
	traits::{AccountIdConversion, Keccak256, Saturating},
	AccountId32, DispatchError, DispatchResult, SaturatedConversion,
};
use sp_std::{str, vec::Vec};
// --- darwinia-network ---
use darwinia_ethereum::InternalTransactHandler;
use darwinia_evm::GasWeightMapping;
use darwinia_relay_primitives::relay_authorities::*;
use darwinia_support::{
	balance::*, evm::IntoAccountId, mapping_token::*, traits::EthereumReceipt, ChainName,
};
use dp_contract::{
	ethereum_backing::{EthereumBacking, EthereumLockEvent, EthereumRegisterEvent},
	mapping_token_factory::{
		basic::BasicMappingTokenFactory as bmtf,
		ethereum2darwinia::{E2dRemoteUnlockInfo, TokenRegisterInfo},
	},
};
use ethereum_primitives::{
	log_entry::LogEntry, receipt::EthereumTransactionIndex, EthereumAddress, U256,
};

const REGISTER_TYPE: u8 = 0;
const BURN_TYPE: u8 = 1;

pub type AccountId<T> = <T as frame_system::Config>::AccountId;
pub type RingBalance<T> = <<T as Config>::RingCurrency as Currency<AccountId<T>>>::Balance;
pub type EthereumReceiptProofThing<T> = <<T as Config>::EthereumRelay as EthereumReceipt<
	AccountId<T>,
	RingBalance<T>,
>>::EthereumReceiptProofThing;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use crate::*;

	#[pallet::config]
	#[pallet::disable_frame_system_supertrait_check]
	pub trait Config: frame_system::Config + darwinia_evm::Config {
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type RingCurrency: LockableCurrency<Self::AccountId>;
		type EthereumRelay: EthereumReceipt<Self::AccountId, RingBalance<Self>>;
		type EcdsaAuthorities: RelayAuthorityProtocol<Self::BlockNumber, Signer = EthereumAddress>;
		type WeightInfo: WeightInfo;
		type InternalTransactHandler: InternalTransactHandler;
		type BackingChainName: Get<ChainName>;
	}

	#[pallet::error]
	/// Issuing pallet errors.
	pub enum Error<T> {
		/// Invalid Issuing System Account
		InvalidIssuingAccount,
		/// assert already registered
		AssetAlreadyRegistered,
		/// assert already redeemed
		AssetAlreadyRedeemed,
		/// StringCF
		StringCF,
		/// encode erc20 tx failed
		InvalidEncodeERC20,
		/// invalid ethereum address length
		InvalidAddressLen,
		/// decode input value error
		InvalidInputData,
		/// decode ethereum event failed
		DecodeEventFailed,
		/// caller has no authority
		NoAuthority,
		/// the action is not supported
		UnsupportedAction,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// register new erc20 token
		TokenRegisterSubmitted(EthereumAddress, EthereumTransactionIndex),
		/// redeem erc20 token
		RedeemErc20(EthereumAddress, EthereumTransactionIndex),
		//  These two events `BurnToken` and `TokenRegisterFinished` will be saved in a special
		// storage, and  will be delivered to the remote chain. Remote ethereum chain will decode
		// them using  scale encoding. And the first parameter `type` is used to distinguish the
		// two events.
		/// burn event
		/// type: 1, backing_address, sender, recipient, original_token, mapping_token, value
		BurnToken(
			u8,
			EthereumAddress,
			EthereumAddress,
			EthereumAddress,
			EthereumAddress,
			EthereumAddress,
			U256,
		),
		/// token registered event
		/// type: u8 = 0, backing_address, original_token(origin erc20), mapping_token(mapped
		/// erc20)
		TokenRegisterFinished(u8, EthereumAddress, EthereumAddress, EthereumAddress),
		/// set mapping token factory address
		/// [old, new]
		MappingFactoryAddressUpdated(EthereumAddress, EthereumAddress),
		/// set ethereum backing address
		/// [old, new]
		EthereumBackingAddressUpdated(EthereumAddress, EthereumAddress),
	}

	#[pallet::storage]
	#[pallet::getter(fn mapping_factory_address)]
	pub type MappingFactoryAddress<T: Config> = StorageValue<_, EthereumAddress, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn ethereum_backing_address)]
	pub type EthereumBackingAddress<T: Config> = StorageValue<_, EthereumAddress, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn verified_issuing_proof)]
	pub type VerifiedIssuingProof<T> = StorageMap<
		_,
		Blake2_128Concat,
		EthereumTransactionIndex,
		bool,
		ValueQuery,
		DefaultVerifiedIssuingProof,
	>;

	#[pallet::type_value]
	pub fn DefaultVerifiedIssuingProof() -> bool {
		false
	}

	#[pallet::storage]
	#[pallet::getter(fn burn_token_events)]
	pub type BurnTokenEvents<T: Config> =
		StorageValue<_, Vec<<T as frame_system::Config>::Event>, ValueQuery>;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			<BurnTokenEvents<T>>::kill();
			T::DbWeight::get().writes(1)
		}
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig {
		pub mapping_factory_address: EthereumAddress,
		pub ethereum_backing_address: EthereumAddress,
	}

	#[cfg(feature = "std")]
	impl Default for GenesisConfig {
		fn default() -> Self {
			Self {
				mapping_factory_address: Default::default(),
				ethereum_backing_address: Default::default(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {
			<MappingFactoryAddress<T>>::put(&self.mapping_factory_address);
			<EthereumBackingAddress<T>>::put(&self.ethereum_backing_address);
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(<T as Config>::WeightInfo::register_erc20())]
		#[transactional]
		pub fn register_erc20(
			origin: OriginFor<T>,
			proof: EthereumReceiptProofThing<T>,
		) -> DispatchResultWithPostInfo {
			log::debug!("start to register erc20 token");
			ensure_signed(origin)?;
			let tx_index = T::EthereumRelay::gen_receipt_index(&proof);
			ensure!(
				!VerifiedIssuingProof::<T>::contains_key(tx_index),
				<Error<T>>::AssetAlreadyRegistered
			);

			let verified_receipt = T::EthereumRelay::verify_receipt(&proof)?;
			let backing_address = EthereumBackingAddress::<T>::get();
			let register_info =
				EthereumBacking::parse_register_event(&verified_receipt, &backing_address)
					.map_err(|_| Error::<T>::DecodeEventFailed)?;
			let name = mapping_token_name(register_info.name, T::BackingChainName::get());
			let symbol = mapping_token_symbol(register_info.symbol);
			let input = bmtf::encode_create_erc20(
				0,
				str::from_utf8(&name.as_slice()).map_err(|_| Error::<T>::StringCF)?,
				str::from_utf8(&symbol.as_slice()).map_err(|_| Error::<T>::StringCF)?,
				register_info.decimals.as_u32() as u8,
				backing_address,
				register_info.token_address,
			)
			.map_err(|_| Error::<T>::InvalidEncodeERC20)?;
			Self::transact_mapping_factory(input)?;
			VerifiedIssuingProof::<T>::insert(tx_index, true);
			Self::deposit_event(Event::TokenRegisterSubmitted(backing_address, tx_index));
			Ok(().into())
		}

		#[pallet::weight(<T as Config>::WeightInfo::redeem_erc20())]
		#[transactional]
		pub fn redeem_erc20(
			origin: OriginFor<T>,
			proof: EthereumReceiptProofThing<T>,
		) -> DispatchResultWithPostInfo {
			log::debug!("start to redeem erc20 token");
			ensure_signed(origin)?;
			let tx_index = T::EthereumRelay::gen_receipt_index(&proof);
			ensure!(
				!VerifiedIssuingProof::<T>::contains_key(tx_index),
				<Error<T>>::AssetAlreadyRedeemed
			);
			let verified_receipt = T::EthereumRelay::verify_receipt(&proof)?;
			let backing_address = EthereumBackingAddress::<T>::get();
			let lock_info =
				EthereumBacking::parse_locking_event(&verified_receipt, &backing_address)
					.map_err(|_| Error::<T>::DecodeEventFailed)?;
			let input = bmtf::encode_issue_erc20(
				lock_info.mapping_token,
				lock_info.recipient,
				lock_info.amount,
			)
			.map_err(|_| Error::<T>::InvalidEncodeERC20)?;
			Self::transact_mapping_factory(input)?;
			VerifiedIssuingProof::<T>::insert(tx_index, true);
			Self::deposit_event(Event::RedeemErc20(backing_address, tx_index));
			Ok(().into())
		}

		// when the token register complete, the contract will call this method to deliver the
		// response information to this issuing pallet
		#[pallet::weight(<T as Config>::WeightInfo::register_response_from_contract())]
		#[transactional]
		pub fn register_response_from_contract(
			origin: OriginFor<T>,
			input: Vec<u8>,
		) -> DispatchResultWithPostInfo {
			let caller = ensure_signed(origin)?;
			let factory = MappingFactoryAddress::<T>::get();
			let factory_id = <T as darwinia_evm::Config>::IntoAccountId::into_account_id(factory);
			ensure!(factory_id == caller, <Error<T>>::NoAuthority);
			let register_info =
				TokenRegisterInfo::decode(&input).map_err(|_| Error::<T>::InvalidInputData)?;
			Self::finish_token_registered(register_info.0, register_info.1, register_info.2);
			Ok(().into())
		}

		// When user burn their mapped tokens to unlock remote origin token, mapping token factory
		// will use precompile to call this, this call will deposit the burn token event and
		// trigger schedule_mmr_root to request authorities to sign mmr root, and then relay this
		// event to ethereum chain.
		#[pallet::weight(<T as Config>::WeightInfo::deposit_burn_token_event_from_precompile())]
		#[transactional]
		pub fn deposit_burn_token_event_from_precompile(
			origin: OriginFor<T>,
			input: Vec<u8>,
		) -> DispatchResultWithPostInfo {
			let caller = ensure_signed(origin)?;
			let factory = MappingFactoryAddress::<T>::get();
			let factory_id = <T as darwinia_evm::Config>::IntoAccountId::into_account_id(factory);
			ensure!(factory_id == caller, <Error<T>>::NoAuthority);
			let burn_info =
				E2dRemoteUnlockInfo::decode(&input).map_err(|_| Error::<T>::InvalidInputData)?;
			Self::deposit_burn_token_event(
				burn_info.backing_address,
				burn_info.sender,
				burn_info.original_token,
				burn_info.recipient,
				burn_info.amount,
			)?;

			Ok(().into())
		}

		#[pallet::weight(<T as Config>::WeightInfo::set_mapping_factory_address())]
		pub fn set_mapping_factory_address(
			origin: OriginFor<T>,
			address: EthereumAddress,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			let old_address = MappingFactoryAddress::<T>::get();
			MappingFactoryAddress::<T>::put(address);
			Self::deposit_event(Event::MappingFactoryAddressUpdated(old_address, address));
			Ok(().into())
		}

		#[pallet::weight(<T as Config>::WeightInfo::set_ethereum_backing_address())]
		pub fn set_ethereum_backing_address(
			origin: OriginFor<T>,
			address: EthereumAddress,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			let old_address = EthereumBackingAddress::<T>::get();
			EthereumBackingAddress::<T>::put(address);
			Self::deposit_event(Event::EthereumBackingAddressUpdated(old_address, address));
			Ok(().into())
		}
	}
}
pub use pallet::*;

impl<T: Config> Pallet<T> {
	pub fn mapped_token_address(
		backing_address: EthereumAddress,
		original_token: EthereumAddress,
	) -> Result<EthereumAddress, DispatchError> {
		let factory_address = MappingFactoryAddress::<T>::get();
		let bytes = bmtf::encode_mapping_token(backing_address, original_token)
			.map_err(|_| Error::<T>::InvalidIssuingAccount)?;
		let mapping_token = T::InternalTransactHandler::read_only_call(factory_address, bytes)?;
		if mapping_token.len() != 32 {
			return Err(Error::<T>::InvalidAddressLen.into());
		}
		Ok(EthereumAddress::from_slice(&mapping_token.as_slice()[12..]))
	}

	pub fn finish_token_registered(
		backing_address: EthereumAddress,
		original_token: EthereumAddress,
		mapping_token: EthereumAddress,
	) -> DispatchResult {
		let raw_event = Event::TokenRegisterFinished(
			REGISTER_TYPE,
			backing_address,
			original_token,
			mapping_token,
		);
		let module_event: <T as Config>::Event = raw_event.clone().into();
		let system_event: <T as frame_system::Config>::Event = module_event.into();
		<BurnTokenEvents<T>>::append(system_event);
		Self::deposit_event(raw_event);
		T::EcdsaAuthorities::schedule_mmr_root(
			(<frame_system::Pallet<T>>::block_number().saturated_into::<u32>() / 10 * 10 + 10)
				.saturated_into(),
		);
		Ok(().into())
	}

	pub fn deposit_burn_token_event(
		backing_address: EthereumAddress,
		sender: EthereumAddress,
		original_token: EthereumAddress,
		recipient: EthereumAddress,
		amount: U256,
	) -> DispatchResultWithPostInfo {
		let mapping_token =
			Self::mapped_token_address(backing_address, original_token).map_err(|e| {
				log::debug!("mapped token address error {:?} ", e);
				e
			})?;

		let raw_event = Event::BurnToken(
			BURN_TYPE,
			backing_address,
			sender,
			recipient,
			original_token,
			mapping_token,
			amount,
		);
		let module_event: <T as Config>::Event = raw_event.clone().into();
		let system_event: <T as frame_system::Config>::Event = module_event.into();
		<BurnTokenEvents<T>>::append(system_event);

		Self::deposit_event(raw_event);
		T::EcdsaAuthorities::schedule_mmr_root(
			(<frame_system::Pallet<T>>::block_number().saturated_into::<u32>() / 10 * 10 + 10)
				.saturated_into(),
		);
		Ok(().into())
	}

	pub fn transact_mapping_factory(input: Vec<u8>) -> DispatchResult {
		let contract = MappingFactoryAddress::<T>::get();
		let result = T::InternalTransactHandler::internal_transact(contract, input).map_err(
			|e| -> &'static str {
				log::debug!("call mapping factory contract error {:?}", &e);
				e.into()
			},
		)?;
		Ok(())
	}
}
