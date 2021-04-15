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

// --- crates ---
pub mod weights;
use ethereum_types::{Address, H160, H256, U256};
use rustc_hex::{FromHex, ToHex};
pub use weights::WeightInfo;
// --- substrate ---
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage,
	dispatch::DispatchResultWithPostInfo,
	ensure, parameter_types,
	traits::{Currency, ExistenceRequirement::*, Get},
	weights::Weight,
};
use frame_system::{ensure_root, ensure_signed};
use sp_runtime::{
	traits::{AccountIdConversion, Saturating},
	AccountId32, DispatchError, DispatchResult, ModuleId, SaturatedConversion,
};
use sp_std::vec::Vec;
// --- darwinia ---
use darwinia_ethereum_issuing_contract::{
	Abi, Event as EthEvent, Log as EthLog, TokenBurnInfo, TokenRegisterInfo,
};
use darwinia_evm::{
	AddressMapping, FeeCalculator, GasWeightMapping, IssuingHandler,
};
use darwinia_relay_primitives::relay_authorities::*;
use darwinia_support::{balance::lock::*, traits::EthereumReceipt};
use dp_evm::CallOrCreateInfo;
use dvm_ethereum::{TransactionAction, TransactionSignature};

mod types {
	use crate::*;

	pub type BlockNumber<T> = <T as frame_system::Config>::BlockNumber;
	pub type AccountId<T> = <T as frame_system::Config>::AccountId;
	pub type RingBalance<T> = <<T as Config>::RingCurrency as Currency<AccountId<T>>>::Balance;
	pub type EthereumReceiptProofThing<T> = <<T as Config>::EthereumRelay as EthereumReceipt<
		AccountId<T>,
		RingBalance<T>,
	>>::EthereumReceiptProofThing;
}

use ethereum_primitives::{
	receipt::{EthereumTransactionIndex, LogEntry},
	EthereumAddress,
};
use types::*;

pub trait Config: dvm_ethereum::Config {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

	type ModuleId: Get<ModuleId>;

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
		/// ReceiptProofInv
		ReceiptProofInv,
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
		InvalidMintEcoding,
		/// invalid ethereum address length
		InvalidAddressLen,
		/// decode input value error
		InvalidInputData,
	}
}

decl_event! {
	pub enum Event<T>
	where
		AccountId = AccountId<T>,
	{
		/// register new erc20 token
		RegisterErc20(AccountId, EthereumAddress),
		/// redeem erc20 token
		RedeemErc20(AccountId, EthereumAddress),
		/// erc20 created
		CreateErc20(EthereumAddress),
		/// burn event
		/// type: 1, backing, recipient, source, target, value
		BurnToken(u8, EthereumAddress, EthereumAddress, EthereumAddress, EthereumAddress, U256),
		/// token registed event
		/// type: u8 = 0, backing, source(origin erc20), target(mapped erc20)
		TokenRegisted(u8, EthereumAddress, EthereumAddress, EthereumAddress),
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

		fn on_initialize(_n: BlockNumber<T>) -> Weight {
			<BurnTokenEvents<T>>::kill();
			0
		}

		#[weight = <T as darwinia_evm::Config>::GasWeightMapping::gas_to_weight(0x100000)]
		pub fn register_erc20(origin, proof: EthereumReceiptProofThing<T>) {
			log::info!(target: "darwinia-issuing", "start to register erc20 token");
			let user = ensure_signed(origin)?;
			let (tx_index, ethlog) = Self::verify_and_parse_proof(
				Abi::register_event(),
				proof)?;
			let backing_address = EthereumBackingAddress::get();
			let input = Self::abi_encode_token_creation(backing_address, ethlog)?;
			Self::transact_mapping_factory(input)?;
			VerifiedIssuingProof::insert(tx_index, true);
			Self::deposit_event(RawEvent::RegisterErc20(user, backing_address));
		}

		#[weight = <T as darwinia_evm::Config>::GasWeightMapping::gas_to_weight(0x100000)]
		pub fn redeem_erc20(origin, proof: EthereumReceiptProofThing<T>) {
			log::info!(target: "darwinia-issuing", "start to redeem erc20 token");
			let user = ensure_signed(origin)?;
			let (tx_index, ethlog) = Self::verify_and_parse_proof(
				Abi::backing_event(),
				proof)?;
			let backing_address = EthereumBackingAddress::get();
			let input = Self::abi_encode_token_redeem(ethlog)?;
			Self::transact_mapping_factory(input)?;
			VerifiedIssuingProof::insert(tx_index, true);
			Self::deposit_event(RawEvent::RedeemErc20(user, backing_address));
		}
	}
}

impl<T: Config> Module<T> {
	/// The account ID of the issuing pot.
	pub fn dvm_account_id() -> H160 {
		let account32: AccountId32 = T::ModuleId::get().into_account();
		let account20: &[u8] = &account32.as_ref();
		H160::from_slice(&account20[..20])
	}

	/// get dvm ethereum unsigned transaction
	pub fn unsigned_transaction(target: H160, input: Vec<u8>) -> dvm_ethereum::Transaction {
		dvm_ethereum::Transaction {
			nonce: U256::zero(),
			gas_price: U256::zero(),
			gas_limit: U256::from(0x100000),
			action: dvm_ethereum::TransactionAction::Call(target),
			value: U256::zero(),
			input,
			signature: TransactionSignature::new(
				// v = CHAIN_ID * 2 + 36, and CHAIN_ID = 42 for pangolin
				0x78,
				H256::from_slice(&[55u8; 32]),
				H256::from_slice(&[55u8; 32]),
			)
			.unwrap()
		}
	}

	fn abi_encode_token_creation(
		backing: EthereumAddress,
		result: EthLog,
	) -> Result<Vec<u8>, DispatchError> {
		log::info!(target: "darwinia-issuing", "start to abi_encode_token_creation");
		let name = result.params[1]
			.value
			.clone()
			.into_string()
			.ok_or(<Error<T>>::StringCF)?;
		let symbol = result.params[2]
			.value
			.clone()
			.into_string()
			.ok_or(<Error<T>>::StringCF)?;
		let decimals = result.params[3]
			.value
			.clone()
			.into_uint()
			.ok_or(<Error<T>>::UintCF)?;
		let fee = result.params[4]
			.value
			.clone()
			.into_uint()
			.ok_or(<Error<T>>::UintCF)?;
		let token_address = result.params[0]
			.value
			.clone()
			.into_address()
			.ok_or(<Error<T>>::AddressCF)?;

		let input = Abi::encode_create_erc20(
			&name,
			&symbol,
			decimals.as_u32() as u8,
			backing,
			token_address,
		)
		.map_err(|_| Error::<T>::InvalidEncodeERC20)?;

		log::info!(target: "darwinia-issuing", "register fee will be delived to fee pallet {}", fee);
		Ok(input)
	}

	fn abi_encode_token_redeem(result: EthLog) -> Result<Vec<u8>, DispatchError> {
		log::info!(target: "darwinia-issuing", "abi_encode_token_redeem");
		let token_address = result.params[0]
			.value
			.clone()
			.into_address()
			.ok_or(<Error<T>>::AddressCF)?;
		let dtoken_address = result.params[1]
			.value
			.clone()
			.into_address()
			.ok_or(<Error<T>>::AddressCF)?;
		let amount = result.params[2]
			.value
			.clone()
			.into_uint()
			.ok_or(<Error<T>>::UintCF)?;
		let recipient = result.params[3]
			.value
			.clone()
			.into_address()
			.ok_or(<Error<T>>::AddressCF)?;
		let fee = result.params[4]
			.value
			.clone()
			.into_uint()
			.ok_or(<Error<T>>::UintCF)?;

		let input = Abi::encode_cross_receive(dtoken_address, recipient, amount)
			.map_err(|_| Error::<T>::InvalidMintEcoding)?;

		log::info!(target: "darwinia-issuing", "transfer fee will be delived to fee pallet {}", fee);
		Ok(input)
	}

	pub fn mapped_token_address(
		backing: EthereumAddress,
		source: EthereumAddress,
	) -> Result<H160, DispatchError> {
		let factory_address = MappingFactoryAddress::get();
		let bytes = Abi::encode_mapping_token(backing, source)
			.map_err(|_| Error::<T>::InvalidIssuingAccount)?;
		let mapped_address = dvm_ethereum::Module::<T>::do_call(factory_address, bytes)
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
		let raw_event = RawEvent::TokenRegisted(0, backing, source, target);
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
		source: EthereumAddress,
		recipient: EthereumAddress,
		amount: U256,
	) -> DispatchResult {
		let mapped_address = Self::mapped_token_address(backing, source).map_err(|e| {
			log::info!(target: "darwinia-issuing", "mapped token address error {:?} ", e);
			e
		})?;

		let raw_event = RawEvent::BurnToken(
			1,
			backing,
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

	pub fn verify_and_parse_proof(
		log_event: EthEvent,
		proof: EthereumReceiptProofThing<T>,
	) -> Result<(EthereumTransactionIndex, EthLog), DispatchError> {
		let tx_index = T::EthereumRelay::gen_receipt_index(&proof);
		ensure!(
			!VerifiedIssuingProof::contains_key(tx_index),
			<Error<T>>::AssetAR
		);
		let verified_receipt = T::EthereumRelay::verify_receipt(&proof).map_err(|err| {
			log::info!(target: "darwinia-issuing", "verify error {:?}", err);
			<Error<T>>::ReceiptProofInv
		})?;

		let backing_address = EthereumBackingAddress::get();
		let log_entry = verified_receipt
			.logs
			.into_iter()
			.find(|x| x.address == backing_address && x.topics[0] == log_event.signature())
			.ok_or(<Error<T>>::LogEntryNE)?;

		let ethlog = Abi::parse_event(
			log_entry.topics.into_iter().collect(),
			log_entry.data.clone(),
			log_event,
		)
		.map_err(|_| <Error<T>>::EthLogPF)?;

		Ok((tx_index, ethlog))
	}

	pub fn transact_mapping_factory(input: Vec<u8>) -> DispatchResult {
		let contract = MappingFactoryAddress::get();
		let transaction = Self::unsigned_transaction(contract, input);
		let result = dvm_ethereum::Module::<T>::do_transact(transaction, false).map_err(
			|e| -> &'static str {
				log::info!(target: "darwinia-issuing", "call mapping factory contract error {:?}", &e);
				e.into()
			},
		)?;
		Ok(())
	}
}

impl<T: Config> IssuingHandler for Module<T> {
	/// handle
	fn handle(address: H160, caller: H160, input: &[u8]) -> DispatchResult {
		ensure!(MappingFactoryAddress::get() == caller, <Error<T>>::AssetAR);
		// in order to use a common precompile contract to deliver these issuing events
		// we just use the len of input to distinguish which event.
		// register-event: input-len = len(abi.encode(backing, source, token)) = 3 * 32
		// burn-event: input-len = len(abi.encode(info.backing, info.source, recipient, amount)) = 4 * 32
		if input.len() == 3 * 32 {
			let register_info =
				TokenRegisterInfo::decode(input).map_err(|_| Error::<T>::InvalidInputData)?;
			Self::finish_token_registered(register_info.0, register_info.1, register_info.2)
		} else {
			let burn_info =
				TokenBurnInfo::decode(input).map_err(|_| Error::<T>::InvalidInputData)?;
			Self::deposit_burn_token_event(
				burn_info.backing,
				burn_info.source,
				burn_info.recipient,
				U256(burn_info.amount.0),
			)
		}
	}
}
