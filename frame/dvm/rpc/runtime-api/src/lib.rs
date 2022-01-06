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
#![cfg_attr(not(feature = "std"), no_std)]

// --- crates.io ---
use codec::{Decode, Encode};
use ethereum::{
	BlockV0 as EthereumBlock, Log, Receipt as EthereumReceiptV0,
	TransactionV0 as EthereumTransactionV0,
};
use ethereum_types::Bloom;
use scale_info::TypeInfo;
// --- paritytech ---
use sp_core::{H160, H256, U256};
use sp_runtime::{traits::Block as BlockT, DispatchError, RuntimeDebug};
use sp_std::vec::Vec;
// --- darwinia-network ---
use dp_evm::{Account, CallInfo, CreateInfo};

#[derive(Clone, Default, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct TransactionStatus {
	pub transaction_hash: H256,
	pub transaction_index: u32,
	pub from: H160,
	pub to: Option<H160>,
	pub contract_address: Option<H160>,
	pub logs: Vec<Log>,
	pub logs_bloom: Bloom,
}

sp_api::decl_runtime_apis! {
	/// API necessary for Ethereum-compatibility layer.
	pub trait EthereumRuntimeRPCApi {
		/// Returns runtime defined darwinia_evm::ChainId.
		fn chain_id() -> u64;
		/// Returns darwinia_evm::Accounts by address.
		fn account_basic(address: H160) -> Account;
		/// Returns FixedGasPrice::min_gas_price
		fn gas_price() -> U256;
		/// For a given account address, returns darwinia_evm::AccountCodes.
		fn account_code_at(address: H160) -> Vec<u8>;
		/// Returns the converted FindAuthor::find_author authority id.
		fn author() -> H160;
		/// For a given account address and index, returns darwinia_evm::AccountStorages.
		fn storage_at(address: H160, index: U256) -> H256;
		/// Returns a dvm_ethereum::call response.
		fn call(
			from: H160,
			to: H160,
			data: Vec<u8>,
			value: U256,
			gas_limit: U256,
			gas_price: Option<U256>,
			nonce: Option<U256>,
			estimate: bool,
		) -> Result<CallInfo, DispatchError>;
		/// Returns a frame_ethereum::create response.
		fn create(
			from: H160,
			data: Vec<u8>,
			value: U256,
			gas_limit: U256,
			gas_price: Option<U256>,
			nonce: Option<U256>,
			estimate: bool,
		) -> Result<CreateInfo, DispatchError>;
		/// Return the current block.
		fn current_block() -> Option<EthereumBlock>;
		/// Return the current receipt.
		fn current_receipts() -> Option<Vec<EthereumReceiptV0>>;
		/// Return the current transaction status.
		fn current_transaction_statuses() -> Option<Vec<TransactionStatus>>;
		/// Return all the current data for a block in a single runtime call.
		fn current_all() -> (
			Option<EthereumBlock>,
			Option<Vec<EthereumReceiptV0>>,
			Option<Vec<TransactionStatus>>
		);
		/// Receives a `Vec<OpaqueExtrinsic>` and filters all the ethereum transactions.
		fn extrinsic_filter(
			xts: Vec<<Block as BlockT>::Extrinsic>,
		) -> Vec<EthereumTransactionV0>;
	}
}

pub trait ConvertTransaction<E> {
	fn convert_transaction(&self, transaction: EthereumTransactionV0) -> E;
}
