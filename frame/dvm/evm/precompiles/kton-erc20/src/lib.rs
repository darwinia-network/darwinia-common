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

// --- core ---
use core::marker::PhantomData;
// --- crates.io ---
use ethabi::{token::Token, Error, ParamType, StateMutability};
// --- darwinia-network ---
use darwinia_evm::AccountBasic;
use darwinia_evm_precompile_utils::{prelude::*, PrecompileHelper};
use darwinia_support::evm::DeriveSubstrateAddress;
// --- paritytech ---
use fp_evm::{Context, ExitRevert, ExitSucceed, Precompile, PrecompileFailure, PrecompileOutput};
use frame_support::{
	storage::types::{StorageDoubleMap, ValueQuery},
	traits::StorageInstance,
	Blake2_128Concat,
};
use sp_core::{Decode, H160, H256, U256};

type BalanceOf<T> = <T as darwinia_balances::Config>::Balance;

pub struct Approves;

impl StorageInstance for Approves {
	const STORAGE_PREFIX: &'static str = "Approves";

	fn pallet_prefix() -> &'static str {
		"ERC20Kton"
	}
}

pub type ApprovesStorage =
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

pub struct KtonErc20<T> {
	_marker: PhantomData<T>,
}

impl<T> Precompile for KtonErc20<T>
where
	T: darwinia_evm::Config + darwinia_balances::Config,
	BalanceOf<T>: Into<U256>,
{
	fn execute(
		input: &[u8],
		target_gas: Option<u64>,
		context: &Context,
		_is_static: bool,
	) -> EvmResult<PrecompileOutput> {
		let mut helper = PrecompileHelper::<T>::new(input, target_gas);
		let (selector, data) = helper.split_input()?;
		let action = Action::from_u32(selector)?;

		// TODO: Add state modifier checker

		match action {
			Action::TotalSupply => Self::total_supply(&mut helper),
			Action::Name => Self::name(&mut helper),
			Action::Symbol => Self::symbol(&mut helper),
			Action::Decimals => Self::decimals(&mut helper),
			Action::BalanceOf => Self::balance_of(&mut helper, data),
			Action::Transfer => Self::transfer(&mut helper, data, context),
			Action::Allowance => Self::allowance(&mut helper, data),
			Action::Approve => Self::approve(&mut helper, data, context),
			Action::TransferFrom => Self::transfer_from(&mut helper, data, context),
			Action::Withdraw => Self::withdraw(&mut helper, data, context),
		}
	}
}

impl<T> KtonErc20<T>
where
	T: darwinia_evm::Config + darwinia_balances::Config,
	BalanceOf<T>: Into<U256>,
{
	fn total_supply(helper: &mut PrecompileHelper<T>) -> EvmResult<PrecompileOutput> {
		helper.record_gas(1, 0)?;

		// TODO: precision check
		let amount: U256 = darwinia_balances::Pallet::<T>::total_issuance().into();

		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			output: EvmDataWriter::new().write(amount).build(),
			cost: helper.used_gas(),
			logs: vec![],
		})
	}

	fn balance_of(helper: &mut PrecompileHelper<T>, input: &[u8]) -> EvmResult<PrecompileOutput> {
		helper.record_gas(1, 0)?;

		let mut reader = EvmDataReader::new_skip_selector(input)?;
		reader.expect_arguments(1)?;
		let owner: H160 = reader.read::<Address>()?.into();

		let amount = <T as darwinia_evm::Config>::KtonAccountBasic::account_basic(&owner).balance;

		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			output: EvmDataWriter::new().write(amount).build(),
			cost: helper.used_gas(),
			logs: vec![],
		})
	}

	fn transfer(
		helper: &mut PrecompileHelper<T>,
		input: &[u8],
		context: &Context,
	) -> EvmResult<PrecompileOutput> {
		// TODO: update the gas record
		helper.record_gas(1, 0)?;

		let mut reader = EvmDataReader::new_skip_selector(input)?;
		reader.expect_arguments(2)?;
		let to: H160 = reader.read::<Address>()?.into();
		let amount: U256 = reader.read()?;

		let origin =
			<T as darwinia_evm::Config>::IntoAccountId::derive_substrate_address(context.caller);
		let to = <T as darwinia_evm::Config>::IntoAccountId::derive_substrate_address(to);

		<T as darwinia_evm::Config>::KtonAccountBasic::transfer(&origin, &to, amount)
			.map_err(|_| helper.revert("Transfer failed"))?;

		// TODO: Add log

		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			output: EvmDataWriter::new().write(true).build(),
			cost: helper.used_gas(),
			logs: vec![],
		})
	}

	fn allowance(helper: &mut PrecompileHelper<T>, input: &[u8]) -> EvmResult<PrecompileOutput> {
		// TODO: update the gas record
		helper.record_gas(1, 0)?;

		let mut reader = EvmDataReader::new_skip_selector(input)?;
		reader.expect_arguments(2)?;
		let owner: H160 = reader.read::<Address>()?.into();
		let spender: H160 = reader.read::<Address>()?.into();

		let amount: U256 = ApprovesStorage::get(owner, spender).into();

		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			output: EvmDataWriter::new().write(amount).build(),
			cost: helper.used_gas(),
			logs: vec![],
		})
	}

	fn approve(
		helper: &mut PrecompileHelper<T>,
		input: &[u8],
		context: &Context,
	) -> EvmResult<PrecompileOutput> {
		// TODO: update the gas record
		helper.record_gas(1, 0)?;

		let mut reader = EvmDataReader::new_skip_selector(input)?;
		reader.expect_arguments(2)?;
		let spender: H160 = reader.read::<Address>()?.into();
		let amount: U256 = reader.read()?;

		ApprovesStorage::insert(context.caller, spender, amount);

		// TODO: add log

		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			output: EvmDataWriter::new().write(true).build(),
			cost: helper.used_gas(),
			logs: vec![],
		})
	}

	fn transfer_from(
		helper: &mut PrecompileHelper<T>,
		input: &[u8],
		context: &Context,
	) -> EvmResult<PrecompileOutput> {
		// TODO: update the gas record
		helper.record_gas(1, 0)?;

		let mut reader = EvmDataReader::new_skip_selector(input)?;
		reader.expect_arguments(3)?;

		let caller = context.caller;
		let from: H160 = reader.read::<Address>()?.into();
		let to: H160 = reader.read::<Address>()?.into();
		let amount: U256 = reader.read()?;

		if caller != from {
			ApprovesStorage::mutate(from.clone(), caller, |value| {
				let new_value = value
					.checked_sub(amount)
					.ok_or_else(|| helper.revert("trying to spend more than allowed"))?;

				*value = new_value;
				EvmResult::Ok(())
			})?;
		}

		let from_account_id =
			<T as darwinia_evm::Config>::IntoAccountId::derive_substrate_address(from);
		let to_account_id =
			<T as darwinia_evm::Config>::IntoAccountId::derive_substrate_address(to);
		<T as darwinia_evm::Config>::KtonAccountBasic::transfer(
			&from_account_id,
			&to_account_id,
			amount,
		)
		.map_err(|_| helper.revert("Transfer failed"))?;

		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			output: EvmDataWriter::new().write(true).build(),
			cost: helper.used_gas(),
			logs: vec![],
		})
	}

	fn withdraw(
		helper: &mut PrecompileHelper<T>,
		input: &[u8],
		context: &Context,
	) -> EvmResult<PrecompileOutput> {
		// TODO: update the gas record
		helper.record_gas(1, 0)?;

		let mut reader = EvmDataReader::new_skip_selector(input)?;
		reader.expect_arguments(2)?;

		// TODO: check this value
		let to_account_id: H256 = reader.read()?;
		let amount: U256 = reader.read()?;

		let origin =
			<T as darwinia_evm::Config>::IntoAccountId::derive_substrate_address(context.caller);
		let to = <T as frame_system::Config>::AccountId::decode(&mut to_account_id.as_bytes())
			.map_err(|_| helper.revert("Invalid target address"))?;

		<T as darwinia_evm::Config>::KtonAccountBasic::transfer(&origin, &to, amount)
			.map_err(|_| helper.revert("Transfer failed"))?;

		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			output: EvmDataWriter::new().write(true).build(),
			cost: helper.used_gas(),
			logs: vec![],
		})
	}

	fn name(helper: &mut PrecompileHelper<T>) -> EvmResult<PrecompileOutput> {
		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			output: EvmDataWriter::new().write::<Bytes>("Wrapped KTON".into()).build(),
			cost: helper.used_gas(),
			logs: vec![],
		})
	}

	fn symbol(helper: &mut PrecompileHelper<T>) -> EvmResult<PrecompileOutput> {
		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			output: EvmDataWriter::new().write::<Bytes>("WKTON".into()).build(),
			cost: helper.used_gas(),
			logs: vec![],
		})
	}

	fn decimals(helper: &mut PrecompileHelper<T>) -> EvmResult<PrecompileOutput> {
		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			output: EvmDataWriter::new().write(18u8).build(),
			cost: helper.used_gas(),
			logs: vec![],
		})
	}
}
