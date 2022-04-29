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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests;

// --- core ---
use core::marker::PhantomData;
// --- crates.io ---
use evm::ExitRevert;
// --- darwinia-network ---
use darwinia_evm_precompile_utils::{PrecompileHelper, StateMutability};
use dp_contract::abi_util::abi_encode_u64;
// --- paritytech ---
use fp_evm::{
	Context, ExitSucceed, Precompile, PrecompileFailure, PrecompileOutput, PrecompileResult,
};
use sp_runtime::SaturatedConversion;

#[darwinia_evm_precompile_utils::selector]
enum Action {
	MarketFee = "market_fee()",
}

pub struct FeeMarket<T, I> {
	_marker: PhantomData<(T, I)>,
}

impl<T, I> Precompile for FeeMarket<T, I>
where
	I: 'static,
	T: darwinia_evm::Config + darwinia_fee_market::Config<I>,
{
	fn execute(
		input: &[u8],
		target_gas: Option<u64>,
		context: &Context,
		is_static: bool,
	) -> PrecompileResult {
		let mut helper = PrecompileHelper::<T>::new(input, target_gas);
		let (selector, _data) = helper.split_input()?;
		let action = Action::from_u32(selector)?;

		// Check state modifiers
		helper.check_state_modifier(context, is_static, StateMutability::View)?;

		let output = match action {
			Action::MarketFee => {
				// Storage: FeeMarket AssignedRelayers (r:1 w:0)
				helper.record_gas(1, 0)?;

				darwinia_fee_market::Pallet::<T, I>::market_fee()
					.map_or(0, |f| f.saturated_into::<u64>())
			},
		};

		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			cost: helper.used_gas(),
			output: abi_encode_u64(output),
			logs: Default::default(),
		})
	}
}
