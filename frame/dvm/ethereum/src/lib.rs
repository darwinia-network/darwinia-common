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

//! # Ethereum pallet
//!
//! The Ethereum pallet works together with EVM pallet to provide full emulation
//! for Ethereum block processing.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

pub mod adapter;

#[doc(no_inline)]
pub use ethereum::{
	AccessListItem, BlockV2 as Block, LegacyTransactionMessage, Log, ReceiptV3 as Receipt,
	TransactionAction, TransactionSignature, TransactionV2 as Transaction,
};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

// --- crates.io ---
use codec::{Decode, Encode};
use ethereum_types::{Bloom, BloomInput, H160, H256, H64, U256};
use evm::ExitReason;
// --- paritytech ---
use fp_consensus::{PostLog, PreLog, FRONTIER_ENGINE_ID};
use fp_evm::CallOrCreateInfo;
use fp_rpc::TransactionStatus;
#[cfg(feature = "std")]
use fp_storage::{EthereumStorageSchema, PALLET_ETHEREUM_SCHEMA};
#[cfg(feature = "std")]
use frame_support::storage::unhashed;
use frame_support::{
	dispatch::DispatchResultWithPostInfo,
	ensure,
	traits::{EnsureOrigin, Get},
	weights::{Pays, PostDispatchInfo, Weight},
	PalletId,
};
use frame_system::{pallet_prelude::OriginFor, WeightInfo};
use pallet_evm::FeeCalculator;
use scale_info::TypeInfo;
use sp_runtime::{
	generic::DigestItem,
	traits::{One, Saturating, UniqueSaturatedInto, Zero},
	transaction_validity::{
		InvalidTransaction, TransactionValidity, TransactionValidityError, ValidTransactionBuilder,
	},
	DispatchError, RuntimeDebug,
};
use sp_std::{marker::PhantomData, prelude::*};
// --- darwinia-network ---
use darwinia_evm::{BlockHashMapping, GasWeightMapping, Runner};
use darwinia_support::evm::{recover_signer, DeriveEthereumAddress, INTERNAL_TX_GAS_LIMIT};

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum RawOrigin {
	EthereumTransaction(H160),
}

pub fn ensure_ethereum_transaction<OuterOrigin>(o: OuterOrigin) -> Result<H160, &'static str>
where
	OuterOrigin: Into<Result<RawOrigin, OuterOrigin>>,
{
	match o.into() {
		Ok(RawOrigin::EthereumTransaction(n)) => Ok(n),
		_ => Err("bad origin: expected to be an Ethereum transaction"),
	}
}

#[derive(Clone, Debug, Encode, Decode, PartialEq, Eq)]
struct TransactionData {
	action: TransactionAction,
	input: Vec<u8>,
	nonce: U256,
	gas_limit: U256,
	gas_price: Option<U256>,
	max_fee_per_gas: Option<U256>,
	max_priority_fee_per_gas: Option<U256>,
	value: U256,
	chain_id: Option<u64>,
	access_list: Vec<(H160, Vec<H256>)>,
}

pub struct EnsureEthereumTransaction;
impl<O: Into<Result<RawOrigin, O>> + From<RawOrigin>> EnsureOrigin<O>
	for EnsureEthereumTransaction
{
	type Success = H160;

	fn try_origin(o: O) -> Result<Self::Success, O> {
		o.into().and_then(|o| match o {
			RawOrigin::EthereumTransaction(id) => Ok(id),
		})
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn successful_origin() -> O {
		O::from(RawOrigin::EthereumTransaction(Default::default()))
	}
}

impl<T: Config> Call<T>
where
	OriginFor<T>: Into<Result<RawOrigin, OriginFor<T>>>,
{
	pub fn is_self_contained(&self) -> bool {
		match self {
			Call::transact { .. } => true,
			_ => false,
		}
	}

	pub fn check_self_contained(&self) -> Option<Result<H160, TransactionValidityError>> {
		if let Call::transact { transaction } = self {
			let check = || {
				let origin = recover_signer(&transaction).ok_or_else(|| {
					InvalidTransaction::Custom(TransactionValidationError::InvalidSignature as u8)
				})?;

				Ok(origin)
			};

			Some(check())
		} else {
			None
		}
	}

	pub fn pre_dispatch_self_contained(
		&self,
		origin: &H160,
	) -> Option<Result<(), TransactionValidityError>> {
		if let Call::transact { transaction } = self {
			Some(Pallet::<T>::validate_transaction_in_block(*origin, &transaction))
		} else {
			None
		}
	}

	pub fn validate_self_contained(&self, origin: &H160) -> Option<TransactionValidity> {
		if let Call::transact { transaction } = self {
			Some(Pallet::<T>::validate_transaction_in_pool(*origin, transaction))
		} else {
			None
		}
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::config]
	pub trait Config:
		frame_system::Config + pallet_timestamp::Config + darwinia_evm::Config
	{
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// How Ethereum state root is calculated.
		type StateRoot: Get<H256>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::origin]
	pub type Origin = RawOrigin;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(n: T::BlockNumber) {
			<Pallet<T>>::store_block(
				fp_consensus::find_pre_log(&<frame_system::Pallet<T>>::digest()).is_err(),
				U256::from(UniqueSaturatedInto::<u128>::unique_saturated_into(
					frame_system::Pallet::<T>::block_number(),
				)),
			);
			// move block hash pruning window by one block
			let block_hash_count = T::BlockHashCount::get();
			let to_remove = n.saturating_sub(block_hash_count).saturating_sub(One::one());
			// keep genesis hash
			if !to_remove.is_zero() {
				<BlockHash<T>>::remove(U256::from(
					UniqueSaturatedInto::<u32>::unique_saturated_into(to_remove),
				));
			}
		}

		fn on_initialize(_block_number: T::BlockNumber) -> Weight {
			Pending::<T>::kill();
			let mut weight = T::SystemWeightInfo::kill_storage(1);

			// If the digest contain an existing ethereum block(encoded as PreLog), If contains,
			// execute the imported block firstly and disable transact dispatch function.
			if let Ok(log) = fp_consensus::find_pre_log(&<frame_system::Pallet<T>>::digest()) {
				let PreLog::Block(block) = log;

				for transaction in block.transactions {
					let source = recover_signer(&transaction).expect(
						"pre-block transaction signature invalid; the block cannot be built",
					);

					Self::validate_transaction_in_block(source, &transaction).expect(
						"pre-block transaction verification failed; the block cannot be built",
					);
					let r = Self::apply_validated_transaction(source, transaction).expect(
						"pre-block transaction execution failed; the block cannot be built",
					);
					weight = weight.saturating_add(r.actual_weight.unwrap_or(0 as Weight));
				}
			}
			// Account for `on_finalize` weight:
			// 	- read: frame_system::Pallet::<T>::digest()
			// 	- read: frame_system::Pallet::<T>::block_number()
			// 	- write: <Pallet<T>>::store_block()
			// 	- write: <BlockHash<T>>::remove()
			weight.saturating_add(T::DbWeight::get().reads_writes(2, 2))
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		OriginFor<T>: Into<Result<RawOrigin, OriginFor<T>>>,
	{
		/// This the endpoint of RPC Ethereum transaction, consistent with frontier.
		#[pallet::weight(<T as darwinia_evm::Config>::GasWeightMapping::gas_to_weight(
			Pallet::<T>::transaction_data(transaction).gas_limit.unique_saturated_into()
		))]
		pub fn transact(
			origin: OriginFor<T>,
			transaction: Transaction,
		) -> DispatchResultWithPostInfo {
			let source = ensure_ethereum_transaction(origin)?;
			// Disable transact functionality if PreLog exist.
			ensure!(
				fp_consensus::find_pre_log(&frame_system::Pallet::<T>::digest()).is_err(),
				Error::<T>::PreLogExists,
			);

			Self::apply_validated_transaction(source, transaction)
		}

		/// This is message transact only for substrate to substrate LCMP to call
		#[pallet::weight(<T as darwinia_evm::Config>::GasWeightMapping::gas_to_weight(
		Pallet::<T>::transaction_data(transaction).gas_limit.unique_saturated_into()
		))]
		pub fn message_transact(
			origin: OriginFor<T>,
			transaction: Transaction,
		) -> DispatchResultWithPostInfo {
			// Source address supposed to be derived address generate from message layer
			let source = ensure_ethereum_transaction(origin)?;

			// Disable transact functionality if PreLog exist.
			ensure!(
				fp_consensus::find_pre_log(&frame_system::Pallet::<T>::digest()).is_err(),
				Error::<T>::PreLogExists,
			);

			let extracted_transaction = match transaction {
				Transaction::Legacy(t) => Ok(Transaction::Legacy(ethereum::LegacyTransaction {
					nonce: darwinia_evm::Pallet::<T>::account_basic(&source).nonce, // auto set
					gas_price: T::FeeCalculator::min_gas_price(),                   // auto set
					gas_limit: t.gas_limit,
					action: t.action,
					value: t.value,
					input: t.input,
					signature: t.signature, // not used.
				})),
				_ => Err(Error::<T>::MessageTransactionError),
			}?;

			ensure!(
				Self::validate_transaction_in_block(source, &extracted_transaction).is_ok(),
				Error::<T>::MessageValidateError
			);

			Self::apply_validated_transaction(source, extracted_transaction)
		}

		/// Internal transaction only for root.
		#[pallet::weight(10_000_000)]
		pub fn root_transact(
			origin: OriginFor<T>,
			target: H160,
			input: Vec<u8>,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			// Disable transact functionality if PreLog exist.
			ensure!(
				fp_consensus::find_pre_log(&frame_system::Pallet::<T>::digest()).is_err(),
				Error::<T>::PreLogExists,
			);
			Self::internal_transact(target, input)
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub fn deposit_event)]
	/// Ethereum pallet events.
	pub enum Event<T: Config> {
		/// An ethereum transaction was successfully executed. \[from, to/contract_address,
		/// transaction_hash, exit_reason\]
		Executed(H160, H160, H256, ExitReason),
		/// DVM transfer. \[from, to, value\]
		DVMTransfer(T::AccountId, T::AccountId, U256),
		/// Kton transfer \[from, to, value\]
		KtonDVMTransfer(T::AccountId, T::AccountId, U256),
	}

	#[pallet::error]
	/// Ethereum pallet errors.
	pub enum Error<T> {
		/// Signature is invalid.
		InvalidSignature,
		/// Pre-log is present, therefore transact is not allowed.
		PreLogExists,
		/// The internal transaction failed.
		InternalTransactionExitError,
		InternalTransactionRevertError,
		InternalTransactionFatalError,
		/// The internal call failed.
		ReadyOnlyCall,
		/// Message transaction invalid
		MessageTransactionError,
		/// Message validate invalid
		MessageValidateError,
	}

	/// Current building block's transactions and receipts.
	#[pallet::storage]
	pub(super) type Pending<T: Config> =
		StorageValue<_, Vec<(Transaction, TransactionStatus, Receipt)>, ValueQuery>;

	/// The current Ethereum block.
	#[pallet::storage]
	pub(super) type CurrentBlock<T: Config> = StorageValue<_, ethereum::BlockV2>;

	/// The current Ethereum receipts.
	#[pallet::storage]
	pub(super) type CurrentReceipts<T: Config> = StorageValue<_, Vec<Receipt>>;

	/// The current transaction statuses.
	#[pallet::storage]
	pub(super) type CurrentTransactionStatuses<T: Config> = StorageValue<_, Vec<TransactionStatus>>;

	/// Remaining ring balance for dvm account.
	#[pallet::storage]
	#[pallet::getter(fn get_ring_remaining_balances)]
	pub(super) type RemainingRingBalance<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, u128, ValueQuery>;

	/// Remaining kton balance for dvm account.
	#[pallet::storage]
	#[pallet::getter(fn get_kton_remaining_balances)]
	pub(super) type RemainingKtonBalance<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, u128, ValueQuery>;

	/// Mapping for block number and hashes.
	#[pallet::storage]
	pub(super) type BlockHash<T: Config> = StorageMap<_, Twox64Concat, U256, H256, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig {}

	#[cfg(feature = "std")]
	impl Default for GenesisConfig {
		fn default() -> Self {
			Self {}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {
			let extra_genesis_builder: fn(&Self) = |_config: &GenesisConfig| {
				<Pallet<T>>::store_block(false, U256::zero());
				unhashed::put::<EthereumStorageSchema>(
					&PALLET_ETHEREUM_SCHEMA,
					&EthereumStorageSchema::V3,
				);
			};
			extra_genesis_builder(self);
		}
	}
}
pub use pallet::*;

impl<T: Config> Pallet<T> {
	fn transaction_data(transaction: &Transaction) -> TransactionData {
		match transaction {
			Transaction::Legacy(t) => TransactionData {
				action: t.action,
				input: t.input.clone(),
				nonce: t.nonce,
				gas_limit: t.gas_limit,
				gas_price: Some(t.gas_price),
				max_fee_per_gas: None,
				max_priority_fee_per_gas: None,
				value: t.value,
				chain_id: t.signature.chain_id(),
				access_list: Vec::new(),
			},
			Transaction::EIP2930(t) => TransactionData {
				action: t.action,
				input: t.input.clone(),
				nonce: t.nonce,
				gas_limit: t.gas_limit,
				gas_price: Some(t.gas_price),
				max_fee_per_gas: None,
				max_priority_fee_per_gas: None,
				value: t.value,
				chain_id: Some(t.chain_id),
				access_list: t.access_list.iter().map(|d| (d.address, d.slots.clone())).collect(),
			},
			Transaction::EIP1559(t) => TransactionData {
				action: t.action,
				input: t.input.clone(),
				nonce: t.nonce,
				gas_limit: t.gas_limit,
				gas_price: None,
				max_fee_per_gas: Some(t.max_fee_per_gas),
				max_priority_fee_per_gas: Some(t.max_priority_fee_per_gas),
				value: t.value,
				chain_id: Some(t.chain_id),
				access_list: t.access_list.iter().map(|d| (d.address, d.slots.clone())).collect(),
			},
		}
	}

	// Common controls to be performed in the same way by the pool and the
	// State Transition Function (STF).
	// This is the case for all controls except those concerning the nonce.
	fn validate_transaction_common(
		origin: H160,
		transaction_data: &TransactionData,
	) -> Result<(U256, u64), TransactionValidityError> {
		let gas_limit = transaction_data.gas_limit;

		// We must ensure a transaction can pay the cost of its data bytes.
		// If it can't it should not be included in a block.
		let mut gasometer = evm::gasometer::Gasometer::new(
			gas_limit.low_u64(),
			<T as darwinia_evm::Config>::config(),
		);
		let transaction_cost = match transaction_data.action {
			TransactionAction::Call(_) => evm::gasometer::call_transaction_cost(
				&transaction_data.input,
				&transaction_data.access_list,
			),
			TransactionAction::Create => evm::gasometer::create_transaction_cost(
				&transaction_data.input,
				&transaction_data.access_list,
			),
		};
		if gasometer.record_transaction(transaction_cost).is_err() {
			return Err(InvalidTransaction::Custom(
				TransactionValidationError::InvalidGasLimit as u8,
			)
			.into());
		}

		if let Some(chain_id) = transaction_data.chain_id {
			if chain_id != T::ChainId::get() {
				return Err(InvalidTransaction::Custom(
					TransactionValidationError::InvalidChainId as u8,
				)
				.into());
			}
		}

		if gas_limit >= T::BlockGasLimit::get() {
			return Err(InvalidTransaction::Custom(
				TransactionValidationError::InvalidGasLimit as u8,
			)
			.into());
		}

		let base_fee = T::FeeCalculator::min_gas_price();
		let mut priority = 0;
		let gas_price = if let Some(gas_price) = transaction_data.gas_price {
			// Legacy and EIP-2930 transactions.
			// Handle priority here. On legacy transaction everything in gas_price except
			// the current base_fee is considered a tip to the miner and thus the priority.
			priority = gas_price.saturating_sub(base_fee).unique_saturated_into();
			gas_price
		} else if let Some(max_fee_per_gas) = transaction_data.max_fee_per_gas {
			// EIP-1559 transactions.
			max_fee_per_gas
		} else {
			return Err(InvalidTransaction::Payment.into());
		};

		if gas_price < base_fee {
			return Err(InvalidTransaction::Payment.into());
		}

		let mut fee = gas_price.saturating_mul(gas_limit);
		if let Some(max_priority_fee_per_gas) = transaction_data.max_priority_fee_per_gas {
			// EIP-1559 transaction priority is determined by `max_priority_fee_per_gas`.
			// If the transaction do not include this optional parameter, priority is now considered
			// zero.
			priority = max_priority_fee_per_gas.unique_saturated_into();
			// Add the priority tip to the payable fee.
			fee = fee.saturating_add(max_priority_fee_per_gas.saturating_mul(gas_limit));
		}

		let account_data = darwinia_evm::Pallet::<T>::account_basic(&origin);
		let total_payment = transaction_data.value.saturating_add(fee);
		if account_data.balance < total_payment {
			return Err(InvalidTransaction::Payment.into());
		}
		Ok((account_data.nonce, priority))
	}

	// Controls that must be performed by the pool.
	// The controls common with the State Transition Function (STF) are in
	// the function `validate_transaction_common`.
	fn validate_transaction_in_pool(
		origin: H160,
		transaction: &Transaction,
	) -> TransactionValidity {
		let transaction_data = Pallet::<T>::transaction_data(&transaction);
		let transaction_nonce = transaction_data.nonce;

		let (account_nonce, priority) =
			Self::validate_transaction_common(origin, &transaction_data)?;
		if transaction_nonce < account_nonce {
			return Err(InvalidTransaction::Stale.into());
		}

		// The tag provides and requires must be filled correctly according to the nonce.
		let mut builder = ValidTransactionBuilder::default()
			.and_provides((origin, transaction_nonce))
			.priority(priority);

		// In the context of the pool, a transaction with
		// too high a nonce is still considered valid
		if transaction_nonce > account_nonce {
			if let Some(prev_nonce) = transaction_nonce.checked_sub(1.into()) {
				builder = builder.and_requires((origin, prev_nonce))
			}
		}

		builder.build()
	}

	/// Validate an Ethereum transaction already in block
	///
	/// This function must be called during the pre-dispatch phase
	/// (just before applying the extrinsic).
	pub fn validate_transaction_in_block(
		origin: H160,
		transaction: &Transaction,
	) -> Result<(), TransactionValidityError> {
		let transaction_data = Pallet::<T>::transaction_data(&transaction);
		let transaction_nonce = transaction_data.nonce;
		let (account_nonce, _) = Self::validate_transaction_common(origin, &transaction_data)?;

		// In the context of the block, a transaction with a nonce that is
		// too high should be considered invalid and make the whole block invalid.
		if transaction_nonce > account_nonce {
			Err(TransactionValidityError::Invalid(InvalidTransaction::Future))
		} else if transaction_nonce < account_nonce {
			Err(TransactionValidityError::Invalid(InvalidTransaction::Stale))
		} else {
			Ok(())
		}
	}

	/// Execute transaction from EthApi or PreLog Block
	/// NOTE: For the rpc transaction, the execution will return ok(..) even when encounters error
	/// 	  from evm runner
	fn apply_validated_transaction(
		source: H160,
		transaction: Transaction,
	) -> DispatchResultWithPostInfo {
		Self::raw_transact(source, transaction.into()).map(|(_, used_gas)| {
			Ok(PostDispatchInfo {
				actual_weight: Some(T::GasWeightMapping::gas_to_weight(
					used_gas.unique_saturated_into(),
				)),
				pays_fee: Pays::No,
			}
			.into())
		})?
	}

	// Execute Transaction in evm runner and save the execution info in Pending
	fn raw_transact(
		source: H160,
		advanced_transaction: AdvancedTransaction,
	) -> Result<(ExitReason, U256), DispatchError> {
		let pending = Pending::<T>::get();
		let transaction_hash = advanced_transaction.hash();
		let transaction_index = pending.len() as u32;

		let (to, _, info) = Self::execute(source, &advanced_transaction, None)?;
		let (reason, status, used_gas, dest) = match info {
			CallOrCreateInfo::Call(info) => (
				info.exit_reason,
				TransactionStatus {
					transaction_hash,
					transaction_index,
					from: source,
					to,
					contract_address: None,
					logs: info.logs.clone(),
					logs_bloom: {
						let mut bloom: Bloom = Bloom::default();
						Self::logs_bloom(info.logs, &mut bloom);
						bloom
					},
				},
				info.used_gas,
				to,
			),
			CallOrCreateInfo::Create(info) => (
				info.exit_reason,
				TransactionStatus {
					transaction_hash,
					transaction_index,
					from: source,
					to,
					contract_address: Some(info.value),
					logs: info.logs.clone(),
					logs_bloom: {
						let mut bloom: Bloom = Bloom::default();
						Self::logs_bloom(info.logs, &mut bloom);
						bloom
					},
				},
				info.used_gas,
				Some(info.value),
			),
		};

		let receipt = {
			let status_code: u8 = match reason {
				ExitReason::Succeed(_) => 1,
				_ => 0,
			};
			let logs_bloom = status.clone().logs_bloom;
			let logs = status.clone().logs;
			let cumulative_gas_used = if let Some((_, _, receipt)) = pending.last() {
				match receipt {
					Receipt::Legacy(d) | Receipt::EIP2930(d) | Receipt::EIP1559(d) =>
						d.used_gas.saturating_add(used_gas),
				}
			} else {
				used_gas
			};

			match advanced_transaction {
				AdvancedTransaction::Ethereum(ref transaction) => match &transaction {
					Transaction::Legacy(_) => Receipt::Legacy(ethereum::EIP658ReceiptData {
						status_code,
						used_gas: cumulative_gas_used,
						logs_bloom,
						logs,
					}),
					Transaction::EIP2930(_) => Receipt::EIP2930(ethereum::EIP2930ReceiptData {
						status_code,
						used_gas: cumulative_gas_used,
						logs_bloom,
						logs,
					}),
					Transaction::EIP1559(_) => Receipt::EIP1559(ethereum::EIP2930ReceiptData {
						status_code,
						used_gas: cumulative_gas_used,
						logs_bloom,
						logs,
					}),
				},
				AdvancedTransaction::Internal(_) => Receipt::Legacy(ethereum::EIP658ReceiptData {
					status_code,
					used_gas: cumulative_gas_used,
					logs_bloom,
					logs,
				}),
			}
		};
		Pending::<T>::append((advanced_transaction.transaction(), status, receipt));
		Self::deposit_event(Event::Executed(
			source,
			dest.unwrap_or_default(),
			transaction_hash,
			reason.clone(),
		));
		Ok((reason, used_gas))
	}

	/// Get the transaction status with given index.
	pub fn current_transaction_statuses() -> Option<Vec<TransactionStatus>> {
		CurrentTransactionStatuses::<T>::get()
	}

	/// Get current block.
	pub fn current_block() -> Option<ethereum::BlockV2> {
		CurrentBlock::<T>::get()
	}

	/// Get current block hash
	pub fn current_block_hash() -> Option<H256> {
		Self::current_block().map(|block| block.header.hash())
	}

	/// Get receipts by number.
	pub fn current_receipts() -> Option<Vec<Receipt>> {
		CurrentReceipts::<T>::get()
	}

	/// Execute an Ethereum transaction
	pub fn execute(
		from: H160,
		advanced_transaction: &AdvancedTransaction,
		config: Option<evm::Config>,
	) -> Result<(Option<H160>, Option<H160>, CallOrCreateInfo), DispatchError> {
		let (
			input,
			value,
			gas_limit,
			max_fee_per_gas,
			max_priority_fee_per_gas,
			nonce,
			action,
			access_list,
		) = match advanced_transaction {
			AdvancedTransaction::Ethereum(transaction) => {
				match transaction {
					// max_fee_per_gas and max_priority_fee_per_gas in legacy and 2930 transactions
					// is the provided gas_price.
					Transaction::Legacy(t) => {
						let base_fee = T::FeeCalculator::min_gas_price();
						let priority_fee = t
							.gas_price
							.checked_sub(base_fee)
							.ok_or_else(|| DispatchError::Other("Gas price too low"))?;
						(
							t.input.clone(),
							t.value,
							t.gas_limit,
							Some(base_fee),
							Some(priority_fee),
							Some(t.nonce),
							t.action,
							Vec::new(),
						)
					},
					Transaction::EIP2930(t) => {
						let base_fee = T::FeeCalculator::min_gas_price();
						let priority_fee = t
							.gas_price
							.checked_sub(base_fee)
							.ok_or_else(|| DispatchError::Other("Gas price too low"))?;
						let access_list: Vec<(H160, Vec<H256>)> = t
							.access_list
							.iter()
							.map(|item| (item.address, item.slots.clone()))
							.collect();
						(
							t.input.clone(),
							t.value,
							t.gas_limit,
							Some(base_fee),
							Some(priority_fee),
							Some(t.nonce),
							t.action,
							access_list,
						)
					},
					Transaction::EIP1559(t) => {
						let access_list: Vec<(H160, Vec<H256>)> = t
							.access_list
							.iter()
							.map(|item| (item.address, item.slots.clone()))
							.collect();
						(
							t.input.clone(),
							t.value,
							t.gas_limit,
							Some(t.max_fee_per_gas),
							Some(t.max_priority_fee_per_gas),
							Some(t.nonce),
							t.action,
							access_list,
						)
					},
				}
			},
			AdvancedTransaction::Internal(t) => (
				t.input.clone(),
				t.value,
				t.gas_limit,
				None,
				None,
				Some(t.nonce),
				t.action,
				Vec::new(),
			),
		};

		match action {
			ethereum::TransactionAction::Call(target) => {
				let res = T::Runner::call(
					from,
					target,
					input,
					value,
					gas_limit.low_u64(),
					max_fee_per_gas,
					max_priority_fee_per_gas,
					nonce,
					access_list,
					config.as_ref().unwrap_or(T::config()),
				)
				.map_err(Into::into)?;

				Ok((Some(target), None, CallOrCreateInfo::Call(res)))
			},
			ethereum::TransactionAction::Create => {
				let res = T::Runner::create(
					from,
					input,
					value,
					gas_limit.low_u64(),
					max_fee_per_gas,
					max_priority_fee_per_gas,
					nonce,
					access_list,
					config.as_ref().unwrap_or(T::config()),
				)
				.map_err(Into::into)?;

				Ok((None, Some(res.value), CallOrCreateInfo::Create(res)))
			},
		}
	}

	/// Save ethereum block
	fn store_block(post_log: bool, block_number: U256) {
		let mut transactions = Vec::new();
		let mut statuses = Vec::new();
		let mut receipts = Vec::new();
		let mut logs_bloom = Bloom::default();
		let mut cumulative_gas_used = U256::zero();
		for (transaction, status, receipt) in Pending::<T>::get() {
			transactions.push(transaction);
			statuses.push(status);
			receipts.push(receipt.clone());
			let (logs, used_gas) = match receipt {
				Receipt::Legacy(d) | Receipt::EIP2930(d) | Receipt::EIP1559(d) =>
					(d.logs.clone(), d.used_gas),
			};
			cumulative_gas_used = used_gas;
			Self::logs_bloom(logs, &mut logs_bloom);
		}

		let ommers = Vec::<ethereum::Header>::new();
		let receipts_root =
			ethereum::util::ordered_trie_root(receipts.iter().map(|r| rlp::encode(r)));
		let partial_header = ethereum::PartialHeader {
			// Instead of using current_block(), obtain the parent block hash from BlockHash storage
			// to avoid Block type upgrade failures See: https://github.com/paritytech/frontier/pull/570
			parent_hash: if block_number > U256::zero() {
				BlockHash::<T>::get(block_number - 1)
			} else {
				H256::default()
			},
			beneficiary: darwinia_evm::Pallet::<T>::find_author(),
			state_root: T::StateRoot::get(),
			receipts_root,
			logs_bloom,
			difficulty: U256::zero(),
			number: block_number,
			gas_limit: T::BlockGasLimit::get(),
			gas_used: cumulative_gas_used,
			timestamp: UniqueSaturatedInto::<u64>::unique_saturated_into(
				<pallet_timestamp::Pallet<T>>::get(),
			),
			extra_data: Vec::new(),
			mix_hash: H256::default(),
			nonce: H64::default(),
		};
		let block = ethereum::Block::new(partial_header, transactions.clone(), ommers);

		CurrentBlock::<T>::put(block.clone());
		CurrentReceipts::<T>::put(receipts);
		CurrentTransactionStatuses::<T>::put(statuses);
		BlockHash::<T>::insert(block_number, block.header.hash());

		if post_log {
			let digest = DigestItem::<T::Hash>::Consensus(
				FRONTIER_ENGINE_ID,
				PostLog::Hashes(fp_consensus::Hashes::from_block(block)).encode(),
			);
			<frame_system::Pallet<T>>::deposit_log(digest);
		}
	}

	fn logs_bloom(logs: Vec<Log>, bloom: &mut Bloom) {
		for log in logs {
			bloom.accrue(BloomInput::Raw(&log.address[..]));
			for topic in log.topics {
				bloom.accrue(BloomInput::Raw(&topic[..]));
			}
		}
	}
}

/// The handler for interacting with The internal transaction.
pub trait InternalTransactHandler {
	/// Internal transaction call.
	fn internal_transact(target: H160, input: Vec<u8>) -> DispatchResultWithPostInfo;
	/// Read-only call to deployed evm contracts.
	fn read_only_call(contract: H160, input: Vec<u8>) -> Result<Vec<u8>, DispatchError>;
}

impl<T: Config> InternalTransactHandler for Pallet<T> {
	/// Execute transaction from other pallets(internal transaction)
	/// NOTE: The difference between the rpc transaction and the internal transaction is that
	/// The internal transactions will catch and throw evm error comes from runner to caller.
	fn internal_transact(target: H160, input: Vec<u8>) -> DispatchResultWithPostInfo {
		let source = T::PalletId::get().derive_ethereum_address();
		let nonce = darwinia_evm::Pallet::<T>::account_basic(&source).nonce;
		let transaction = internal_transaction::<T>(nonce, target, input);

		Self::raw_transact(source, transaction).map(|(reason, used_gas)| match reason {
			// Only when exit_reason is successful, return Ok(...)
			ExitReason::Succeed(_) => Ok(PostDispatchInfo {
				actual_weight: Some(T::GasWeightMapping::gas_to_weight(
					used_gas.unique_saturated_into(),
				)),
				pays_fee: Pays::No,
			}),
			ExitReason::Error(_) => Err(<Error<T>>::InternalTransactionExitError.into()),
			ExitReason::Revert(_) => Err(<Error<T>>::InternalTransactionRevertError.into()),
			ExitReason::Fatal(_) => Err(<Error<T>>::InternalTransactionFatalError.into()),
		})?
	}

	/// Pure read-only call to contract, the sender is pallet dvm account.
	/// NOTE: You should never use raw call for any non-read-only operation, be carefully.
	fn read_only_call(contract: H160, input: Vec<u8>) -> Result<Vec<u8>, DispatchError> {
		sp_io::storage::start_transaction();
		let source = T::PalletId::get().derive_ethereum_address();
		let nonce = darwinia_evm::Pallet::<T>::account_basic(&source).nonce;
		let transaction = internal_transaction::<T>(nonce, contract, input);

		let (_, _, info) = Self::execute(source, &transaction, None)?;
		sp_io::storage::rollback_transaction();
		match info {
			CallOrCreateInfo::Call(info) => match info.exit_reason {
				ExitReason::Succeed(_) => Ok(info.value),
				ExitReason::Error(_) => Err(<Error<T>>::InternalTransactionExitError.into()),
				ExitReason::Revert(_) => Err(<Error<T>>::InternalTransactionRevertError.into()),
				ExitReason::Fatal(_) => Err(<Error<T>>::InternalTransactionFatalError.into()),
			},
			_ => Err(<Error<T>>::ReadyOnlyCall.into()),
		}
	}
}

#[repr(u8)]
enum TransactionValidationError {
	#[allow(dead_code)]
	UnknownError,
	InvalidChainId,
	InvalidSignature,
	InvalidGasLimit,
}
/// Returns the Ethereum block hash by number.
pub struct EthereumBlockHashMapping<T>(PhantomData<T>);
impl<T: Config> BlockHashMapping for EthereumBlockHashMapping<T> {
	fn block_hash(number: u32) -> H256 {
		BlockHash::<T>::get(U256::from(number))
	}
}

/// Returned the Ethereum block state root.
pub struct IntermediateStateRoot;
impl Get<H256> for IntermediateStateRoot {
	fn get() -> H256 {
		H256::decode(&mut &sp_io::storage::root()[..])
			.expect("Node is configured to use the same hash; qed")
	}
}

#[doc(hidden)]
pub mod migration {
	#[cfg(feature = "try-runtime")]
	pub mod try_runtime {
		pub fn pre_migrate() -> Result<(), &'static str> {
			Ok(())
		}
	}

	pub fn migrate() {}
}

pub enum AdvancedTransaction {
	Ethereum(Transaction),
	// The internal transaction is an special LegacyTransaction
	Internal(ethereum::TransactionV0),
}

impl AdvancedTransaction {
	fn hash(&self) -> H256 {
		match self {
			Self::Ethereum(t) => t.hash(),
			Self::Internal(t) => t.hash(),
		}
	}

	fn transaction(&self) -> Transaction {
		match self {
			Self::Ethereum(t) => t.clone(),
			Self::Internal(t) => Transaction::Legacy(t.clone()),
		}
	}
}

impl From<Transaction> for AdvancedTransaction {
	fn from(t: Transaction) -> Self {
		Self::Ethereum(t)
	}
}

pub fn internal_transaction<T: Config>(
	nonce: U256,
	target: H160,
	input: Vec<u8>,
) -> AdvancedTransaction {
	let transaction = ethereum::TransactionV0 {
		nonce,
		// Not used, and will be overwritten by None later.
		gas_price: U256::zero(),
		gas_limit: U256::from(INTERNAL_TX_GAS_LIMIT),
		action: ethereum::TransactionAction::Call(target),
		value: U256::zero(),
		input,
		signature: ethereum::TransactionSignature::new(
			// Reference https://github.com/ethereum/EIPs/issues/155
			//
			// The internal transaction is sent by specific pallets, no signature
			// validation, just create a valid transaction signature is enough.
			T::ChainId::get() * 2 + 36,
			H256::from_slice(&[55u8; 32]),
			H256::from_slice(&[55u8; 32]),
		)
		.unwrap(),
	};

	AdvancedTransaction::Internal(transaction)
}
