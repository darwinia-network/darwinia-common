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

pub use darwinia_evm_precompile_utils_macro::selector;
use darwinia_support::evm::SELECTOR;
use evm::ExitError;
use fp_evm::PrecompileFailure;

#[derive(Clone, Copy, Debug)]
pub struct DvmInputParser<'a> {
	pub input: &'a [u8],
	pub selector: u32,
}

impl<'a> DvmInputParser<'a> {
	pub fn new(input: &'a [u8]) -> Result<Self, PrecompileFailure> {
		if input.len() < SELECTOR {
			return Err(PrecompileFailure::Error {
				exit_status: ExitError::Other("input length less than 4 bytes".into()),
			});
		}

		let mut buffer = [0u8; SELECTOR];
		buffer.copy_from_slice(&input[0..SELECTOR]);
		let selector = u32::from_be_bytes(buffer);
		Ok(Self {
			input: &input[SELECTOR..],
			selector,
		})
	}
}
