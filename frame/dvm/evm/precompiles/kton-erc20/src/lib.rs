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
use darwinia_evm_precompile_utils::{prelude::*, PrecompileHelper};
use dp_contract::abi_util::abi_encode_bytes;
// --- paritytech ---
use fp_evm::{
	Context, ExitRevert, ExitSucceed, Precompile, PrecompileFailure, PrecompileOutput,
	PrecompileResult,
};
use sp_core::{H160, U256};

type BalanceOf<T> = <T as darwinia_balances::Config>::Balance;

#[darwinia_evm_precompile_utils::selector]
enum Action {
	TotalSupply = "totalSupply()",
	BalanceOf = "balanceOf(address)",
	Transfer = "transfer(address,uint256)",
	Allowance = "allowance(address,address)",
	Approve = "approve(address,uint256)",
	TransferFrom = "transferFrom(address,address,uint256)",
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
			Action::BalanceOf => Self::balance_of(&mut helper, data),
			Action::Transfer => Self::transfer(&mut helper, data),
			Action::Allowance => Self::allowance(&mut helper, data),
			Action::Approve => Self::approve(&mut helper, data),
			Action::TransferFrom => Self::transfer_from(&mut helper, data),
		}
	}
}

impl<T> KtonErc20<T>
where
	T: darwinia_evm::Config + darwinia_balances::Config,
	BalanceOf<T>: Into<U256>,
{
	fn total_supply(helper: &mut PrecompileHelper<T>) -> EvmResult<PrecompileOutput> {
		helper.record_gas(1, 0);

		let amount: U256 = darwinia_balances::Pallet::<T>::total_issuance().into();

		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			output: EvmDataWriter::new().write(amount).build(),
			cost: helper.used_gas(),
			logs: vec![],
		})
	}

	fn balance_of(helper: &mut PrecompileHelper<T>, input: &[u8]) -> EvmResult<PrecompileOutput> {
		let mut reader = EvmDataReader::new_skip_selector(input)?;
		reader.expect_arguments(1)?;
		let owner: H160 = reader.read::<Address>()?.into();

		todo!()
	}

	fn transfer(helper: &mut PrecompileHelper<T>, input: &[u8]) -> EvmResult<PrecompileOutput> {
		let mut reader = EvmDataReader::new_skip_selector(input)?;
		reader.expect_arguments(2)?;

		let to: H160 = reader.read::<Address>()?.into();
		let amount: U256 = reader.read()?;

		todo!();
	}

	fn allowance(helper: &mut PrecompileHelper<T>, input: &[u8]) -> EvmResult<PrecompileOutput> {
		let mut reader = EvmDataReader::new_skip_selector(input)?;
		reader.expect_arguments(2)?;

		let owner: H160 = reader.read::<Address>()?.into();
		let spender: H160 = reader.read::<Address>()?.into();

		todo!();
	}

	fn approve(helper: &mut PrecompileHelper<T>, input: &[u8]) -> EvmResult<PrecompileOutput> {
		let mut reader = EvmDataReader::new_skip_selector(input)?;
		reader.expect_arguments(2)?;

		let spender: H160 = reader.read::<Address>()?.into();
		let amount: U256 = reader.read()?;

		todo!();
	}

	fn transfer_from(
		helper: &mut PrecompileHelper<T>,
		input: &[u8],
	) -> EvmResult<PrecompileOutput> {
		let mut reader = EvmDataReader::new_skip_selector(input)?;
		reader.expect_arguments(3)?;

		let from: H160 = reader.read::<Address>()?.into();
		let to: H160 = reader.read::<Address>()?.into();
		let amount: U256 = reader.read()?;

		todo!();
	}
}
