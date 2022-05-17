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

#[cfg(feature = "testing")]
pub mod test_helper;

pub use darwinia_evm_precompile_utils_macro::selector;
pub use ethabi::StateMutability;

// --- crates.io ---
use ethabi::{Function, Param, ParamType, Token};
use evm::ExitRevert;
// --- darwinia-network ---
use darwinia_evm::GasWeightMapping;
use darwinia_support::evm::SELECTOR;
// --- paritytech ---
use fp_evm::{Context, ExitError, PrecompileFailure};
use frame_support::traits::Get;
use sp_core::U256;
use sp_std::{borrow::ToOwned, marker::PhantomData, vec};

#[derive(Clone, Copy, Debug)]
pub struct PrecompileHelper<'a, T> {
	input: &'a [u8],
	target_gas: Option<u64>,
	used_gas: u64,
	_marker: PhantomData<T>,
}

impl<'a, T: darwinia_evm::Config> PrecompileHelper<'a, T> {
	pub fn new(input: &'a [u8], target_gas: Option<u64>) -> Self {
		Self { input, target_gas, used_gas: 0, _marker: PhantomData }
	}

	pub fn split_input(&self) -> Result<(u32, &'a [u8]), PrecompileFailure> {
		if self.input.len() < SELECTOR {
			return Err(self.revert("input length less than 4 bytes"));
		}

		let mut buffer = [0u8; SELECTOR];
		buffer.copy_from_slice(&self.input[0..SELECTOR]);
		let selector = u32::from_be_bytes(buffer);
		Ok((selector, &self.input[SELECTOR..]))
	}

	/// Check that a function call is compatible with the context it is
	/// called into.
	pub fn check_state_modifier(
		&self,
		context: &Context,
		is_static: bool,
		modifier: StateMutability,
	) -> Result<(), PrecompileFailure> {
		if is_static && modifier != StateMutability::View {
			return Err(self.revert("can't call non-static function in static context"));
		}

		if modifier != StateMutability::Payable && context.apparent_value > U256::zero() {
			return Err(self.revert("function is not payable"));
		}

		Ok(())
	}

	pub fn record_gas(&mut self, reads: u64, writes: u64) -> Result<(), PrecompileFailure> {
		let reads_cost = <T as darwinia_evm::Config>::GasWeightMapping::weight_to_gas(
			<T as frame_system::Config>::DbWeight::get().read,
		)
		.checked_mul(reads)
		.ok_or(self.revert("Cost Overflow"))?;
		let writes_cost = <T as darwinia_evm::Config>::GasWeightMapping::weight_to_gas(
			<T as frame_system::Config>::DbWeight::get().write,
		)
		.checked_mul(writes)
		.ok_or(self.revert("Cost Overflow"))?;
		let cost = reads_cost.checked_add(writes_cost).ok_or(self.revert("Cost Overflow"))?;

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

	pub fn used_gas(&self) -> u64 {
		self.used_gas
	}

	/// Revert the execution, making the user pay for the the currently
	/// recorded cost. It is better to **revert** instead of **error** as
	/// erroring consumes the entire gas limit, and **revert** returns an error
	/// message to the calling contract.
	pub fn revert(&self, message: &'static str) -> PrecompileFailure {
		#[allow(deprecated)]
		let func = Function {
			name: "Error".to_owned(),
			inputs: vec![Param {
				name: "error_message".to_owned(),
				kind: ParamType::String,
				internal_type: None,
			}],
			outputs: vec![],
			constant: false,
			state_mutability: StateMutability::NonPayable,
		};

		PrecompileFailure::Revert {
			exit_status: ExitRevert::Reverted,
			output: func.encode_input(&[Token::String(message.to_owned())]).unwrap_or_default(),
			cost: self.used_gas,
		}
	}
}
