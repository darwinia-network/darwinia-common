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

pub mod kton;
pub mod ring;
pub mod util;

// --- paritytech ---
use fp_evm::{Context, ExitError, Precompile, PrecompileOutput};
use sp_std::marker::PhantomData;
// --- darwinia-network ---
use darwinia_evm::Config;
use darwinia_support::{evm::SELECTOR, AccountId};
use kton::Kton;
use ring::RingBack;

/// Transfer Precompile Contract, used to support the exchange of KTON and RING transfer.
pub struct Transfer<Runtime, RingAccountBasic, KtonAccountBasic>(
	PhantomData<(Runtime, RingAccountBasic, KtonAccountBasic)>,
);
impl<Runtime, RingAccountBasic, KtonAccountBasic> Precompile
	for Transfer<Runtime, RingAccountBasic, KtonAccountBasic>
where
	Runtime: darwinia_evm::Config,
	RingAccountBasic: darwinia_evm::AccountBasic<Runtime>,
	KtonAccountBasic: darwinia_evm::AccountBasic<Runtime>,
{
	fn execute(
		input: &[u8],
		target_gas: Option<u64>,
		context: &Context,
	) -> core::result::Result<PrecompileOutput, ExitError> {
		if input.len() < SELECTOR {
			return Err(ExitError::Other("Invalid input data！".into()));
		}

		if kton::is_kton_transfer(input) {
			<RingBack<Runtime, RingAccountBasic>>::transfer(&input, target_gas, context)
		} else {
			<Kton<Runtime, KtonAccountBasic>>::transfer(&input, target_gas, context)
		}
	}
}
