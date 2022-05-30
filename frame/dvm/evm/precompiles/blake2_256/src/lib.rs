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

extern crate core;

// --- core ---
use core::marker::PhantomData;
use evm::ExitRevert;
// --- crates.io ---
use ethabi::{ParamType, StateMutability, Token};
// --- darwinia-network ---
use darwinia_evm_precompile_utils::PrecompileHelper;
// --- paritytech ---
use fp_evm::{
	Context, ExitSucceed, Precompile, PrecompileFailure, PrecompileOutput, PrecompileResult,
};
use frame_support::StorageHasher;

#[darwinia_evm_precompile_utils::selector]
enum Action {
	Blake2_256 = "blake2_256(bytes)",
}

pub struct Blake2b<T> {
	_marker: PhantomData<T>,
}

impl<T> Precompile for Blake2b<T>
where
	T: darwinia_evm::Config,
{
	fn execute(
		input: &[u8],
		target_gas: Option<u64>,
		context: &Context,
		is_static: bool,
	) -> PrecompileResult {
		let helper = PrecompileHelper::<T>::new(input, target_gas);
		let (selector, data) = helper.split_input()?;
		let action = Action::from_u32(selector)?;

		// Check state modifiers
		helper.check_state_modifier(context, is_static, StateMutability::View)?;

		let output = match action {
			Action::Blake2_256 => {
				let tokens = ethabi::decode(&[ParamType::Bytes], data)
					.map_err(|_| helper.revert("Ethabi decoded failed"))?;
				let data = match &tokens[0] {
					Token::Bytes(bytes) => bytes,
					_ => return Err(helper.revert("Ethabi decode failed")),
				};

				frame_support::Blake2_256::hash(data.as_slice())
			},
		};

		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			cost: helper.used_gas(),
			output: output.to_vec(),
			logs: Default::default(),
		})
	}
}
