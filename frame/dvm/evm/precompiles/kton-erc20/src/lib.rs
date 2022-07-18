// SPDX-License-Identifier: Apache-2.0
// This file is part of Frontier.
//
// Copyright (c) 2020 Parity Technologies (UK) Ltd.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

extern crate alloc;

// --- core ---
use alloc::vec;
use core::marker::PhantomData;
// --- crates.io ---
use ethabi::StateMutability;
// --- darwinia-network ---
use darwinia_evm::CurrencyAdapt;
use darwinia_evm_precompile_utils::{prelude::*, PrecompileHelper};
use darwinia_support::evm::DeriveSubstrateAddress;
// --- paritytech ---
use fp_evm::{Context, ExitRevert, ExitSucceed, Precompile, PrecompileFailure, PrecompileOutput};
use frame_support::{
	storage::types::{StorageDoubleMap, ValueQuery},
	traits::StorageInstance,
	Blake2_128Concat,
};
use sp_core::{Decode, H160, U256};

const TOKEN_NAME: &str = "Wrapped KTON";
const TOKEN_SYMBOL: &str = "WKTON";
const TOKEN_DECIMAL: u8 = 18;

/// Solidity selector of the Transfer log, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_TRANSFER: [u8; 32] = keccak256!("Transfer(address,address,uint256)");
/// Solidity selector of the Approval log, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_APPROVAL: [u8; 32] = keccak256!("Approval(address,address,uint256)");
/// Solidity selector of the Withdraw log, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_WITHDRAWAL: [u8; 32] = keccak256!("Withdrawal(address,uint256)");

type KtonBalanceAdapter<T> = <T as darwinia_evm::Config>::KtonBalanceAdapter;
type IntoAccountId<T> = <T as darwinia_evm::Config>::IntoAccountId;

struct Approves;
impl StorageInstance for Approves {
	const STORAGE_PREFIX: &'static str = "Approves";

	fn pallet_prefix() -> &'static str {
		"ERC20Kton"
	}
}

type ApprovesStorage =
	StorageDoubleMap<Approves, Blake2_128Concat, H160, Blake2_128Concat, H160, U256, ValueQuery>;

#[darwinia_evm_precompile_utils::selector]
enum Action {
	TotalSupply = "totalSupply()",
	BalanceOf = "balanceOf(address)",
	Transfer = "transfer(address,uint256)",
	Allowance = "allowance(address,address)",
	Approve = "approve(address,uint256)",
	TransferFrom = "transferFrom(address,address,uint256)",
	Withdraw = "withdraw(bytes32,uint256)",
	Name = "name()",
	Symbol = "symbol()",
	Decimals = "decimals()",
}

pub struct KtonErc20<T>(PhantomData<T>);

impl<T> Precompile for KtonErc20<T>
where
	T: darwinia_evm::Config,
{
	fn execute(
		input: &[u8],
		target_gas: Option<u64>,
		context: &Context,
		is_static: bool,
	) -> EvmResult<PrecompileOutput> {
		let mut helper = PrecompileHelper::<T>::new(input, target_gas, context, is_static);
		let action = helper.selector().unwrap_or_else(|_| Action::Name);

		match action {
			Action::Transfer
			| Action::Allowance
			| Action::Approve
			| Action::TransferFrom
			| Action::Withdraw => helper.check_state_modifier(StateMutability::NonPayable)?,
			_ => helper.check_state_modifier(StateMutability::View)?,
		};

		match action {
			Action::TotalSupply => Self::total_supply(&mut helper),
			Action::Name => Self::name(&mut helper),
			Action::Symbol => Self::symbol(&mut helper),
			Action::Decimals => Self::decimals(&mut helper),
			Action::BalanceOf => Self::balance_of(&mut helper),
			Action::Transfer => Self::transfer(&mut helper, context),
			Action::Allowance => Self::allowance(&mut helper),
			Action::Approve => Self::approve(&mut helper, context),
			Action::TransferFrom => Self::transfer_from(&mut helper, context),
			Action::Withdraw => Self::withdraw(&mut helper, context),
		}
	}
}

impl<T> KtonErc20<T>
where
	T: darwinia_evm::Config,
{
	fn total_supply(helper: &mut PrecompileHelper<T>) -> EvmResult<PrecompileOutput> {
		let reader = helper.reader()?;
		reader.expect_arguments(0)?;

		helper.record_db_gas(1, 0)?;

		let amount = <KtonBalanceAdapter<T>>::evm_total_supply();

		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			output: EvmDataWriter::new().write(amount).build(),
			cost: helper.used_gas(),
			logs: vec![],
		})
	}

	fn balance_of(helper: &mut PrecompileHelper<T>) -> EvmResult<PrecompileOutput> {
		let mut reader = helper.reader()?;
		reader.expect_arguments(1)?;
		let owner: H160 = reader.read::<Address>()?.into();

		helper.record_db_gas(2, 0)?;

		let amount = <KtonBalanceAdapter<T>>::evm_balance(&owner);

		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			output: EvmDataWriter::new().write(amount).build(),
			cost: helper.used_gas(),
			logs: vec![],
		})
	}

	fn allowance(helper: &mut PrecompileHelper<T>) -> EvmResult<PrecompileOutput> {
		let mut reader = helper.reader()?;
		reader.expect_arguments(2)?;
		let owner: H160 = reader.read::<Address>()?.into();
		let spender: H160 = reader.read::<Address>()?.into();

		helper.record_db_gas(1, 0)?;

		let amount: U256 = ApprovesStorage::get(owner, spender).into();

		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			output: EvmDataWriter::new().write(amount).build(),
			cost: helper.used_gas(),
			logs: vec![],
		})
	}

	fn approve(helper: &mut PrecompileHelper<T>, context: &Context) -> EvmResult<PrecompileOutput> {
		let mut reader = helper.reader()?;
		reader.expect_arguments(2)?;
		let spender: H160 = reader.read::<Address>()?.into();
		let amount: U256 = reader.read()?;

		helper.record_db_gas(1, 0)?;
		helper.record_log_gas(3, 32)?;

		ApprovesStorage::insert(context.caller, spender, amount);

		let approve_log = log3(
			context.address,
			SELECTOR_LOG_APPROVAL,
			context.caller,
			spender,
			EvmDataWriter::new().write(amount).build(),
		);

		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			output: EvmDataWriter::new().write(true).build(),
			cost: helper.used_gas(),
			logs: vec![approve_log],
		})
	}

	fn transfer(
		helper: &mut PrecompileHelper<T>,
		context: &Context,
	) -> EvmResult<PrecompileOutput> {
		let mut reader = helper.reader()?;
		reader.expect_arguments(2)?;
		let to: H160 = reader.read::<Address>()?.into();
		let amount: U256 = reader.read()?;

		helper.record_db_gas(2, 2)?;
		helper.record_log_gas(3, 32)?;

		let origin = <IntoAccountId<T>>::derive_substrate_address(&context.caller);
		let to_account_id = <IntoAccountId<T>>::derive_substrate_address(&to);
		<KtonBalanceAdapter<T>>::evm_transfer(&origin, &to_account_id, amount)
			.map_err(|_| revert("Transfer failed"))?;

		let transfer_log = log3(
			context.address,
			SELECTOR_LOG_TRANSFER,
			context.caller,
			to,
			EvmDataWriter::new().write(amount).build(),
		);

		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			output: EvmDataWriter::new().write(true).build(),
			cost: helper.used_gas(),
			logs: vec![transfer_log],
		})
	}

	fn transfer_from(
		helper: &mut PrecompileHelper<T>,
		context: &Context,
	) -> EvmResult<PrecompileOutput> {
		let mut reader = helper.reader()?;
		reader.expect_arguments(3)?;
		let from: H160 = reader.read::<Address>()?.into();
		let to: H160 = reader.read::<Address>()?.into();
		let amount: U256 = reader.read()?;

		helper.record_db_gas(3, 3)?;

		let caller = context.caller;
		if caller != from {
			ApprovesStorage::mutate(from, caller, |value| {
				let new_value = value
					.checked_sub(amount)
					.ok_or_else(|| revert("trying to spend more than allowed"))?;

				*value = new_value;
				EvmResult::Ok(())
			})?;
		}

		let origin = <IntoAccountId<T>>::derive_substrate_address(&from);
		let to_account_id = <IntoAccountId<T>>::derive_substrate_address(&to);
		<KtonBalanceAdapter<T>>::evm_transfer(&origin, &to_account_id, amount)
			.map_err(|_| revert("Transfer failed"))?;

		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			output: EvmDataWriter::new().write(true).build(),
			cost: helper.used_gas(),
			logs: vec![],
		})
	}

	fn withdraw(
		helper: &mut PrecompileHelper<T>,
		context: &Context,
	) -> EvmResult<PrecompileOutput> {
		let mut reader = helper.reader()?;
		reader.expect_arguments(2)?;
		let to: Bytes = reader.read()?;
		let amount: U256 = reader.read()?;

		helper.record_db_gas(1, 0)?;
		helper.record_log_gas(2, 32)?;

		let origin = <IntoAccountId<T>>::derive_substrate_address(&context.caller);
		let to_account_id = <T as frame_system::Config>::AccountId::decode(&mut to.as_bytes())
			.map_err(|_| revert("Invalid target address"))?;

		<KtonBalanceAdapter<T>>::evm_transfer(&origin, &to_account_id, amount)
			.map_err(|_| revert("Transfer failed"))?;

		let withdraw_log = log2(
			context.address,
			SELECTOR_LOG_WITHDRAWAL,
			context.caller,
			EvmDataWriter::new().write(amount).build(),
		);

		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			output: EvmDataWriter::new().write(true).build(),
			cost: helper.used_gas(),
			logs: vec![withdraw_log],
		})
	}

	fn name(helper: &mut PrecompileHelper<T>) -> EvmResult<PrecompileOutput> {
		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			output: EvmDataWriter::new().write::<Bytes>(TOKEN_NAME.into()).build(),
			cost: helper.used_gas(),
			logs: vec![],
		})
	}

	fn symbol(helper: &mut PrecompileHelper<T>) -> EvmResult<PrecompileOutput> {
		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			output: EvmDataWriter::new().write::<Bytes>(TOKEN_SYMBOL.into()).build(),
			cost: helper.used_gas(),
			logs: vec![],
		})
	}

	fn decimals(helper: &mut PrecompileHelper<T>) -> EvmResult<PrecompileOutput> {
		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			output: EvmDataWriter::new().write(TOKEN_DECIMAL).build(),
			cost: helper.used_gas(),
			logs: vec![],
		})
	}
}
