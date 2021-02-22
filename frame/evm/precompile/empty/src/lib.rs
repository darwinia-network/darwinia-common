// This file is part of Darwinia.
//
// Copyright (C) 2018-2021 Darwinia Network
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
extern crate alloc;

use alloc::vec::Vec;
use darwinia_evm_primitives::LinearCostPrecompile;
use evm::{ExitError, ExitSucceed};

/// The empty precompile.
pub struct Empty;

impl LinearCostPrecompile for Empty {
	const BASE: usize = 0;
	const WORD: usize = 0;

	fn execute(_: &[u8], _: usize) -> core::result::Result<(ExitSucceed, Vec<u8>), ExitError> {
		Err(ExitError::Other("Not implement yet".into()))
	}
}
