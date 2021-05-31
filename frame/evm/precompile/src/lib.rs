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

pub use darwinia_evm_precompile_issuing::Issuing;
pub use darwinia_evm_precompile_kton::Kton;
pub use darwinia_evm_precompile_simple::{ECRecover, Identity, Ripemd160, Sha256};
pub use darwinia_evm_precompile_withdraw::WithDraw;

use dp_evm::{Precompile, PrecompileSet};
use evm::{Context, ExitError, ExitSucceed};
use sp_core::H160;
use sp_std::{marker::PhantomData, vec::Vec};

pub struct PangolinPrecompiles<R>(PhantomData<R>);

impl<R: dvm_ethereum::Config> PrecompileSet for PangolinPrecompiles<R> {
	fn execute(
		address: H160,
		input: &[u8],
		target_gas: Option<u64>,
		context: &Context,
	) -> Option<core::result::Result<(ExitSucceed, Vec<u8>, u64), ExitError>> {
		match address {
			// Ethereum precompiles
			a if a == to_address(1) => Some(ECRecover::execute(input, target_gas, context)),
			a if a == to_address(2) => Some(Sha256::execute(input, target_gas, context)),
			a if a == to_address(3) => Some(Ripemd160::execute(input, target_gas, context)),
			a if a == to_address(4) => Some(Identity::execute(input, target_gas, context)),
			// Darwinia precompiles
			a if a == to_address(21) => Some(WithDraw::<R>::execute(input, target_gas, context)),
			a if a == to_address(22) => Some(Kton::<R>::execute(input, target_gas, context)),
			a if a == to_address(23) => Some(Issuing::<R>::execute(input, target_gas, context)),
			_ => None,
		}
	}
}

fn to_address(a: u64) -> H160 {
	H160::from_low_u64_be(a)
}
