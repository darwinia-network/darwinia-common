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
	BlockV0 as EthereumBlockV0, LegacyTransactionMessage, Log, Receipt as EthereumReceiptV0,
	TransactionAction, TransactionSignature, TransactionV0,
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
#[cfg(feature = "std")]
use frame_support::storage::unhashed;
use frame_support::{
	dispatch::DispatchResultWithPostInfo,
	ensure,
	traits::{Currency, Get},
	weights::{Pays, PostDispatchInfo, Weight},
	PalletId,
};
use frame_system::ensure_none;
use sp_runtime::{
	generic::DigestItem,
	traits::{One, Saturating, UniqueSaturatedInto, Zero},
	transaction_validity::{
		InvalidTransaction, TransactionSource, TransactionValidity, ValidTransactionBuilder,
	},
	DispatchError,
};
use sp_std::{marker::PhantomData, prelude::*};
// --- darwinia-network ---
use darwinia_evm::{AccountBasic, BlockHashMapping, FeeCalculator, GasWeightMapping, Runner};
use darwinia_support::evm::{recover_signer, DVMTransaction, IntoH160, INTERNAL_TX_GAS_LIMIT};
use dp_consensus::{PostLog, PreLog, FRONTIER_ENGINE_ID};
use dp_evm::CallOrCreateInfo;
#[cfg(feature = "std")]
use dp_storage::PALLET_ETHEREUM_SCHEMA;

/// A type alias for the balance type from this pallet's point of view.
type AccountId<T> = <T as frame_system::Config>::AccountId;
type RingCurrency<T> = <T as Config>::RingCurrency;
type KtonCurrency<T> = <T as Config>::KtonCurrency;
type RingBalance<T> = <RingCurrency<T> as Currency<AccountId<T>>>::Balance;
type KtonBalance<T> = <KtonCurrency<T> as Currency<AccountId<T>>>::Balance;

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
			if let Ok(log) = dp_consensus::find_pre_log(&<frame_system::Pallet<T>>::digest()) {
				let PreLog::Block(block) = log;

				for transaction in block.transactions {
					Self::rpc_transact(transaction).expect(
						"pre-block transaction verification failed; the block cannot be built",
					);
				}
			}
			0
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Transact an Ethereum transaction.
		#[pallet::weight(<T as darwinia_evm::Config>::GasWeightMapping::gas_to_weight(transaction.gas_limit.unique_saturated_into()))]
		pub fn transact(
			origin: OriginFor<T>,
			transaction: TransactionV0,
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;

			Self::rpc_transact(transaction)
		}

		/// Internal transaction only for root.
		#[pallet::weight(10_000_000)]
		pub fn root_transact(
			origin: OriginFor<T>,
			target: H160,
			input: Vec<u8>,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			Self::internal_transact(target, input)
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

	#[pallet::validate_unsigned]
	impl<T: Config> frame_support::unsigned::ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			if let Call::transact { transaction } = call {
				// We must ensure a transaction can pay the cost of its data bytes.
				// If it can't it should not be included in a block.
				let mut gasometer = evm::gasometer::Gasometer::new(
					transaction.gas_limit.low_u64(),
					<T as darwinia_evm::Config>::config(),
				);
				let transaction_cost = match transaction.action {
					TransactionAction::Call(_) => {
						evm::gasometer::call_transaction_cost(&transaction.input)
					}
					TransactionAction::Create => {
						evm::gasometer::create_transaction_cost(&transaction.input)
					}
				};
				if gasometer.record_transaction(transaction_cost).is_err() {
					return InvalidTransaction::Custom(
						TransactionValidationError::InvalidGasLimit as u8,
					)
					.into();
				}

				// Check chain id correctly
				if let Some(chain_id) = transaction.signature.chain_id() {
					if chain_id != T::ChainId::get() {
						return InvalidTransaction::Custom(
							TransactionValidationError::InvalidChainId as u8,
						)
						.into();
					}
				}
				// Check signature correctly
				let origin = recover_signer(&transaction).ok_or_else(|| {
					InvalidTransaction::Custom(TransactionValidationError::InvalidSignature as u8)
				})?;
				// Check transaction gas limit correctly
				if transaction.gas_limit >= T::BlockGasLimit::get() {
					return InvalidTransaction::Custom(
						TransactionValidationError::InvalidGasLimit as u8,
					)
					.into();
				}
				let account_data =
					<T as darwinia_evm::Config>::RingAccountBasic::account_basic(&origin);
				// Check sender's nonce correctly
				if transaction.nonce < account_data.nonce {
					return InvalidTransaction::Stale.into();
				}
				// Check sender's balance correctly
				let fee = transaction.gas_price.saturating_mul(transaction.gas_limit);
				let total_payment = transaction.value.saturating_add(fee);
				if account_data.balance < total_payment {
					return InvalidTransaction::Payment.into();
				}
				// Check transaction gas price correctly
				let min_gas_price = T::FeeCalculator::min_gas_price();
				if transaction.gas_price < min_gas_price {
					return InvalidTransaction::Payment.into();
				}

				let mut builder = ValidTransactionBuilder::default()
					.and_provides((origin, transaction.nonce))
					.priority(transaction.gas_price.unique_saturated_into());

				if transaction.nonce > account_data.nonce {
					if let Some(prev_nonce) = transaction.nonce.checked_sub(1.into()) {
						builder = builder.and_requires((origin, prev_nonce))
					}
				}

				builder.build()
			} else {
				Err(InvalidTransaction::Call.into())
			}
		}
	}

	/// Current building block's transactions and receipts.
	#[pallet::storage]
	pub(super) type Pending<T: Config> =
		StorageValue<_, Vec<(TransactionV0, TransactionStatus, EthereumReceiptV0)>, ValueQuery>;

	/// The current Ethereum block.
	#[pallet::storage]
	pub(super) type CurrentBlock<T: Config> = StorageValue<_, EthereumBlockV0>;

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
					&EthereumStorageSchema::V1,
				);
			};
			extra_genesis_builder(self);
		}
	}
}
pub use pallet::*;

impl<T: Config> Pallet<T> {
	/// Execute transaction from EthApi(network transaction)
	/// NOTE: For the rpc transaction, the execution will return ok(..) even when encounters error
	/// from evm runner
	pub fn rpc_transact(transaction: TransactionV0) -> DispatchResultWithPostInfo {
		ensure!(
			dp_consensus::find_pre_log(&<frame_system::Pallet<T>>::digest()).is_err(),
			Error::<T>::PreLogExists,
		);
		let transaction = Self::to_dvm_transaction(transaction)?;
		Self::raw_transact(transaction).map(|(_, used_gas)| {
			Ok(Some(T::GasWeightMapping::gas_to_weight(
				used_gas.unique_saturated_into(),
			))
			.into())
		})?
	}

	/// Execute DVMTransaction in evm runner and save the execution info in Pending
	fn raw_transact(transaction: DVMTransaction) -> Result<(ExitReason, U256), DispatchError> {
		let transaction_hash =
			H256::from_slice(Keccak256::digest(&rlp::encode(&transaction.tx)).as_slice());
		let transaction_index = Pending::<T>::get().len() as u32;

		let (to, _contract_address, info) = Self::execute(
			transaction.source,
			transaction.tx.input.clone(),
			transaction.tx.value,
			transaction.tx.gas_limit,
			transaction.gas_price,
			Some(transaction.tx.nonce),
			transaction.tx.action,
			None,
		)?;

		let (reason, status, used_gas, dest) = match info {
			CallOrCreateInfo::Call(info) => (
				info.exit_reason,
				TransactionStatus {
					transaction_hash,
					transaction_index,
					from: transaction.source,
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
					from: transaction.source,
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

		let receipt = EthereumReceiptV0 {
			state_root: match reason {
				ExitReason::Succeed(_) => H256::from_low_u64_be(1),
				ExitReason::Error(_) => H256::from_low_u64_le(0),
				ExitReason::Revert(_) => H256::from_low_u64_le(0),
				ExitReason::Fatal(_) => H256::from_low_u64_le(0),
			},
			used_gas,
			logs_bloom: status.logs_bloom,
			logs: status.clone().logs,
		};

		Pending::<T>::append((transaction.tx, status, receipt));

		Self::deposit_event(Event::Executed(
			transaction.source,
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
	pub fn current_block() -> Option<EthereumBlockV0> {
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
		input: Vec<u8>,
		value: U256,
		gas_limit: U256,
		gas_price: Option<U256>,
		nonce: Option<U256>,
		action: TransactionAction,
		config: Option<evm::Config>,
	) -> Result<(Option<H160>, Option<H160>, CallOrCreateInfo), DispatchError> {
		match action {
			ethereum::TransactionAction::Call(target) => {
				let res = T::Runner::call(
					from,
					target,
					input,
					value,
					gas_limit.low_u64(),
					gas_price,
					nonce,
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
					gas_price,
					nonce,
					config.as_ref().unwrap_or(T::config()),
				)
				.map_err(Into::into)?;

				Ok((None, Some(res.value), CallOrCreateInfo::Create(res)))
			}
		}
	}

	/// Transfer rpc transaction to dvm transaction
	fn to_dvm_transaction(transaction: TransactionV0) -> Result<DVMTransaction, DispatchError> {
		let source = recover_signer(&transaction).ok_or(Error::<T>::InvalidSignature)?;
		Ok(DVMTransaction {
			source,
			gas_price: Some(transaction.gas_price),
			tx: transaction,
		})
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
		let block = EthereumBlockV0::new(partial_header, transactions, ommers);

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

impl<T: Config> InternalTransactHandler for Pallet<T> {
	/// Execute transaction from pallet(internal transaction)
	/// NOTE: The difference between the rpc transaction and the internal transaction is that
	/// The internal transactions will catch and throw evm error comes from runner to caller.
	fn internal_transact(target: H160, input: Vec<u8>) -> DispatchResultWithPostInfo {
		ensure!(
			dp_consensus::find_pre_log(&<frame_system::Pallet<T>>::digest()).is_err(),
			Error::<T>::PreLogExists,
		);

		let source = T::PalletId::get().into_h160();
		let nonce = <T as darwinia_evm::Config>::RingAccountBasic::account_basic(&source).nonce;
		let transaction = DVMTransaction::new_internal_transaction(source, nonce, target, input);
		Self::raw_transact(transaction).map(|(reason, used_gas)| match reason {
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
		let (_, _, info) = Self::execute(
			T::PalletId::get().into_h160(),
			input,
			U256::zero(),
			U256::from(INTERNAL_TX_GAS_LIMIT),
			None,
			None,
			TransactionAction::Call(contract),
			None,
		)?;
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

/// The schema version for Pallet Ethereum's storage
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Encode, Decode)]
pub enum EthereumStorageSchema {
	Undefined,
	V1,
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
