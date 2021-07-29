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

//! Prototype module for cross chain assets issuing.

#![allow(unused)]
#![cfg_attr(not(feature = "std"), no_std)]

pub mod weights;
pub use weights::WeightInfo;

mod types {
	use crate::*;

	pub type AccountId<T> = <T as frame_system::Config>::AccountId;
	pub type RingBalance<T> = <<T as Config>::RingCurrency as Currency<AccountId<T>>>::Balance;
	pub type EthereumReceiptProofThing<T> = <<T as Config>::EthereumRelay as EthereumReceipt<
		AccountId<T>,
		RingBalance<T>,
	>>::EthereumReceiptProofThing;
}

// --- crates ---
use ethereum_types::{Address, H160, H256, U256};
use sha3::Digest;
// --- substrate ---
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage,
	dispatch::DispatchResultWithPostInfo,
	ensure,
	pallet_prelude::*,
	parameter_types,
	traits::{Currency, ExistenceRequirement::*, Get},
	weights::Weight,
	PalletId,
};
use frame_system::{ensure_root, ensure_signed, pallet_prelude::*};
use sp_runtime::{
	traits::{AccountIdConversion, Keccak256, Saturating},
	AccountId32, DispatchError, DispatchResult, SaturatedConversion,
};
use sp_std::{str, vec::Vec};
// --- darwinia ---
use darwinia_evm::AddressMapping;
use darwinia_evm::GasWeightMapping;
use darwinia_relay_primitives::relay_authorities::*;
use darwinia_support::{balance::*, traits::EthereumReceipt, PalletDigest};
use dp_contract::{
	ethereum_backing::{EthereumBacking, EthereumLockEvent, EthereumRegisterEvent},
	mapping_token_factory::{
		MappingTokenFactory as mtf, TokenBurnInfo, TokenRegisterInfo, BURN_ACTION, REGISTER_ACTION,
	},
};
use dp_evm::CallOrCreateInfo;
use ethereum_primitives::{
	receipt::{EthereumTransactionIndex, LogEntry},
	EthereumAddress,
};
use types::*;

pub trait Config: dvm_ethereum::Config {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

	type PalletId: Get<PalletId>;

	type RingCurrency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

	type EthereumRelay: EthereumReceipt<Self::AccountId, RingBalance<Self>>;
	type EcdsaAuthorities: RelayAuthorityProtocol<Self::BlockNumber, Signer = EthereumAddress>;

	type WeightInfo: WeightInfo;
}

decl_error! {
	/// Issuing pallet errors.
	pub enum Error for Module<T: Config> {
		/// Invalid Issuing System Account
		InvalidIssuingAccount,
		/// assert ar
		AssetAR,
		/// LogEntryNE
		LogEntryNE,
		/// EthLogPF
		EthLogPF,
		/// StringCF
		StringCF,
		/// Unit
		UintCF,
		/// Address - CONVERSION FAILED
		AddressCF,
		/// encode erc20 tx failed
		InvalidEncodeERC20,
		/// encode mint tx failed
		InvalidMintEncoding,
		/// invalid ethereum address length
		InvalidAddressLen,
		/// decode input value error
		InvalidInputData,
		/// decode ethereum event failed
		DecodeEventFailed,
		/// invalid input length
		InvalidInput,
	}
}

decl_event! {
	pub enum Event<T>
	where
		AccountId = AccountId<T>,
	{
		/// register new erc20 token
		RegisterErc20(AccountId, EthereumAddress, EthereumTransactionIndex),
		/// redeem erc20 token
		RedeemErc20(AccountId, EthereumAddress, EthereumTransactionIndex),
		/// erc20 created
		CreateErc20(EthereumAddress),
		/// burn event
		/// type: 1, backing, sender, recipient, source, target, value
		BurnToken(u8, EthereumAddress, EthereumAddress, EthereumAddress, EthereumAddress, EthereumAddress, U256),
		/// token registered event
		/// type: u8 = 0, backing, source(origin erc20), target(mapped erc20)
		TokenRegistered(u8, EthereumAddress, EthereumAddress, EthereumAddress),
		/// set mapping token factory address
		/// [old, new]
		MappingFactoryAddressUpdated(H160, H160),
		/// set ethereum backing address
		/// [old, new]
		EthereumBackingAddressUpdated(H160, H160),
	}
}

decl_storage! {
	trait Store for Module<T: Config> as DarwiniaEthereumIssuing {
		pub MappingFactoryAddress get(fn mapping_factory_address) config(): EthereumAddress;
		pub EthereumBackingAddress get(fn ethereum_backing_address) config(): EthereumAddress;
		pub VerifiedIssuingProof
			get(fn verified_issuing_proof)
			: map hasher(blake2_128_concat) EthereumTransactionIndex => bool = false;
		pub BurnTokenEvents get(fn burn_token_events): Vec<<T as frame_system::Config>::Event>;
	}
}

decl_module! {
	pub struct Module<T: Config> for enum Call
	where
		origin: T::Origin
	{
		fn deposit_event() = default;

		fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
			<BurnTokenEvents<T>>::kill();
			0
		}

		#[weight = <T as darwinia_evm::Config>::GasWeightMapping::gas_to_weight(0x100000)]
		pub fn register_erc20(origin, proof: EthereumReceiptProofThing<T>) {
			log::debug!("start to register erc20 token");
			let user = ensure_signed(origin)?;
			let tx_index = T::EthereumRelay::gen_receipt_index(&proof);
			ensure!(
				!VerifiedIssuingProof::contains_key(tx_index),
				<Error<T>>::AssetAR
				);
			let verified_receipt = T::EthereumRelay::verify_receipt(&proof)?;
			let backing_address = EthereumBackingAddress::get();
			let register_info = EthereumBacking::parse_register_event(
				&verified_receipt,
				&backing_address
			).map_err(|_| Error::<T>::DecodeEventFailed)?;
			let input = mtf::encode_create_erc20(
				Self::digest(),
				0,
				str::from_utf8(&register_info.name[..]).map_err(|_| Error::<T>::StringCF)?,
				str::from_utf8(&register_info.symbol[..]).map_err(|_| Error::<T>::StringCF)?,
				register_info.decimals.as_u32() as u8,
				backing_address,
				register_info.token_address,
			).map_err(|_| Error::<T>::InvalidEncodeERC20)?;
			Self::transact_mapping_factory(input)?;
			VerifiedIssuingProof::insert(tx_index, true);
			Self::deposit_event(RawEvent::RegisterErc20(user, backing_address, tx_index));
		}

		#[weight = <T as darwinia_evm::Config>::GasWeightMapping::gas_to_weight(0x100000)]
		pub fn redeem_erc20(origin, proof: EthereumReceiptProofThing<T>) {
			log::debug!("start to redeem erc20 token");
			let user = ensure_signed(origin)?;
			let tx_index = T::EthereumRelay::gen_receipt_index(&proof);
			ensure!(
				!VerifiedIssuingProof::contains_key(tx_index),
				<Error<T>>::AssetAR
				);
			let verified_receipt = T::EthereumRelay::verify_receipt(&proof)?;
			let backing_address = EthereumBackingAddress::get();
			let lock_info = EthereumBacking::parse_locking_event(
				&verified_receipt,
				&backing_address
			).map_err(|_| Error::<T>::DecodeEventFailed)?;
			let input = mtf::encode_cross_receive(
				lock_info.mapped_address,
				lock_info.recipient,
				lock_info.amount,
			).map_err(|_| Error::<T>::InvalidEncodeERC20)?;
			Self::transact_mapping_factory(input)?;
			VerifiedIssuingProof::insert(tx_index, true);
			Self::deposit_event(RawEvent::RedeemErc20(user, backing_address, tx_index));
		}

		#[weight = 0]
		pub fn mapping_factory_event_handle(
			origin,
			input: Vec<u8>,
		) {
			let caller = ensure_signed(origin)?;
			ensure!(input.len() >= 8, <Error<T>>::InvalidInput);
			let factory = MappingFactoryAddress::get();
			let factory_id = <T as darwinia_evm::Config>::AddressMapping::into_account_id(factory);
			ensure!(factory_id == caller, <Error<T>>::AssetAR);
			let burn_action = &sha3::Keccak256::digest(&BURN_ACTION)[0..4];
			let register_action = &sha3::Keccak256::digest(&REGISTER_ACTION)[0..4];
			if &input[4..8] == burn_action {
				let burn_info =
					TokenBurnInfo::decode(&input[8..]).map_err(|_| Error::<T>::InvalidInputData)?;
				ensure!(burn_info.recipient.len() == 20, <Error<T>>::AssetAR);
				Self::deposit_burn_token_event(
					burn_info.backing,
					burn_info.sender,
					burn_info.source,
					EthereumAddress::from_slice(burn_info.recipient.as_slice()),
					burn_info.amount,
					)?;
			} else if &input[4..8] == register_action {
				let register_info =
					TokenRegisterInfo::decode(&input[8..]).map_err(|_| Error::<T>::InvalidInputData)?;
				Self::finish_token_registered(register_info.0, register_info.1, register_info.2);
			} else {
				log::trace!("Unsupport action!");
			}
		}

		#[weight = 0]
		pub fn set_mapping_factory_address(
			origin,
			address: H160,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			let old_address = MappingFactoryAddress::get();
			MappingFactoryAddress::put(address);
			Self::deposit_event(RawEvent::MappingFactoryAddressUpdated(old_address, address));
			Ok(().into())
		}

		#[weight = 0]
		pub fn set_ethereum_backing_address(
			origin,
			address: H160,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			let old_address = EthereumBackingAddress::get();
			EthereumBackingAddress::put(address);
			Self::deposit_event(RawEvent::EthereumBackingAddressUpdated(old_address, address));
			Ok(().into())
		}
	}
}

impl<T: Config> Module<T> {
	/// The account ID of the issuing pot.
	pub fn dvm_account_id() -> H160 {
		let account32: AccountId32 = T::PalletId::get().into_account();
		let account20: &[u8] = &account32.as_ref();
		H160::from_slice(&account20[..20])
	}

	pub fn digest() -> PalletDigest {
		let mut digest: PalletDigest = Default::default();
		let pallet_digest = sha3::Keccak256::digest(T::PalletId::get().encode().as_slice());
		digest.copy_from_slice(&pallet_digest[..4]);
		digest
	}

	pub fn mapped_token_address(
		backing: EthereumAddress,
		source: EthereumAddress,
	) -> Result<H160, DispatchError> {
		let factory_address = MappingFactoryAddress::get();
		let bytes = mtf::encode_mapping_token(backing, source)
			.map_err(|_| Error::<T>::InvalidIssuingAccount)?;
		let mapped_address = dvm_ethereum::Pallet::<T>::do_call(factory_address, bytes)
			.map_err(|e| -> &'static str { e.into() })?;
		if mapped_address.len() != 32 {
			return Err(Error::<T>::InvalidAddressLen.into());
		}
		Ok(H160::from_slice(&mapped_address.as_slice()[12..]))
	}

	pub fn finish_token_registered(
		backing: EthereumAddress,
		source: EthereumAddress,
		target: EthereumAddress,
	) -> DispatchResult {
		let raw_event = RawEvent::TokenRegistered(0, backing, source, target);
		let module_event: <T as Config>::Event = raw_event.clone().into();
		let system_event: <T as frame_system::Config>::Event = module_event.into();
		<BurnTokenEvents<T>>::append(system_event);
		Self::deposit_event(raw_event);
		T::EcdsaAuthorities::schedule_mmr_root(
			(<frame_system::Pallet<T>>::block_number().saturated_into::<u32>() / 10 * 10 + 10)
				.saturated_into(),
		);
		Ok(())
	}

	pub fn deposit_burn_token_event(
		backing: EthereumAddress,
		sender: EthereumAddress,
		source: EthereumAddress,
		recipient: EthereumAddress,
		amount: U256,
	) -> DispatchResult {
		let mapped_address = Self::mapped_token_address(backing, source).map_err(|e| {
			log::debug!("mapped token address error {:?} ", e);
			e
		})?;

		let raw_event = RawEvent::BurnToken(
			1,
			backing,
			sender,
			recipient,
			source,
			mapped_address,
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
		Ok(())
	}

	pub fn transact_mapping_factory(input: Vec<u8>) -> DispatchResult {
		let contract = MappingFactoryAddress::get();
		let result = dvm_ethereum::Pallet::<T>::internal_transact(contract, input).map_err(
			|e| -> &'static str {
				log::debug!("call mapping factory contract error {:?}", &e);
				e.into()
			},
		)?;
		Ok(())
	}
}
