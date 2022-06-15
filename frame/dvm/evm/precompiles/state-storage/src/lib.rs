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
use ethabi::{ParamType, StateMutability, Token};
// --- darwinia-network ---
use darwinia_evm_precompile_utils::PrecompileHelper;
use dp_contract::abi_util::abi_encode_bytes;
// --- paritytech ---
use fp_evm::{
	Context, ExitRevert, ExitSucceed, Precompile, PrecompileFailure, PrecompileOutput,
	PrecompileResult,
};

const PALLET_PREFIX_LENGTH: usize = 16;

#[darwinia_evm_precompile_utils::selector]
enum Action {
	StateGetStorage = "state_storage(bytes)",
}

pub trait StorageFilterT {
	fn allow(prefix: &[u8]) -> bool;
}

pub struct StateStorage<T, F> {
	_marker: PhantomData<(T, F)>,
}

impl<T, F> Precompile for StateStorage<T, F>
where
	T: darwinia_evm::Config,
	F: StorageFilterT,
{
	fn execute(
		input: &[u8],
		target_gas: Option<u64>,
		context: &Context,
		is_static: bool,
	) -> PrecompileResult {
		let mut helper = PrecompileHelper::<T>::new(input, target_gas);
		let (selector, data) = helper.split_input()?;
		let action = Action::from_u32(selector)?;

		// Check state modifiers
		helper.check_state_modifier(context, is_static, StateMutability::View)?;

		let output = match action {
			Action::StateGetStorage => {
				let tokens = ethabi::decode(&[ParamType::Bytes], data)
					.map_err(|_| helper.revert("Ethabi decoded failed"))?;
				let key = match &tokens[0] {
					Token::Bytes(bytes) => bytes,
					_ => return Err(helper.revert("Ethabi decode failed")),
				};

				if key.len() < PALLET_PREFIX_LENGTH || !F::allow(&key[0..PALLET_PREFIX_LENGTH]) {
					return Err(helper.revert("Read restriction"));
				}

				// Storage: FeeMarket AssignedRelayers (r:1 w:0)
				helper.record_gas(1, 0)?;

				frame_support::storage::unhashed::get_raw(key)
			},
		};

		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			cost: helper.used_gas(),
			output: abi_encode_bytes(&output.unwrap_or_default()),
			logs: Default::default(),
		})
	}
}
