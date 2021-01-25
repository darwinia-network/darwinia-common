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

use evm::{Context, ExitError, ExitSucceed};
use impl_trait_for_tuples::impl_for_tuples;
use sp_core::H160;
use sp_std::vec::Vec;

/// Custom precompiles to be used by EVM engine.
pub trait PrecompileSet {
	/// Try to execute the code address as precompile. If the code address is not
	/// a precompile or the precompile is not yet available, return `None`.
	/// Otherwise, calculate the amount of gas needed with given `input` and
	/// `target_gas`. Return `Some(Ok(status, output, gas_used))` if the execution
	/// is successful. Otherwise return `Some(Err(_))`.
	fn execute(
		address: H160,
		input: &[u8],
		target_gas: Option<usize>,
		context: &Context,
	) -> Option<core::result::Result<(ExitSucceed, Vec<u8>, usize), ExitError>>;
}

/// One single precompile used by EVM engine.
pub trait Precompile {
	/// Try to execute the precompile. Calculate the amount of gas needed with given `input` and
	/// `target_gas`. Return `Ok(status, output, gas_used)` if the execution is
	/// successful. Otherwise return `Err(_)`.
	fn execute(
		input: &[u8],
		target_gas: Option<usize>,
		context: &Context,
	) -> core::result::Result<(ExitSucceed, Vec<u8>, usize), ExitError>;
}

#[impl_for_tuples(16)]
#[tuple_types_no_default_trait_bound]
impl PrecompileSet for Tuple {
	for_tuples!( where #( Tuple: Precompile )* );

	fn execute(
		address: H160,
		input: &[u8],
		target_gas: Option<usize>,
		context: &Context,
	) -> Option<core::result::Result<(ExitSucceed, Vec<u8>, usize), ExitError>> {
		let mut index = 0;

		for_tuples!( #(
			index += 1;
			if address == H160::from_low_u64_be(index) {
				return Some(Tuple::execute(input, target_gas, context))
			}
		)* );

		None
	}
}

pub trait LinearCostPrecompile {
	const BASE: usize;
	const WORD: usize;

	fn execute(
		input: &[u8],
		cost: usize,
	) -> core::result::Result<(ExitSucceed, Vec<u8>), ExitError>;
}

impl<T: LinearCostPrecompile> Precompile for T {
	fn execute(
		input: &[u8],
		target_gas: Option<usize>,
		_: &Context,
	) -> core::result::Result<(ExitSucceed, Vec<u8>, usize), ExitError> {
		let cost = ensure_linear_cost(target_gas, input.len(), T::BASE, T::WORD)?;

		let (succeed, out) = T::execute(input, cost)?;
		Ok((succeed, out, cost))
	}
}

/// Linear gas cost
fn ensure_linear_cost(
	target_gas: Option<usize>,
	len: usize,
	base: usize,
	word: usize,
) -> Result<usize, ExitError> {
	let cost = base
		.checked_add(
			word.checked_mul(len.saturating_add(31) / 32)
				.ok_or(ExitError::OutOfGas)?,
		)
		.ok_or(ExitError::OutOfGas)?;

	if let Some(target_gas) = target_gas {
		if cost > target_gas {
			return Err(ExitError::OutOfGas);
		}
	}

	Ok(cost)
}
