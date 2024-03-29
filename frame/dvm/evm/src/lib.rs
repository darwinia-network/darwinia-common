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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

//! EVM execution pallet for Substrate

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::all)]

pub mod runner;

#[cfg(any(test, feature = "runtime-benchmarks"))]
mod benchmarking;
#[cfg(test)]
mod tests;

pub use crate::runner::Runner;
#[doc(no_inline)]
pub use fp_evm::{
	Account, CallInfo, CreateInfo, ExecutionInfo, Log, Precompile, PrecompileFailure,
	PrecompileOutput, PrecompileResult, PrecompileSet, Vicinity,
};

// --- std ---
#[cfg(feature = "std")]
use std::collections::BTreeMap;
// --- crates.io ---
use evm::{Config as EvmConfig, ExitError, ExitReason};
// --- paritytech ---
use fp_evm::FeeCalculator;
#[cfg(feature = "std")]
use fp_evm::GenesisAccount;
use frame_support::{
	traits::{FindAuthor, WithdrawReasons},
	weights::{PostDispatchInfo, Weight},
};
use frame_system::RawOrigin;
use sp_core::{H160, H256, U256};
use sp_runtime::{
	traits::{BadOrigin, UniqueSaturatedInto},
	SaturatedConversion,
};
use sp_std::{marker::PhantomData, prelude::*};
// --- darwinia-network ---
use darwinia_support::evm::DeriveSubstrateAddress;

pub type AccountId<T> = <T as frame_system::Config>::AccountId;

static LONDON_CONFIG: EvmConfig = EvmConfig::london();

#[frame_support::pallet]
pub mod pallet {
	// --- paritytech ---
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	// --- darwinia-network ---
	use crate::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_timestamp::Config {
		/// Calculator for current gas price.
		type FeeCalculator: FeeCalculator;
		/// Maps Ethereum gas to Substrate weight.
		type GasWeightMapping: GasWeightMapping;
		/// The block gas limit. Can be a simple constant, or an adjustment algorithm in another
		/// pallet.
		type BlockGasLimit: Get<U256>;

		/// Allow the origin to call on behalf of given address.
		type CallOrigin: EnsureAddressOrigin<Self::Origin>;
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// Chain ID of EVM.
		type ChainId: Get<u64>;

		/// Convert from H160 to account id.
		type IntoAccountId: DeriveSubstrateAddress<Self::AccountId>;
		/// Block number to block hash.
		type BlockHashMapping: BlockHashMapping;
		/// Find author for the current block.
		type FindAuthor: FindAuthor<H160>;

		/// *RING* balance adapter for decimal convert
		type RingBalanceAdapter: CurrencyAdapt<Self>;
		/// *KTON* balance adapter for decimal convert
		type KtonBalanceAdapter: CurrencyAdapt<Self>;

		/// Precompiles associated with this EVM engine.
		type PrecompilesType: PrecompileSet;
		type PrecompilesValue: Get<Self::PrecompilesType>;
		/// EVM execution runner.
		type Runner: Runner<Self>;

		/// To handle fee deduction for EVM transactions. An example is this pallet being used by
		/// `pallet_ethereum` where the chain implementing `pallet_ethereum` should be able to
		/// configure what happens to the fees Similar to `OnChargeTransaction` of
		/// `pallet_transaction_payment`
		type OnChargeTransaction: OnChargeEVMTransaction<Self>;

		/// EVM config used in the Pallet.
		fn config() -> &'static EvmConfig {
			&LONDON_CONFIG
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Ethereum events from contracts.
		Log { log: Log },
		/// A contract has been created at given.
		Created { address: H160 },
		/// A contract was attempted to be created, but the execution failed.
		CreatedFailed { address: H160 },
		/// A contract has been executed successfully with states applied.
		Executed { address: H160 },
		/// A contract has been executed with errors. States are reverted with only gas fees
		/// applied.
		ExecutedFailed { address: H160 },
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
	pub(super) type AccountCodes<T: Config> =
		StorageMap<_, Blake2_128Concat, H160, Vec<u8>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn account_storages)]
	pub(super) type AccountStorages<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, H160, Blake2_128Concat, H256, H256, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig {
		pub accounts: BTreeMap<H160, GenesisAccount>,
	}
	#[cfg(feature = "std")]
	impl Default for GenesisConfig {
		fn default() -> Self {
			Self { accounts: Default::default() }
		}
	}
	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {
			for (address, account) in &self.accounts {
				let account_id = T::IntoAccountId::derive_substrate_address(address);

				// ASSUME: in one single EVM transaction, the nonce will not increase more than
				// `u128::max_value()`.
				for _ in 0..account.nonce.low_u128() {
					frame_system::Pallet::<T>::inc_account_nonce(&account_id);
				}

				T::RingBalanceAdapter::mutate_evm_balance(&address, account.balance);
				Pallet::<T>::create_account(address, account.code.clone());
				for (index, value) in &account.storage {
					AccountStorages::<T>::insert(address, index, value);
				}
			}
		}
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(PhantomData<T>);
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Issue an EVM call operation. This is similar to a message call transaction in Ethereum.
		#[pallet::weight(T::GasWeightMapping::gas_to_weight(*gas_limit))]
		pub fn call(
			origin: OriginFor<T>,
			source: H160,
			target: H160,
			input: Vec<u8>,
			value: U256,
			gas_limit: u64,
			max_fee_per_gas: U256,
			max_priority_fee_per_gas: Option<U256>,
			nonce: Option<U256>,
			access_list: Vec<(H160, Vec<H256>)>,
		) -> DispatchResultWithPostInfo {
			T::CallOrigin::ensure_address_origin(&source, origin)?;

			let is_transactional = true;
			let info = T::Runner::call(
				source,
				target,
				input,
				value,
				gas_limit,
				Some(max_fee_per_gas),
				max_priority_fee_per_gas,
				nonce,
				access_list,
				is_transactional,
				T::config(),
			)?;

			match info.exit_reason {
				ExitReason::Succeed(_) => {
					Pallet::<T>::deposit_event(Event::<T>::Executed { address: target });
				},
				_ => {
					Pallet::<T>::deposit_event(Event::<T>::ExecutedFailed { address: target });
				},
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
		pub fn create(
			origin: OriginFor<T>,
			source: H160,
			init: Vec<u8>,
			value: U256,
			gas_limit: u64,
			max_fee_per_gas: U256,
			max_priority_fee_per_gas: Option<U256>,
			nonce: Option<U256>,
			access_list: Vec<(H160, Vec<H256>)>,
		) -> DispatchResultWithPostInfo {
			T::CallOrigin::ensure_address_origin(&source, origin)?;

			let is_transactional = true;
			let info = T::Runner::create(
				source,
				init,
				value,
				gas_limit,
				Some(max_fee_per_gas),
				max_priority_fee_per_gas,
				nonce,
				access_list,
				is_transactional,
				T::config(),
			)?;
			match info {
				CreateInfo {
					exit_reason: ExitReason::Succeed(_), value: create_address, ..
				} => {
					Pallet::<T>::deposit_event(Event::<T>::Created { address: create_address });
				},
				CreateInfo { exit_reason: _, value: create_address, .. } => {
					Pallet::<T>::deposit_event(Event::<T>::CreatedFailed {
						address: create_address,
					});
				},
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
		pub fn create2(
			origin: OriginFor<T>,
			source: H160,
			init: Vec<u8>,
			salt: H256,
			value: U256,
			gas_limit: u64,
			max_fee_per_gas: U256,
			max_priority_fee_per_gas: Option<U256>,
			nonce: Option<U256>,
			access_list: Vec<(H160, Vec<H256>)>,
		) -> DispatchResultWithPostInfo {
			T::CallOrigin::ensure_address_origin(&source, origin)?;

			let is_transactional = true;
			let info = T::Runner::create2(
				source,
				init,
				salt,
				value,
				gas_limit,
				Some(max_fee_per_gas),
				max_priority_fee_per_gas,
				nonce,
				access_list,
				is_transactional,
				T::config(),
			)?;
			match info {
				CreateInfo {
					exit_reason: ExitReason::Succeed(_), value: create_address, ..
				} => {
					Pallet::<T>::deposit_event(Event::<T>::Created { address: create_address });
				},
				CreateInfo { exit_reason: _, value: create_address, .. } => {
					Pallet::<T>::deposit_event(Event::<T>::CreatedFailed {
						address: create_address,
					});
				},
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
		pub fn account_basic(address: &H160) -> Account {
			let account_id = T::IntoAccountId::derive_substrate_address(address);
			let nonce = <frame_system::Pallet<T>>::account_nonce(&account_id);
			Account {
				nonce: nonce.saturated_into::<u128>().into(),
				balance: T::RingBalanceAdapter::account_balance(&account_id),
			}
		}

		pub fn remove_account(address: &H160) {
			if AccountCodes::<T>::contains_key(address) {
				let account_id = T::IntoAccountId::derive_substrate_address(address);
				let _ = frame_system::Pallet::<T>::dec_sufficients(&account_id);
			}

			AccountCodes::<T>::remove(address);
			AccountStorages::<T>::remove_prefix(address, None);
		}

		/// Create an account.
		pub fn create_account(address: &H160, code: Vec<u8>) {
			if code.is_empty() {
				return;
			}

			if !AccountCodes::<T>::contains_key(&address) {
				let account_id = T::IntoAccountId::derive_substrate_address(address);
				let _ = frame_system::Pallet::<T>::inc_sufficients(&account_id);
			}

			AccountCodes::<T>::insert(address, code);
		}

		/// Check whether an account is empty.
		pub fn is_account_empty(address: &H160) -> bool {
			let account = Self::account_basic(address);
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

		/// Get the author using the FindAuthor trait.
		pub fn find_author() -> H160 {
			let digest = <frame_system::Pallet<T>>::digest();
			let pre_runtime_digests = digest.logs.iter().filter_map(|d| d.as_pre_runtime());

			T::FindAuthor::find_author(pre_runtime_digests).unwrap_or_default()
		}
	}
}
pub use pallet::*;

/// Handle withdrawing, refunding and depositing of transaction fees.
/// Similar to `OnChargeTransaction` of `pallet_transaction_payment`
pub trait OnChargeEVMTransaction<T: Config> {
	type LiquidityInfo: Default;

	/// Before the transaction is executed the payment of the transaction fees
	/// need to be secured.
	fn withdraw_fee(who: &H160, fee: U256) -> Result<Self::LiquidityInfo, Error<T>>;

	/// After the transaction was executed the actual fee can be calculated.
	/// This function should refund any overpaid fees and optionally deposit
	/// the corrected amount.
	fn correct_and_deposit_fee(
		who: &H160,
		corrected_fee: U256,
		already_withdrawn: Self::LiquidityInfo,
	);

	/// Introduced in EIP1559 to handle the priority tip payment to the block Author.
	fn pay_priority_fee(tip: U256);
}

pub struct EVMCurrencyAdapter<F>(PhantomData<F>);
impl<T, F> OnChargeEVMTransaction<T> for EVMCurrencyAdapter<F>
where
	T: Config,
	F: FindAuthor<T::AccountId>,
{
	type LiquidityInfo = U256;

	fn withdraw_fee(who: &H160, fee: U256) -> Result<Self::LiquidityInfo, Error<T>> {
		let balance = T::RingBalanceAdapter::evm_balance(who);
		let new_account_balance = balance.saturating_sub(fee);
		T::RingBalanceAdapter::mutate_evm_balance(who, new_account_balance);
		Ok(fee)
	}

	fn correct_and_deposit_fee(
		who: &H160,
		corrected_fee: U256,
		already_withdrawn: Self::LiquidityInfo,
	) {
		let balance = T::RingBalanceAdapter::evm_balance(who);
		let refund = already_withdrawn.saturating_sub(corrected_fee);
		let new_account_balance = balance.saturating_add(refund);
		T::RingBalanceAdapter::mutate_evm_balance(who, new_account_balance);
	}

	fn pay_priority_fee(tip: U256) {
		let digest = <frame_system::Pallet<T>>::digest();
		let pre_runtime_digests = digest.logs.iter().filter_map(|d| d.as_pre_runtime());
		if let Some(author) = F::find_author(pre_runtime_digests) {
			let account_balance = T::RingBalanceAdapter::account_balance(&author);
			let new_account_balance = account_balance.saturating_add(tip);
			T::RingBalanceAdapter::mutate_account_balance(&author, new_account_balance);
		}
	}
}

/// A trait to perform origin check.
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

/// A trait for handling currency decimal difference between native and evm tokens.
pub trait CurrencyAdapt<T: Config> {
	/// Get account balance, the decimal of the returned result is consistent with Ethereum.
	fn account_balance(account_id: &T::AccountId) -> U256;

	/// Get the total supply of token in Ethereum decimal.
	fn evm_total_supply() -> U256;

	/// Mutate account balance, the new_balance's decimal should be the same as Ethereum.
	fn mutate_account_balance(account_id: &T::AccountId, balance: U256);

	/// Ensure that an account can withdraw from their fee balance.
	fn ensure_can_withdraw(
		who: &T::AccountId,
		amount: U256,
		reasons: WithdrawReasons,
	) -> Result<(), ExitError>;

	/// Transfer value. the value's decimal should be the same as Ethereum.
	fn evm_transfer(
		source: &T::AccountId,
		target: &T::AccountId,
		value: U256,
	) -> Result<(), ExitError>;

	/// Get the account balance by ethereum address, the decimal of the returned result is
	/// consistent with Ethereum.
	fn evm_balance(address: &H160) -> U256 {
		let account_id = <T as Config>::IntoAccountId::derive_substrate_address(address);
		Self::account_balance(&account_id)
	}

	fn mutate_evm_balance(address: &H160, new_balance: U256) {
		let account_id = <T as Config>::IntoAccountId::derive_substrate_address(address);
		Self::mutate_account_balance(&account_id, new_balance)
	}
}

/// A mapping function that converts Ethereum gas to Substrate weight.
pub trait GasWeightMapping {
	fn gas_to_weight(gas: u64) -> Weight;
	fn weight_to_gas(weight: Weight) -> u64;
}
// The radio of gas to weight comes from benchmark test.
impl GasWeightMapping for () {
	fn gas_to_weight(gas: u64) -> Weight {
		gas * 16_000 as Weight
	}

	fn weight_to_gas(weight: Weight) -> u64 {
		weight / 16_000
	}
}

/// A trait for getting a block hash by number.
pub trait BlockHashMapping {
	fn block_hash(number: u32) -> H256;
}

/// Returns the Substrate block hash by number.
pub struct SubstrateBlockHashMapping<T>(sp_std::marker::PhantomData<T>);
impl<T: Config> BlockHashMapping for SubstrateBlockHashMapping<T> {
	fn block_hash(number: u32) -> H256 {
		let number = T::BlockNumber::from(number);
		H256::from_slice(frame_system::Pallet::<T>::block_hash(number).as_ref())
	}
}

/// Ensure that the address is truncated hash of the origin.
pub struct EnsureAddressTruncated<AccountId>(PhantomData<AccountId>);
impl<AccountId, OuterOrigin> EnsureAddressOrigin<OuterOrigin> for EnsureAddressTruncated<AccountId>
where
	AccountId: AsRef<[u8; 32]>,
	OuterOrigin: Into<Result<RawOrigin<AccountId>, OuterOrigin>> + From<RawOrigin<AccountId>>,
{
	type Success = AccountId;

	fn try_address_origin(address: &H160, origin: OuterOrigin) -> Result<AccountId, OuterOrigin> {
		origin.into().and_then(|o| match o {
			RawOrigin::Signed(who) if who.as_ref()[0..20] == address[0..20] => Ok(who),
			r => Err(OuterOrigin::from(r)),
		})
	}
}
