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

use frame_support::traits::{Currency, ExistenceRequirement};
use sp_core::U256;
use sp_runtime::traits::UniqueSaturatedInto;
use sp_std::marker::PhantomData;
use sp_std::prelude::*;
use sp_std::vec::Vec;

use codec::Decode;
use darwinia_evm::{AddressMapping, Trait};
use darwinia_evm_primitives::Precompile;
use evm::{Context, ExitError, ExitSucceed};

type AccountId<T> = <T as frame_system::Trait>::AccountId;

/// Kton Precompile Contract
///
/// The contract address: 0000000000000000000000000000000000000016
pub struct Kton<T: Trait> {
	_maker: PhantomData<T>,
}

impl<T: Trait> Precompile for Kton<T> {
	fn execute(
		input: &[u8],
		_: Option<usize>,
		context: &Context,
	) -> core::result::Result<(ExitSucceed, Vec<u8>, usize), ExitError> {
        Ok((ExitSucceed::Returned, vec![], 10000))
	}
}