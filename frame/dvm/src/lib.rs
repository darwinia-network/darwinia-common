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

pub mod account_basic;

use dvm_rpc_runtime_api::TransactionStatus;
#[doc(no_inline)]
pub use ethereum::{
	BlockV2 as Block, LegacyTransactionMessage, Log, Receipt as EthereumReceiptV0,
	TransactionAction, TransactionSignature, TransactionV2 as Transaction,
};

#[cfg(all(feature = "std", test))]
mod mock;
#[cfg(all(feature = "std", test))]
mod tests;

// --- crates.io ---
use codec::{Decode, Encode};
use ethereum_types::{Bloom, BloomInput, H160, H256, H64, U256};
use evm::ExitReason;
use sha3::{Digest, Keccak256};
// --- paritytech ---
use fp_evm::CallOrCreateInfo;
#[cfg(feature = "std")]
use fp_storage::PALLET_ETHEREUM_SCHEMA;
#[cfg(feature = "std")]
use frame_support::storage::unhashed;
use frame_support::{
	dispatch::DispatchResultWithPostInfo,
	ensure,
	traits::{Currency, EnsureOrigin, Get},
	weights::{Pays, PostDispatchInfo, Weight},
	PalletId,
};
use frame_system::pallet_prelude::OriginFor;
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
use darwinia_evm::{AccountBasic, BlockHashMapping, GasWeightMapping, Runner};
use darwinia_support::evm::{
	new_internal_transaction, recover_signer, DVMTransaction, IntoH160, INTERNAL_TX_GAS_LIMIT,
};
use dp_consensus::{PostLog, PreLog, FRONTIER_ENGINE_ID};

/// A type alias for the balance type from this pallet's point of view.
type AccountId<T> = <T as frame_system::Config>::AccountId;
type RingCurrency<T> = <T as Config>::RingCurrency;
type KtonCurrency<T> = <T as Config>::KtonCurrency;
type RingBalance<T> = <RingCurrency<T> as Currency<AccountId<T>>>::Balance;
type KtonBalance<T> = <KtonCurrency<T> as Currency<AccountId<T>>>::Balance;

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
			Some(Pallet::<T>::validate_transaction_in_block(
				*origin,
				&transaction,
			))
		} else {
			None
		}
	}

	pub fn validate_self_contained(&self, origin: &H160) -> Option<TransactionValidity> {
		if let Call::transact { transaction } = self {
			Some(Pallet::<T>::validate_transaction_in_pool(
				*origin,
				transaction,
			))
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
		type Event: From<Event> + IsType<<Self as frame_system::Config>::Event>;
		/// How Ethereum state root is calculated.
		type StateRoot: Get<H256>;
		/// *RING* balances module.
		type RingCurrency: Currency<Self::AccountId>;
		/// *KTON* balances module.
		type KtonCurrency: Currency<Self::AccountId>;
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
				dp_consensus::find_pre_log(&<frame_system::Pallet<T>>::digest()).is_err(),
				U256::from(UniqueSaturatedInto::<u128>::unique_saturated_into(
					frame_system::Pallet::<T>::block_number(),
				)),
			);
			// move block hash pruning window by one block
			let block_hash_count = T::BlockHashCount::get();
			let to_remove = n
				.saturating_sub(block_hash_count)
				.saturating_sub(One::one());
			// keep genesis hash
			if !to_remove.is_zero() {
				<BlockHash<T>>::remove(U256::from(
					UniqueSaturatedInto::<u32>::unique_saturated_into(to_remove),
				));
			}
		}

		fn on_initialize(_block_number: T::BlockNumber) -> Weight {
			Pending::<T>::kill();

			// If the digest contain an existing ethereum block(encoded as PreLog), If contains,
			// execute the imported block firstly and disable transact dispatch function.
			if let Ok(log) = dp_consensus::find_pre_log(&<frame_system::Pallet<T>>::digest()) {
				let PreLog::Block(block) = log;

				for transaction in block.transactions {
					let source = recover_signer(&transaction).expect(
						"pre-block transaction signature invalid; the block cannot be built",
					);

					Self::validate_transaction_in_block(source, &transaction).expect(
						"pre-block transaction verification failed; the block cannot be built",
					);
					Self::apply_validated_transaction(source, transaction).expect(
						"pre-block transaction execution failed; the block cannot be built",
					);
				}
			}
			0
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		OriginFor<T>: Into<Result<RawOrigin, OriginFor<T>>>,
	{
		/// Transact an Ethereum transaction.
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
				dp_consensus::find_pre_log(&frame_system::Pallet::<T>::digest()).is_err(),
				Error::<T>::PreLogExists,
			);

			Self::apply_validated_transaction(source, transaction)
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
				dp_consensus::find_pre_log(&frame_system::Pallet::<T>::digest()).is_err(),
				Error::<T>::PreLogExists,
			);

			Ok(().into())

			// Self::internal_transact(target, input)
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	/// Ethereum pallet events.
	pub enum Event {
		/// An ethereum transaction was successfully executed. [from, to/contract_address, transaction_hash, exit_reason]
		Executed(H160, H160, H256, ExitReason),
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
	}

	/// Current building block's transactions and receipts.
	#[pallet::storage]
	pub(super) type Pending<T: Config> =
		StorageValue<_, Vec<(Transaction, TransactionStatus, EthereumReceiptV0)>, ValueQuery>;

	/// The current Ethereum block.
	#[pallet::storage]
	pub(super) type CurrentBlock<T: Config> = StorageValue<_, ethereum::BlockV2>;

	/// The current Ethereum receipts.
	#[pallet::storage]
	pub(super) type CurrentReceipts<T: Config> = StorageValue<_, Vec<EthereumReceiptV0>>;

	/// The current transaction statuses.
	#[pallet::storage]
	pub(super) type CurrentTransactionStatuses<T: Config> = StorageValue<_, Vec<TransactionStatus>>;

	/// Remaining ring balance for dvm account.
	#[pallet::storage]
	#[pallet::getter(fn get_ring_remaining_balances)]
	pub(super) type RemainingRingBalance<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, RingBalance<T>, ValueQuery>;

	/// Remaining kton balance for dvm account.
	#[pallet::storage]
	#[pallet::getter(fn get_kton_remaining_balances)]
	pub(super) type RemainingKtonBalance<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, KtonBalance<T>, ValueQuery>;

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
					&EthereumStorageSchema::V2,
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
				access_list: t
					.access_list
					.iter()
					.map(|d| (d.address, d.slots.clone()))
					.collect(),
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
				access_list: t
					.access_list
					.iter()
					.map(|d| (d.address, d.slots.clone()))
					.collect(),
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
			// If the transaction do not include this optional parameter, priority is now considered zero.
			priority = max_priority_fee_per_gas.unique_saturated_into();
			// Add the priority tip to the payable fee.
			fee = fee.saturating_add(max_priority_fee_per_gas.saturating_mul(gas_limit));
		}

		let account_data = <T as darwinia_evm::Config>::RingAccountBasic::account_basic(&origin);
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
			Err(TransactionValidityError::Invalid(
				InvalidTransaction::Future,
			))
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
		dvm_transaction: Transaction,
	) -> Result<(ExitReason, U256), DispatchError> {
		// TODO: use hash() directly
		let transaction_hash =
			H256::from_slice(Keccak256::digest(&rlp::encode(&dvm_transaction)).as_slice());
		let transaction_index = Pending::<T>::get().len() as u32;

		// let (to, _, info) = Self::execute(
		// 	source,
		// 	dvm_transaction.tx.input.clone(),
		// 	dvm_transaction.tx.value,
		// 	dvm_transaction.tx.gas_limit,
		// 	dvm_transaction.gas_price,
		// 	Some(dvm_transaction.tx.nonce),
		// 	dvm_transaction.tx.action,
		// 	None,
		// )?;
		// TODO: Rename the param field later
		let (to, _, info) = Self::execute(source, &dvm_transaction, None)
			.expect("transaction is already validated; error indicates that the block is invalid");

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
		let receipt = ethereum::Receipt {
			state_root: match reason {
				ExitReason::Succeed(_) => H256::from_low_u64_be(1),
				ExitReason::Error(_) => H256::from_low_u64_le(0),
				ExitReason::Revert(_) => H256::from_low_u64_le(0),
				ExitReason::Fatal(_) => H256::from_low_u64_le(0),
			},
			used_gas,
			logs_bloom: status.clone().logs_bloom,
			logs: status.clone().logs,
		};
		// Pending::<T>::append((dvm_transaction.tx, status, receipt));
		Pending::<T>::append((dvm_transaction, status, receipt));
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
	pub fn current_receipts() -> Option<Vec<EthereumReceiptV0>> {
		CurrentReceipts::<T>::get()
	}

	/// Execute an Ethereum transaction
	pub fn execute(
		from: H160,
		transaction: &Transaction,
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
		) = {
			match transaction {
				// max_fee_per_gas and max_priority_fee_per_gas in legacy and 2930 transactions is
				// the provided gas_price.
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
				}
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
				}
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
				}
			}
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
			}
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
			}
		}
	}

	/// Save ethereum block
	fn store_block(post_log: bool, block_number: U256) {
		let mut transactions = Vec::new();
		let mut statuses = Vec::new();
		let mut receipts = Vec::new();
		let mut logs_bloom = Bloom::default();
		for (transaction, status, receipt) in Pending::<T>::get() {
			transactions.push(transaction);
			statuses.push(status);
			receipts.push(receipt.clone());
			Self::logs_bloom(receipt.logs.clone(), &mut logs_bloom);
		}

		let ommers = Vec::<ethereum::Header>::new();
		let receipts_root =
			ethereum::util::ordered_trie_root(receipts.iter().map(|r| rlp::encode(r)));
		let partial_header = ethereum::PartialHeader {
			parent_hash: Self::current_block_hash().unwrap_or_default(),
			beneficiary: darwinia_evm::Pallet::<T>::find_author(),
			state_root: T::StateRoot::get(),
			receipts_root,
			logs_bloom,
			difficulty: U256::zero(),
			number: block_number,
			gas_limit: T::BlockGasLimit::get(),
			gas_used: receipts
				.clone()
				.into_iter()
				.fold(U256::zero(), |acc, r| acc + r.used_gas),
			timestamp: UniqueSaturatedInto::<u64>::unique_saturated_into(
				<pallet_timestamp::Pallet<T>>::get(),
			),
			extra_data: Vec::new(),
			mix_hash: H256::default(),
			nonce: H64::default(),
		};
		// let block = EthereumBlockV0::new(partial_header, transactions, ommers);
		let block = ethereum::Block::new(partial_header, transactions.clone(), ommers);

		CurrentBlock::<T>::put(block.clone());
		CurrentReceipts::<T>::put(receipts);
		CurrentTransactionStatuses::<T>::put(statuses);
		BlockHash::<T>::insert(block_number, block.header.hash());

		if post_log {
			let digest = DigestItem::<T::Hash>::Consensus(
				FRONTIER_ENGINE_ID,
				PostLog::Hashes(dp_consensus::Hashes::from_block(block)).encode(),
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

// impl<T: Config> InternalTransactHandler for Pallet<T> {
// 	/// Execute transaction from pallet(internal transaction)
// 	/// NOTE: The difference between the rpc transaction and the internal transaction is that
// 	/// The internal transactions will catch and throw evm error comes from runner to caller.
// 	fn internal_transact(target: H160, input: Vec<u8>) -> DispatchResultWithPostInfo {
// 		let source = T::PalletId::get().into_h160();
// 		let nonce = <T as darwinia_evm::Config>::RingAccountBasic::account_basic(&source).nonce;
// 		let transaction = new_internal_transaction(nonce, target, input);
// 		debug_assert_eq!(transaction.tx.nonce, nonce);
// 		debug_assert_eq!(transaction.gas_price, None);

// 		Self::raw_transact(source, transaction).map(|(reason, used_gas)| match reason {
// 			// Only when exit_reason is successful, return Ok(...)
// 			ExitReason::Succeed(_) => Ok(PostDispatchInfo {
// 				actual_weight: Some(T::GasWeightMapping::gas_to_weight(
// 					used_gas.unique_saturated_into(),
// 				)),
// 				pays_fee: Pays::No,
// 			}),
// 			ExitReason::Error(_) => Err(<Error<T>>::InternalTransactionExitError.into()),
// 			ExitReason::Revert(_) => Err(<Error<T>>::InternalTransactionRevertError.into()),
// 			ExitReason::Fatal(_) => Err(<Error<T>>::InternalTransactionFatalError.into()),
// 		})?
// 	}

// 	/// Pure read-only call to contract, the sender is pallet dvm account.
// 	/// NOTE: You should never use raw call for any non-read-only operation, be carefully.
// 	fn read_only_call(contract: H160, input: Vec<u8>) -> Result<Vec<u8>, DispatchError> {
// 		sp_io::storage::start_transaction();
// 		let (_, _, info) = Self::execute(
// 			T::PalletId::get().into_h160(),
// 			input,
// 			U256::zero(),
// 			U256::from(INTERNAL_TX_GAS_LIMIT),
// 			None,
// 			None,
// 			TransactionAction::Call(contract),
// 			None,
// 		)?;
// 		sp_io::storage::rollback_transaction();
// 		match info {
// 			CallOrCreateInfo::Call(info) => match info.exit_reason {
// 				ExitReason::Succeed(_) => Ok(info.value),
// 				ExitReason::Error(_) => Err(<Error<T>>::InternalTransactionExitError.into()),
// 				ExitReason::Revert(_) => Err(<Error<T>>::InternalTransactionRevertError.into()),
// 				ExitReason::Fatal(_) => Err(<Error<T>>::InternalTransactionFatalError.into()),
// 			},
// 			_ => Err(<Error<T>>::ReadyOnlyCall.into()),
// 		}
// 	}
// }

/// The schema version for Pallet Ethereum's storage
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Encode, Decode)]
pub enum EthereumStorageSchema {
	Undefined,
	V1,
	V2,
}
impl Default for EthereumStorageSchema {
	fn default() -> Self {
		Self::Undefined
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
