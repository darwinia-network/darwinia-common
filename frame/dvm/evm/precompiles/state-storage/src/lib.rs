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

// #[cfg(test)]
// mod tests;

// --- core ---
use core::marker::PhantomData;
// --- crates.io ---
use ethabi::{Function, Param, ParamType, StateMutability, Token};
use evm::ExitRevert;
// --- darwinia-network ---
use darwinia_evm_precompile_utils::PrecompileHelper;
use dp_contract::abi_util::abi_encode_bytes;
// --- paritytech ---
use fp_evm::{
	Context, ExitSucceed, Precompile, PrecompileFailure, PrecompileOutput, PrecompileResult,
};
use sp_runtime::SaturatedConversion;

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
		log::debug!("bear: --- enter the state storage precompile, input {:?}", input);
		let mut helper = PrecompileHelper::<T>::new(input, target_gas);
		let (selector, data) = helper.split_input()?;
		log::debug!("bear: --- selector {:?}", selector);
		log::debug!("bear: --- data {:?}", data);
		let action = Action::from_u32(selector)?;

		// Check state modifiers
		helper.check_state_modifier(context, is_static, StateMutability::View)?;

		let output = match action {
			Action::StateGetStorage => {
				// todo: add tests
				if data.len() < 16 || !F::allow(&data[0..16]) {
					return Err(helper.revert("This state of the module has read restriction"));
				}

				log::debug!("bear: --- enter the state get storage branch");
				// Storage: FeeMarket AssignedRelayers (r:1 w:0)
				helper.record_gas(1, 0)?;

				let tokens = ethabi::decode(&[ParamType::Bytes], data)
					.map_err(|_| helper.revert("ethabi decoded error"))?;
				let key = match &tokens[0] {
					Token::Bytes(bytes) => bytes,
					_ => return Err(helper.revert("The input param error")),
				};

				log::debug!("bear: --- key {:?}", key);
				// read storage accourding to the storage key
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

#[cfg(test)]
mod tests {
	use frame_support::{StorageHasher, Twox128};
	use hex::ToHex;

	#[test]
	fn test_input() {
		let mut key = vec![0u8; 32];
		assert_eq!(
			Twox128::hash(b"Sudo"),
			[92, 13, 17, 118, 165, 104, 193, 249, 41, 68, 52, 13, 191, 237, 158, 156]
		);
		println!("Sudo str: {:?}", Twox128::hash(b"Sudo").encode_hex::<String>());
		key[0..16].copy_from_slice(&Twox128::hash(b"Sudo"));
		key[16..32].copy_from_slice(&Twox128::hash(b"Key"));
		assert_eq!(
			Twox128::hash(b"Key"),
			[83, 14, 188, 167, 3, 200, 89, 16, 231, 22, 76, 183, 209, 201, 228, 123]
		);
		println!("Key str: {:?}", Twox128::hash(b"Key").encode_hex::<String>());
		assert_eq!(
			key,
			[
				92, 13, 17, 118, 165, 104, 193, 249, 41, 68, 52, 13, 191, 237, 158, 156, 83, 14,
				188, 167, 3, 200, 89, 16, 231, 22, 76, 183, 209, 201, 228, 123
			]
		);
		println!(
			"key: {:?}",
			[
				92, 13, 17, 118, 165, 104, 193, 249, 41, 68, 52, 13, 191, 237, 158, 156, 83, 14,
				188, 167, 3, 200, 89, 16, 231, 22, 76, 183, 209, 201, 228, 123
			]
			.encode_hex::<String>()
		);

		let key_str = "5c0d1176a568c1f92944340dbfed9e9c530ebca703c85910e7164cb7d1c9e47b";
		let key_bytes = hex::decode(&key_str).unwrap();
		println!("{:?}", key_bytes);

		let a = b"15";
		println!("a {:?}", a);
		let a_hex = hex::decode("1503").unwrap();
		println!("a_hex {:?}", a_hex);
	}
}
