// Copyright 2017-2020 Parity Technologies (UK) Ltd.
// This file is part of Frontier.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate. If not, see <http://www.gnu.org/licenses/>.

//! # Ethereum pallet
//!
//! The Ethereum pallet works together with EVM pallet to provide full emulation
//! for Ethereum block processing.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

// --- darwinia ---
use darwinia_evm::{AccountBasic, FeeCalculator, GasWeightMapping, Runner};
use dp_consensus::{PostLog, PreLog, FRONTIER_ENGINE_ID};
use dp_evm::CallOrCreateInfo;
#[cfg(feature = "std")]
use dp_storage::PALLET_ETHEREUM_SCHEMA;
pub use dvm_rpc_runtime_api::TransactionStatus;
// --- substrate ---
use frame_support::ensure;
use frame_support::traits::Currency;
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage, dispatch::DispatchResultWithPostInfo,
	traits::FindAuthor, traits::Get, weights::Weight,
};
use frame_system::ensure_none;
use sp_runtime::{
	generic::DigestItem,
	traits::UniqueSaturatedInto,
	transaction_validity::{
		InvalidTransaction, TransactionSource, TransactionValidity, ValidTransactionBuilder,
	},
	DispatchError,
};
use sp_std::prelude::*;
// --- std ---
use codec::{Decode, Encode};
pub use ethereum::{Block, Log, Receipt, Transaction, TransactionAction, TransactionMessage};
use ethereum_types::{Bloom, BloomInput, H160, H256, H64, U256};
use evm::ExitReason;
use sha3::{Digest, Keccak256};

#[cfg(all(feature = "std", test))]
mod tests;

pub mod account_basic;
#[cfg(all(feature = "std", test))]
mod mock;

#[derive(Eq, PartialEq, Clone, sp_runtime::RuntimeDebug)]
pub enum ReturnValue {
	Bytes(Vec<u8>),
	Hash(H160),
}

/// The schema version for Pallet Ethereum's storage
#[derive(Clone, Debug, Encode, Decode, PartialEq, Eq, PartialOrd, Ord)]
pub enum EthereumStorageSchema {
	Undefined,
	V1,
}

impl Default for EthereumStorageSchema {
	fn default() -> Self {
		Self::Undefined
	}
}

/// A type alias for the balance type from this pallet's point of view.
type AccountId<T> = <T as frame_system::Config>::AccountId;
pub type RingCurrency<T> = <T as Config>::RingCurrency;
pub type KtonCurrency<T> = <T as Config>::KtonCurrency;
pub type RingBalance<T> = <RingCurrency<T> as Currency<AccountId<T>>>::Balance;
pub type KtonBalance<T> = <KtonCurrency<T> as Currency<AccountId<T>>>::Balance;

pub struct IntermediateStateRoot;

impl Get<H256> for IntermediateStateRoot {
	fn get() -> H256 {
		H256::decode(&mut &sp_io::storage::root()[..])
			.expect("Node is configured to use the same hash; qed")
	}
}

/// Config for Ethereum pallet.
pub trait Config:
	frame_system::Config<Hash = H256> + pallet_timestamp::Config + darwinia_evm::Config
{
	/// The overarching event type.
	type Event: From<Event> + Into<<Self as frame_system::Config>::Event>;
	/// Find author for Ethereum.
	type FindAuthor: FindAuthor<H160>;
	/// How Ethereum state root is calculated.
	type StateRoot: Get<H256>;
	// RING Balance module
	type RingCurrency: Currency<Self::AccountId>;
	// KTON Balance module
	type KtonCurrency: Currency<Self::AccountId>;
}

decl_storage! {
	trait Store for Module<T: Config> as Ethereum {
		/// Current building block's transactions and receipts.
		Pending: Vec<(ethereum::Transaction, TransactionStatus, ethereum::Receipt)>;

		/// The current Ethereum block.
		CurrentBlock: Option<ethereum::Block>;
		/// The current Ethereum receipts.
		CurrentReceipts: Option<Vec<ethereum::Receipt>>;
		/// The current transaction statuses.
		CurrentTransactionStatuses: Option<Vec<TransactionStatus>>;
		/// Remaining ring balance for account
		RemainingRingBalance get(fn get_ring_remaining_balances): map hasher(blake2_128_concat) T::AccountId => RingBalance<T>;
		/// Remaining kton balance for account
		RemainingKtonBalance get(fn get_kton_remaining_balances): map hasher(blake2_128_concat) T::AccountId => KtonBalance<T>;
	}
	add_extra_genesis {
		build(|_config: &GenesisConfig| {
			<Module<T>>::store_block(false);

			// Initialize the storage schema at the well known key.
			frame_support::storage::unhashed::put::<EthereumStorageSchema>(&PALLET_ETHEREUM_SCHEMA, &EthereumStorageSchema::V1);
		});
	}
}

decl_event!(
	/// Ethereum pallet events.
	pub enum Event {
		/// An ethereum transaction was successfully executed. [from, to/contract_address, transaction_hash, exit_reason]
		Executed(H160, H160, H256, ExitReason),
	}
);

decl_error! {
	/// Ethereum pallet errors.
	pub enum Error for Module<T: Config> {
		/// Signature is invalid.
		InvalidSignature,
		/// Pre-log is present, therefore transact is not allowed.
		PreLogExists,
	}
}

decl_module! {
	/// Ethereum pallet module.
	pub struct Module<T: Config> for enum Call where origin: T::Origin {
		/// Deposit one of this pallet's events by using the default implementation.
		fn deposit_event() = default;

		/// Transact an Ethereum transaction.
		#[weight = <T as darwinia_evm::Config>::GasWeightMapping::gas_to_weight(transaction.gas_limit.unique_saturated_into())]
		fn transact(origin, transaction: ethereum::Transaction) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;

			Self::do_transact(transaction)
		}

		fn on_finalize(_block_number: T::BlockNumber) {
			<Module<T>>::store_block(
				dp_consensus::find_pre_log(&<frame_system::Pallet<T>>::digest()).is_err(),
			);
		}

		fn on_initialize(_block_number: T::BlockNumber) -> Weight {
			Pending::kill();
			if let Ok(log) = dp_consensus::find_pre_log(&<frame_system::Pallet<T>>::digest()) {
				let PreLog::Block(block) = log;

				for transaction in block.transactions {
					Self::do_transact(transaction).expect("pre-block transaction verification failed; the block cannot be built");
				}
			}
			0
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

impl<T: Config> frame_support::unsigned::ValidateUnsigned for Module<T> {
	type Call = Call<T>;

	fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
		if let Call::transact(transaction) = call {
			if let Some(chain_id) = transaction.signature.chain_id() {
				if chain_id != T::ChainId::get() {
					return InvalidTransaction::Custom(
						TransactionValidationError::InvalidChainId as u8,
					)
					.into();
				}
			}

			let origin = Self::recover_signer(&transaction).ok_or_else(|| {
				InvalidTransaction::Custom(TransactionValidationError::InvalidSignature as u8)
			})?;

			if transaction.gas_limit >= T::BlockGasLimit::get() {
				return InvalidTransaction::Custom(
					TransactionValidationError::InvalidGasLimit as u8,
				)
				.into();
			}

			let account_data =
				<T as darwinia_evm::Config>::RingAccountBasic::account_basic(&origin);

			if transaction.nonce < account_data.nonce {
				return InvalidTransaction::Stale.into();
			}

			let fee = transaction.gas_price.saturating_mul(transaction.gas_limit);
			let total_payment = transaction.value.saturating_add(fee);
			if account_data.balance < total_payment {
				return InvalidTransaction::Payment.into();
			}

			if transaction.gas_price < T::FeeCalculator::min_gas_price() {
				return InvalidTransaction::Payment.into();
			}

			let mut builder =
				ValidTransactionBuilder::default().and_provides((origin, transaction.nonce));

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

impl<T: Config> Module<T> {
	fn recover_signer(transaction: &ethereum::Transaction) -> Option<H160> {
		let mut sig = [0u8; 65];
		let mut msg = [0u8; 32];
		sig[0..32].copy_from_slice(&transaction.signature.r()[..]);
		sig[32..64].copy_from_slice(&transaction.signature.s()[..]);
		sig[64] = transaction.signature.standard_v();
		msg.copy_from_slice(&TransactionMessage::from(transaction.clone()).hash()[..]);

		let pubkey = sp_io::crypto::secp256k1_ecdsa_recover(&sig, &msg).ok()?;
		Some(H160::from(H256::from_slice(
			Keccak256::digest(&pubkey).as_slice(),
		)))
	}

	fn store_block(post_log: bool) {
		let mut transactions = Vec::new();
		let mut statuses = Vec::new();
		let mut receipts = Vec::new();
		let mut logs_bloom = Bloom::default();
		for (transaction, status, receipt) in Pending::get() {
			transactions.push(transaction);
			statuses.push(status);
			receipts.push(receipt.clone());
			Self::logs_bloom(receipt.logs.clone(), &mut logs_bloom);
		}

		let ommers = Vec::<ethereum::Header>::new();
		let partial_header = ethereum::PartialHeader {
			parent_hash: Self::current_block_hash().unwrap_or_default(),
			beneficiary: <Module<T>>::find_author(),
			// TODO: figure out if there's better way to get a sort-of-valid state root.
			state_root: H256::default(),
			receipts_root: H256::from_slice(
				Keccak256::digest(&rlp::encode_list(&receipts)[..]).as_slice(),
			), // TODO: check receipts hash.
			logs_bloom,
			difficulty: U256::zero(),
			number: U256::from(UniqueSaturatedInto::<u128>::unique_saturated_into(
				<frame_system::Pallet<T>>::block_number(),
			)),
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
		let mut block = ethereum::Block::new(partial_header, transactions.clone(), ommers);
		block.header.state_root = T::StateRoot::get();

		CurrentBlock::put(block.clone());
		CurrentReceipts::put(receipts.clone());
		CurrentTransactionStatuses::put(statuses.clone());

		if post_log {
			let digest = DigestItem::<T::Hash>::Consensus(
				FRONTIER_ENGINE_ID,
				PostLog::Hashes(dp_consensus::Hashes::from_block(block)).encode(),
			);
			<frame_system::Pallet<T>>::deposit_log(digest.into());
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

	fn do_transact(transaction: ethereum::Transaction) -> DispatchResultWithPostInfo {
		ensure!(
			dp_consensus::find_pre_log(&<frame_system::Pallet<T>>::digest()).is_err(),
			Error::<T>::PreLogExists,
		);
		let source =
			Self::recover_signer(&transaction).ok_or_else(|| Error::<T>::InvalidSignature)?;

		let transaction_hash =
			H256::from_slice(Keccak256::digest(&rlp::encode(&transaction)).as_slice());
		let transaction_index = Pending::get().len() as u32;

		let (to, contract_address, info) = Self::execute(
			source,
			transaction.input.clone(),
			transaction.value,
			transaction.gas_limit,
			Some(transaction.gas_price),
			Some(transaction.nonce),
			transaction.action,
			None,
		)?;

		let (reason, status, used_gas) = match info {
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

		Pending::append((transaction, status, receipt));

		Self::deposit_event(Event::Executed(
			source,
			contract_address.unwrap_or_default(),
			transaction_hash,
			reason,
		));
		Ok(Some(T::GasWeightMapping::gas_to_weight(
			used_gas.unique_saturated_into(),
		))
		.into())
	}

	/// Get the author using the FindAuthor trait.
	pub fn find_author() -> H160 {
		let digest = <frame_system::Pallet<T>>::digest();
		let pre_runtime_digests = digest.logs.iter().filter_map(|d| d.as_pre_runtime());

		T::FindAuthor::find_author(pre_runtime_digests).unwrap_or_default()
	}

	/// Get the transaction status with given index.
	pub fn current_transaction_statuses() -> Option<Vec<TransactionStatus>> {
		CurrentTransactionStatuses::get()
	}
	/// Get current block.
	pub fn current_block() -> Option<ethereum::Block> {
		CurrentBlock::get()
	}

	/// Get current block hash
	pub fn current_block_hash() -> Option<H256> {
		Self::current_block().map(|block| block.header.hash())
	}

	/// Get receipts by number.
	pub fn current_receipts() -> Option<Vec<ethereum::Receipt>> {
		CurrentReceipts::get()
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
					input.clone(),
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
					input.clone(),
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
}
