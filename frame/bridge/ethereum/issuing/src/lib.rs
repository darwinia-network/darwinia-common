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

// --- substrate ---
use darwinia_evm::{AccountBasicMapping, AddressMapping, ContractHandler, GasWeightMapping};
use darwinia_relay_primitives::relay_authorities::*;
use dp_evm::CallOrCreateInfo;
use dvm_ethereum::TransactionAction;
use dvm_ethereum::TransactionSignature;
use ethereum_types::{Address, H160, H256, U256};
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage,
	dispatch::DispatchResultWithPostInfo,
	ensure, parameter_types,
	traits::{Currency, ExistenceRequirement::*, Get},
	weights::Weight,
};
use frame_system::{ensure_root, ensure_signed};
use rustc_hex::{FromHex, ToHex};

use sp_std::vec::Vec;

use sp_runtime::{
	traits::{AccountIdConversion, Saturating},
	AccountId32, DispatchError, DispatchResult, ModuleId, SaturatedConversion,
};

use darwinia_support::{
	balance::lock::*,
	traits::{DvmRawTransactor as DvmRawTransactorT, EthereumReceipt},
};

pub mod weights;
// --- darwinia ---
pub use weights::WeightInfo;

use darwinia_ethereum_issuing_contract::{
	Abi, Event as EthEvent, Log as EthLog, TokenBurnInfo, TokenRegisterInfo, Topic,
};

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

pub trait Config: frame_system::Config + darwinia_evm::Config {
	type ModuleId: Get<ModuleId>;
	type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;
	type DvmCaller: DvmRawTransactorT<H160, dvm_ethereum::Transaction, DispatchResultWithPostInfo>;
	type EthereumRelay: EthereumReceipt<Self::AccountId, RingBalance<Self>>;
	type RingCurrency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;
	type EcdsaAuthorities: RelayAuthorityProtocol<Self::BlockNumber, Signer = EthereumAddress>;
	type WeightInfo: WeightInfo;
	type FeeEstimate: Get<RingBalance<Self>>;
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
		/// test
		Test(AccountId),
		/// erc20 created
		CreateErc20(EthereumAddress),
		/// burn event
		/// type: 1, backing, recipient, delegator, source, target, value
		BurnToken(u8, EthereumAddress, EthereumAddress, EthereumAddress, EthereumAddress, EthereumAddress, U256),
		/// token registed event
		/// type: u8 = 0, backing, source(origin erc20), target(mapped erc20)
		TokenRegisted(u8, EthereumAddress, EthereumAddress, EthereumAddress),
	}
}

decl_storage! {
	trait Store for Module<T: Config> as DarwiniaEthereumIssuing {
		pub MappingFactoryAddress get(fn mapping_factory_address) config(): EthereumAddress;
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
		const FeeEstimate: RingBalance<T> = T::FeeEstimate::get();

		fn deposit_event() = default;

		fn on_initialize(_n: BlockNumber<T>) -> Weight {
			<BurnTokenEvents<T>>::kill();
			0
		}

		#[weight = <T as darwinia_evm::Config>::GasWeightMapping::gas_to_weight(0x100000)]
		pub fn register_or_redeem_erc20(origin, backing: EthereumAddress, proof: EthereumReceiptProofThing<T>) {
			log::trace!(target: "darwinia-issuing", "start to register_or_issuing_erc20");
			let user = ensure_signed(origin)?;
			let tx_index = T::EthereumRelay::gen_receipt_index(&proof);
			ensure!(!VerifiedIssuingProof::contains_key(tx_index), <Error<T>>::AssetAR);
			let verified_receipt = T::EthereumRelay::verify_receipt(&proof)
				.map_err(|err| {
					log::trace!(target: "darwinia-issuing", "verify error {:?}", err);
					<Error<T>>::ReceiptProofInv
				})?;

			let register_event = Abi::register_event();
			let backing_event = Abi::backing_event();
			let log_entry = verified_receipt
				.logs
				.into_iter()
				.find(|x| {
					x.address == backing &&
						( x.topics[0] == register_event.signature()
						  || x.topics[0] == backing_event.signature() )
				})
			.ok_or(<Error<T>>::LogEntryNE)?;

			let input = if log_entry.topics[0] == register_event.signature() {
				let ethlog = Self::parse_event(register_event, log_entry)?;
				Self::process_erc20_creation(backing, ethlog)?
			} else {
				let ethlog = Self::parse_event(backing_event, log_entry)?;
				Self::process_token_issuing(ethlog)?
			};

			let contract = MappingFactoryAddress::get();
			let account = Self::dvm_account_id();
			let basic = <T as darwinia_evm::Config>::AccountBasicMapping::account_basic(&account);
			let substrate_account = <T as darwinia_evm::Config>::AddressMapping::into_account_id(account.clone());

			<T as Config>::RingCurrency::transfer(&user, &substrate_account, T::FeeEstimate::get(), KeepAlive)?;
			let transaction = Self::unsigned_transaction(basic.nonce, contract.0.into(), input);
			let result = T::DvmCaller::raw_transact(account, transaction).map_err(|e| -> &'static str {
				log::trace!(target: "darwinia-issuing", "register_or_issuing_erc20 error {:?}", &e);
				e.into()
			} )?;

			let leaved_balance = <T as Config>::RingCurrency::free_balance(&substrate_account);
			// we should reserve some balance to keepalive this system account
			let maxrefund = leaved_balance.saturating_sub(<T as Config>::RingCurrency::minimum_balance());

			<T as Config>::RingCurrency::transfer(&substrate_account, &user, maxrefund, KeepAlive)?;
			VerifiedIssuingProof::insert(tx_index, true);
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
	pub fn unsigned_transaction(
		nonce: U256,
		target: H160,
		input: Vec<u8>,
	) -> dvm_ethereum::Transaction {
		dvm_ethereum::Transaction {
			nonce,
			gas_price: U256::from(1),
			gas_limit: U256::from(0x100000),
			action: dvm_ethereum::TransactionAction::Call(target),
			value: U256::zero(),
			input,
			signature: TransactionSignature::new(
				0x78,
				H256::from_slice(&[55u8; 32]),
				H256::from_slice(&[55u8; 32]),
			)
			.unwrap(),
		}
	}

	fn parse_event(event: EthEvent, log_entry: LogEntry) -> Result<EthLog, DispatchError> {
		let ethlog = Abi::parse_event(
			log_entry
				.topics
				.into_iter()
				.map(|t| -> Topic { t.0.into() })
				.collect(),
			log_entry.data.clone(),
			event,
		)
		.map_err(|_| <Error<T>>::EthLogPF)?;
		Ok(ethlog)
	}

	fn process_erc20_creation(
		backing: EthereumAddress,
		result: EthLog,
	) -> Result<Vec<u8>, DispatchError> {
		log::trace!(target: "darwinia-issuing", "start to process_erc20_creation");
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
			backing.0.into(),
			token_address.0.into(),
		)
		.map_err(|_| Error::<T>::InvalidEncodeERC20)?;

		log::trace!(target: "darwinia-issuing", "register fee will be delived to fee pallet {}", fee);
		Ok(input)
	}

	fn process_token_issuing(result: EthLog) -> Result<Vec<u8>, DispatchError> {
		log::trace!(target: "darwinia-issuing", "process_token_issuing");
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

		let input =
			Abi::encode_cross_receive(dtoken_address.0.into(), recipient.0.into(), amount.0.into())
				.map_err(|_| Error::<T>::InvalidMintEcoding)?;

		log::trace!(target: "darwinia-issuing", "transfer fee will be delived to fee pallet {}", fee);
		Ok(input)
	}

	pub fn mapped_token_address(
		backing: EthereumAddress,
		source: EthereumAddress,
	) -> Result<H160, DispatchError> {
		let factory_address = MappingFactoryAddress::get();
		let bytes = Abi::encode_mapping_token(backing.0.into(), source.0.into())
			.map_err(|_| Error::<T>::InvalidIssuingAccount)?;
		let transaction =
			Self::unsigned_transaction(U256::from(1), factory_address.0.into(), bytes);
		let account = Self::dvm_account_id();
		let mapped_address = T::DvmCaller::raw_call(account, transaction)
			.map_err(|e| -> &'static str { e.into() })?;
		if mapped_address.len() != 32 {
			return Err(Error::<T>::InvalidAddressLen.into());
		}
		Ok(H160::from_slice(&mapped_address.as_slice()[12..]))
	}

	pub fn token_registed(
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

	pub fn burn_token(
		backing: EthereumAddress,
		source: EthereumAddress,
		recipient: EthereumAddress,
		delegator: EthereumAddress,
		amount: U256,
	) -> DispatchResult {
		let mapped_address = Self::mapped_token_address(backing, source).map_err(|e| {
			log::trace!(target: "darwinia-issuing", "mapped token address error {:?} ", e);
			e
		})?;

		let raw_event = RawEvent::BurnToken(
			1,
			backing,
			recipient,
			delegator,
			source,
			mapped_address.0.into(),
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
}

impl<T: Config> ContractHandler for Module<T> {
	/// handle
	fn handle(address: H160, caller: H160, input: &[u8]) -> DispatchResult {
        ensure!(MappingFactoryAddress::get() == caller.0.into(), <Error<T>>::AssetAR);
        if input.len() == 3 * 32 {
            let registed_info =
                TokenRegisterInfo::decode(input).map_err(|_| Error::<T>::InvalidInputData)?;
            Self::token_registed(registed_info.0, registed_info.1, registed_info.2)
        } else {
            let burn_info =
                TokenBurnInfo::decode(input).map_err(|_| Error::<T>::InvalidInputData)?;
            Self::burn_token(
                burn_info.backing,
                burn_info.source,
                burn_info.recipient,
                burn_info.delegator,
                U256(burn_info.amount.0),
                )
        }
	}
}
