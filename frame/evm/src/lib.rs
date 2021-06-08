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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

//! EVM execution pallet for Substrate

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

pub mod runner;

#[cfg(test)]
mod tests;

pub use crate::runner::Runner;
pub use dp_evm::{
	Account, CallInfo, CreateInfo, ExecutionInfo, LinearCostPrecompile, Log, Precompile,
	PrecompileSet, Vicinity,
};

// --- crates.io ---
#[cfg(feature = "std")]
use codec::{Decode, Encode};
use evm::{Config as EvmConfig, ExitError, ExitReason};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

// --- substrate ---
use frame_support::{
	traits::Currency,
	weights::{PostDispatchInfo, Weight},
};
use frame_system::RawOrigin;
use sp_core::{H160, H256, U256};
use sp_runtime::{
	traits::{BadOrigin, UniqueSaturatedInto},
	AccountId32, DispatchResult,
};
use sp_std::prelude::*;

static ISTANBUL_CONFIG: EvmConfig = EvmConfig::istanbul();

#[frame_support::pallet]
pub mod pallet {
	// --- substrate ---
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	// --- darwinia ---
	use crate::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_timestamp::Config {
		/// Calculator for current gas price.
		type FeeCalculator: FeeCalculator;
		/// Maps Ethereum gas to Substrate weight.
		type GasWeightMapping: GasWeightMapping;
		/// Allow the origin to call on behalf of given address.
		type CallOrigin: EnsureAddressOrigin<Self::Origin>;

		/// Mapping from address to account id.
		type AddressMapping: AddressMapping<Self::AccountId>;
		/// Ring Currency type
		type RingCurrency: Currency<Self::AccountId>;
		/// Kton Currency type
		type KtonCurrency: Currency<Self::AccountId>;

		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// Precompiles associated with this EVM engine.
		type Precompiles: PrecompileSet;
		/// Chain ID of EVM.
		type ChainId: Get<u64>;
		/// The block gas limit. Can be a simple constant, or an adjustment algorithm in another pallet.
		type BlockGasLimit: Get<U256>;
		/// EVM execution runner.
		type Runner: Runner<Self>;
		/// The account basic mapping way
		type RingAccountBasic: AccountBasic<Self>;
		type KtonAccountBasic: AccountBasic<Self>;
		/// Issuing contracts handler
		type IssuingHandler: IssuingHandler;

		/// EVM config used in the Pallet.
		fn config() -> &'static EvmConfig {
			&ISTANBUL_CONFIG
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	#[pallet::metadata(T::AccountId = "AccountId")]
	pub enum Event<T: Config> {
		/// Ethereum events from contracts.
		Log(Log),
		/// A contract has been created at given \[address\].
		Created(H160),
		/// A \[contract\] was attempted to be created, but the execution failed.
		CreatedFailed(H160),
		/// A \[contract\] has been executed successfully with states applied.
		Executed(H160),
		/// A \[contract\] has been executed with errors. States are reverted with only gas fees applied.
		ExecutedFailed(H160),
		/// A deposit has been made at a given address. \[sender, address, value\]
		BalanceDeposit(T::AccountId, H160, U256),
		/// A withdrawal has been made from a given address. \[sender, address, value\]
		BalanceWithdraw(T::AccountId, H160, U256),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Not enough balance to perform action
		BalanceLow,
		/// Calculating total fee overflowed
		FeeOverflow,
		/// Calculating total payment overflowed
		PaymentOverflow,
		/// Withdraw fee failed
		WithdrawFailed,
		/// Gas price is too low.
		GasPriceTooLow,
		/// Nonce is invalid
		InvalidNonce,
	}

	#[pallet::storage]
	#[pallet::getter(fn account_codes)]
	pub type AccountCodes<T: Config> = StorageMap<_, Blake2_128Concat, H160, Vec<u8>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn account_storages)]
	pub type AccountStorages<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, H160, Blake2_128Concat, H256, H256, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig {
		pub accounts: std::collections::BTreeMap<H160, GenesisAccount>,
	}
	#[cfg(feature = "std")]
	impl Default for GenesisConfig {
		fn default() -> Self {
			Self {
				accounts: Default::default(),
			}
		}
	}
	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {
			let extra_genesis_builder: fn(&Self) = |config: &GenesisConfig| {
				for (address, account) in &config.accounts {
					T::RingAccountBasic::mutate_account_basic_balance(&address, account.balance);
					T::KtonAccountBasic::mutate_account_basic_balance(&address, account.balance);
					AccountCodes::<T>::insert(address, &account.code);
					for (index, value) in &account.storage {
						AccountStorages::<T>::insert(address, index, value);
					}
				}
			};
			extra_genesis_builder(self);
		}
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(PhantomData<T>);
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Issue an EVM call operation. This is similar to a message call transaction in Ethereum.
		#[pallet::weight(T::GasWeightMapping::gas_to_weight(*gas_limit))]
		pub(super) fn call(
			origin: OriginFor<T>,
			source: H160,
			target: H160,
			input: Vec<u8>,
			value: U256,
			gas_limit: u64,
			gas_price: U256,
			nonce: Option<U256>,
		) -> DispatchResultWithPostInfo {
			T::CallOrigin::ensure_address_origin(&source, origin)?;

			let info = T::Runner::call(
				source,
				target,
				input,
				value,
				gas_limit,
				Some(gas_price),
				nonce,
				T::config(),
			)?;

			match info.exit_reason {
				ExitReason::Succeed(_) => {
					Pallet::<T>::deposit_event(Event::<T>::Executed(target));
				}
				_ => {
					Pallet::<T>::deposit_event(Event::<T>::ExecutedFailed(target));
				}
			};

			Ok(PostDispatchInfo {
				actual_weight: Some(T::GasWeightMapping::gas_to_weight(
					info.used_gas.unique_saturated_into(),
				)),
				pays_fee: Pays::No,
			})
		}

		/// Issue an EVM create operation. This is similar to a contract creation transaction in
		/// Ethereum.
		#[pallet::weight(T::GasWeightMapping::gas_to_weight(*gas_limit))]
		fn create(
			origin: OriginFor<T>,
			source: H160,
			init: Vec<u8>,
			value: U256,
			gas_limit: u64,
			gas_price: U256,
			nonce: Option<U256>,
		) -> DispatchResultWithPostInfo {
			T::CallOrigin::ensure_address_origin(&source, origin)?;

			let info = T::Runner::create(
				source,
				init,
				value,
				gas_limit,
				Some(gas_price),
				nonce,
				T::config(),
			)?;
			match info {
				CreateInfo {
					exit_reason: ExitReason::Succeed(_),
					value: create_address,
					..
				} => {
					Pallet::<T>::deposit_event(Event::<T>::Created(create_address));
				}
				CreateInfo {
					exit_reason: _,
					value: create_address,
					..
				} => {
					Pallet::<T>::deposit_event(Event::<T>::CreatedFailed(create_address));
				}
			}

			Ok(PostDispatchInfo {
				actual_weight: Some(T::GasWeightMapping::gas_to_weight(
					info.used_gas.unique_saturated_into(),
				)),
				pays_fee: Pays::No,
			})
		}

		/// Issue an EVM create2 operation.
		#[pallet::weight(T::GasWeightMapping::gas_to_weight(*gas_limit))]
		fn create2(
			origin: OriginFor<T>,
			source: H160,
			init: Vec<u8>,
			salt: H256,
			value: U256,
			gas_limit: u64,
			gas_price: U256,
			nonce: Option<U256>,
		) -> DispatchResultWithPostInfo {
			T::CallOrigin::ensure_address_origin(&source, origin)?;

			let info = T::Runner::create2(
				source,
				init,
				salt,
				value,
				gas_limit,
				Some(gas_price),
				nonce,
				T::config(),
			)?;
			match info {
				CreateInfo {
					exit_reason: ExitReason::Succeed(_),
					value: create_address,
					..
				} => {
					Pallet::<T>::deposit_event(Event::<T>::Created(create_address));
				}
				CreateInfo {
					exit_reason: _,
					value: create_address,
					..
				} => {
					Pallet::<T>::deposit_event(Event::<T>::CreatedFailed(create_address));
				}
			}

			Ok(PostDispatchInfo {
				actual_weight: Some(T::GasWeightMapping::gas_to_weight(
					info.used_gas.unique_saturated_into(),
				)),
				pays_fee: Pays::No,
			})
		}
	}
	impl<T: Config> Pallet<T> {
		pub fn remove_account(address: &H160) {
			if AccountCodes::<T>::contains_key(address) {
				let account_id = T::AddressMapping::into_account_id(*address);
				let _ = <frame_system::Pallet<T>>::dec_consumers(&account_id);
			}

			AccountCodes::<T>::remove(address);
			AccountStorages::<T>::remove_prefix(address);
		}

		/// Create an account.
		pub fn create_account(address: H160, code: Vec<u8>) {
			if code.is_empty() {
				return;
			}

			if !AccountCodes::<T>::contains_key(&address) {
				let account_id = T::AddressMapping::into_account_id(address);
				let _ = <frame_system::Pallet<T>>::inc_consumers(&account_id);
			}

			AccountCodes::<T>::insert(address, code);
		}

		/// Check whether an account is empty.
		pub fn is_account_empty(address: &H160) -> bool {
			let account = T::RingAccountBasic::account_basic(address);
			let code_len = AccountCodes::<T>::decode_len(address).unwrap_or(0);

			account.nonce == U256::zero() && account.balance == U256::zero() && code_len == 0
		}

		pub fn is_contract_code_empty(address: &H160) -> bool {
			let code_len = AccountCodes::<T>::decode_len(address).unwrap_or(0);
			code_len == 0
		}

		/// Remove an account if its empty.
		pub fn remove_account_if_empty(address: &H160) {
			if Self::is_account_empty(address) {
				Self::remove_account(address);
			}
		}

		/// Withdraw fee.
		pub fn withdraw_fee(address: &H160, value: U256) {
			let account = T::RingAccountBasic::account_basic(address);
			let new_account_balance = account.balance.saturating_sub(value);

			T::RingAccountBasic::mutate_account_basic_balance(&address, new_account_balance);
		}

		/// Deposit fee.
		pub fn deposit_fee(address: &H160, value: U256) {
			let account = T::RingAccountBasic::account_basic(address);
			let new_account_balance = account.balance.saturating_add(value);

			T::RingAccountBasic::mutate_account_basic_balance(&address, new_account_balance);
		}
	}
}
pub use pallet::*;

pub trait EnsureAddressOrigin<OuterOrigin> {
	/// Success return type.
	type Success;

	/// Perform the origin check.
	fn ensure_address_origin(
		address: &H160,
		origin: OuterOrigin,
	) -> Result<Self::Success, BadOrigin> {
		Self::try_address_origin(address, origin).map_err(|_| BadOrigin)
	}

	/// Try with origin.
	fn try_address_origin(
		address: &H160,
		origin: OuterOrigin,
	) -> Result<Self::Success, OuterOrigin>;
}

pub trait AddressMapping<A> {
	fn into_account_id(address: H160) -> A;
}

/// Account basic info operations
pub trait AccountBasic<T: frame_system::Config> {
	/// Get the account basic in EVM format.
	fn account_basic(address: &H160) -> Account;
	/// Mutate the basic account
	fn mutate_account_basic_balance(address: &H160, new_balance: U256);
	/// Transfer value
	fn transfer(source: &H160, target: &H160, value: U256) -> Result<(), ExitError>;
	/// Get account balance
	fn account_balance(account_id: &T::AccountId) -> U256;
	/// Mutate account balance
	fn mutate_account_balance(account_id: &T::AccountId, balance: U256);
}

/// Config that outputs the current transaction gas price.
pub trait FeeCalculator {
	/// Return the minimal required gas price.
	fn min_gas_price() -> U256;
}
impl FeeCalculator for () {
	fn min_gas_price() -> U256 {
		U256::zero()
	}
}

/// A mapping function that converts Ethereum gas to Substrate weight
pub trait GasWeightMapping {
	fn gas_to_weight(gas: u64) -> Weight;
	fn weight_to_gas(weight: Weight) -> u64;
}
impl GasWeightMapping for () {
	fn gas_to_weight(gas: u64) -> Weight {
		gas as Weight
	}
	fn weight_to_gas(weight: Weight) -> u64 {
		weight
	}
}

/// A contract handle for ethereum issuing
pub trait IssuingHandler {
	fn handle(address: H160, caller: H160, input: &[u8]) -> DispatchResult;
}
/// A default empty issuingHandler, usually used in the test scenario.
impl IssuingHandler for () {
	fn handle(_: H160, _: H160, _: &[u8]) -> DispatchResult {
		Ok(())
	}
}

/// Ensure that the address is truncated hash of the origin. Only works if the account id is
/// `AccountId32`.
pub struct EnsureAddressTruncated;
impl<OuterOrigin> EnsureAddressOrigin<OuterOrigin> for EnsureAddressTruncated
where
	OuterOrigin: Into<Result<RawOrigin<AccountId32>, OuterOrigin>> + From<RawOrigin<AccountId32>>,
{
	type Success = AccountId32;

	fn try_address_origin(address: &H160, origin: OuterOrigin) -> Result<AccountId32, OuterOrigin> {
		origin.into().and_then(|o| match o {
			RawOrigin::Signed(who) if AsRef::<[u8; 32]>::as_ref(&who)[0..20] == address[0..20] => {
				Ok(who)
			}
			r => Err(OuterOrigin::from(r)),
		})
	}
}

pub struct ConcatAddressMapping;
/// The ConcatAddressMapping used for transfer from evm 20-length to substrate 32-length address
/// The concat rule inclued three parts:
/// 1. AccountId Prefix: concat("dvm", "0x00000000000000"), length: 11 byetes
/// 2. EVM address: the original evm address, length: 20 bytes
/// 3. CheckSum:  byte_xor(AccountId Prefix + EVM address), length: 1 bytes
impl AddressMapping<AccountId32> for ConcatAddressMapping {
	fn into_account_id(address: H160) -> AccountId32 {
		let mut data = [0u8; 32];
		data[0..4].copy_from_slice(b"dvm:");
		data[11..31].copy_from_slice(&address[..]);
		let checksum: u8 = data[1..31].iter().fold(data[0], |sum, &byte| sum ^ byte);
		data[31] = checksum;
		AccountId32::from(data)
	}
}

#[cfg(feature = "std")]
#[derive(Clone, Eq, PartialEq, Encode, Decode, Debug, Serialize, Deserialize)]
/// Account definition used for genesis block construction.
pub struct GenesisAccount {
	/// Account nonce.
	pub nonce: U256,
	/// Account balance.
	pub balance: U256,
	/// Full account storage.
	pub storage: std::collections::BTreeMap<H256, H256>,
	/// Account code.
	pub code: Vec<u8>,
}
