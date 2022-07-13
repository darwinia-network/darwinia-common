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

pub mod data;
pub mod log;

#[cfg(feature = "testing")]
pub mod test_helper;
#[cfg(test)]
pub mod tests;

pub use darwinia_evm_precompile_utils_macro::selector;
pub use ethabi::StateMutability;

// --- crates.io ---
use sha3::{Digest, Keccak256};
// --- darwinia-network ---
use crate::prelude::*;
use darwinia_evm::GasWeightMapping;
// --- paritytech ---
use fp_evm::{Context, ExitError, ExitRevert, PrecompileFailure};
use frame_support::traits::Get;
use sp_core::U256;
use sp_std::marker::PhantomData;

/// Alias for Result returning an EVM precompile error.
pub type EvmResult<T = ()> = Result<T, PrecompileFailure>;

#[derive(Clone, Copy, Debug)]
pub struct PrecompileHelper<'a, T> {
	input: &'a [u8],
	target_gas: Option<u64>,
	context: &'a Context,
	is_static: bool,
	used_gas: u64,
	_marker: PhantomData<T>,
}

impl<'a, T: darwinia_evm::Config> PrecompileHelper<'a, T> {
	pub fn new(
		input: &'a [u8],
		target_gas: Option<u64>,
		context: &'a Context,
		is_static: bool,
	) -> Self {
		Self { input, target_gas, context, is_static, used_gas: 0, _marker: PhantomData }
	}

	// FIXME: Replace this with selector and data reader in the next prs.
	pub fn split_input(&self) -> Result<(u32, &'a [u8]), PrecompileFailure> {
		if self.input.len() < 4 {
			return Err(revert("input length less than 4 bytes"));
		}

		let mut buffer = [0u8; 4];
		buffer.copy_from_slice(&self.input[0..4]);
		let selector = u32::from_be_bytes(buffer);
		Ok((selector, &self.input[4..]))
	}

	pub fn selector<U>(&self) -> EvmResult<U>
	where
		U: num_enum::TryFromPrimitive<Primitive = u32>,
	{
		EvmDataReader::read_selector(self.input)
	}

	pub fn reader(&self) -> EvmResult<EvmDataReader> {
		EvmDataReader::new_skip_selector(self.input)
	}

	/// Check that a function call is compatible with the context it is
	/// called into.
	pub fn check_state_modifier(&self, modifier: StateMutability) -> EvmResult<()> {
		if self.is_static && modifier != StateMutability::View {
			return Err(revert("can't call non-static function in static context"));
		}

		if modifier != StateMutability::Payable && self.context.apparent_value > U256::zero() {
			return Err(revert("function is not payable"));
		}

		Ok(())
	}

	pub fn record_db_gas(&mut self, reads: u64, writes: u64) -> EvmResult<()> {
		let reads_cost = <T as darwinia_evm::Config>::GasWeightMapping::weight_to_gas(
			<T as frame_system::Config>::DbWeight::get().read,
		)
		.checked_mul(reads)
		.ok_or(revert("Cost Overflow"))?;
		let writes_cost = <T as darwinia_evm::Config>::GasWeightMapping::weight_to_gas(
			<T as frame_system::Config>::DbWeight::get().write,
		)
		.checked_mul(writes)
		.ok_or(revert("Cost Overflow"))?;
		let cost = reads_cost.checked_add(writes_cost).ok_or(revert("Cost Overflow"))?;

		self.used_gas = self
			.used_gas
			.checked_add(cost)
			.ok_or(PrecompileFailure::Error { exit_status: ExitError::OutOfGas })?;

		match self.target_gas {
			Some(gas_limit) if self.used_gas > gas_limit =>
				Err(PrecompileFailure::Error { exit_status: ExitError::OutOfGas }),
			_ => Ok(()),
		}
	}

	pub fn record_log_gas(&mut self, topics: usize, data_len: usize) -> EvmResult<()> {
		let log_costs = log::log_costs(topics, data_len)?;
		self.used_gas = self
			.used_gas
			.checked_add(log_costs)
			.ok_or(PrecompileFailure::Error { exit_status: ExitError::OutOfGas })?;

		match self.target_gas {
			Some(gas_limit) if self.used_gas > gas_limit =>
				Err(PrecompileFailure::Error { exit_status: ExitError::OutOfGas }),
			_ => Ok(()),
		}
	}

	pub fn used_gas(&self) -> u64 {
		self.used_gas
	}
}

/// Revert the execution, making the user pay for the the currently
/// recorded cost. It is better to **revert** instead of **error** as
/// erroring consumes the entire gas limit, and **revert** returns an error
/// message to the calling contract.
pub fn revert(message: impl AsRef<[u8]>) -> PrecompileFailure {
	let selector =
		u32::from_be_bytes(Keccak256::digest(b"Error(string)")[0..4].try_into().unwrap());

	PrecompileFailure::Revert {
		exit_status: ExitRevert::Reverted,
		output: EvmDataWriter::new_with_selector(selector)
			.write::<Bytes>(Bytes(message.as_ref().to_vec()))
			.build(),
		cost: 0,
	}
}

pub mod prelude {
	pub use crate::{
		data::{Address, Bytes, EvmData, EvmDataReader, EvmDataWriter},
		log::{log0, log1, log2, log3},
		revert, EvmResult,
	};
	pub use darwinia_evm_precompile_utils_macro::{keccak256, selector};
}
