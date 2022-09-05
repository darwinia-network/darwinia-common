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

#[cfg(test)]
mod tests;

// --- core ---
use core::marker::PhantomData;
// --- darwinia-network ---
use darwinia_evm_precompile_utils::{prelude::*, revert, PrecompileHelper};
// --- paritytech ---
use fp_evm::{
	Context, ExitRevert, ExitSucceed, Precompile, PrecompileFailure, PrecompileOutput,
	PrecompileResult,
};

const PALLET_PREFIX_LENGTH: usize = 16;

#[selector]
pub enum Action {
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
		let mut helper = PrecompileHelper::<T>::new(input, target_gas, context, is_static);
		let (selector, _data) = helper.split_input()?;
		let action = Action::from_u32(selector)?;

		// Check state modifiers
		helper.check_state_modifier(StateMutability::View)?;

		let output = match action {
			Action::StateGetStorage => {
				let mut reader = helper.reader()?;
				reader.expect_arguments(1)?;
				let key: Bytes = reader.read()?;

				if key.0.len() < PALLET_PREFIX_LENGTH || !F::allow(&key.0[0..PALLET_PREFIX_LENGTH])
				{
					return Err(revert("Read restriction"));
				}

				helper.record_db_gas(1, 0)?;

				frame_support::storage::unhashed::get_raw(&key.0)
			},
		};

		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			cost: helper.used_gas(),
			output: EvmDataWriter::new()
				.write::<Bytes>(output.unwrap_or_default().as_slice().into())
				.build(),
			logs: Default::default(),
		})
	}
}
