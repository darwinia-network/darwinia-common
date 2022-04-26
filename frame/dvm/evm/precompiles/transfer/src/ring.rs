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

// --- paritytech ---
use fp_evm::{Context, ExitSucceed, PrecompileFailure, PrecompileOutput, PrecompileResult};
use frame_support::ensure;
use sp_std::{marker::PhantomData, prelude::*};
// --- darwinia-network ---
use darwinia_evm::{AccountBasic, AccountId};
use darwinia_evm_precompile_utils::{PrecompileHelper, StateMutability};
use darwinia_support::evm::{IntoAccountId, TRANSFER_ADDR};
// --- crates.io ---
use codec::Decode;

pub struct RingBack<T> {
	_maker: PhantomData<T>,
}

impl<T: darwinia_ethereum::Config> RingBack<T> {
	/// The Withdraw process is divided into two part:
	/// 1. parse the withdrawal address from the input parameter and get the contract address and
	/// value from the context 2. transfer from the contract address to withdrawal address
	///
	/// Input data: 32-bit substrate withdrawal public key
	pub fn transfer(
		input: &[u8],
		target_gas: Option<u64>,
		context: &Context,
		is_static: bool,
	) -> PrecompileResult {
		let mut helper = PrecompileHelper::<T>::new(input, target_gas);
		// Check state modifiers
		helper.check_state_modifier(context, is_static, StateMutability::Payable)?;

		// Storage: System Account (r:2 w:2)
		// Storage: Ethereum RemainingRingBalance (r:2 w:2)
		helper.record_gas(4, 4)?;

		// Decode input data
		let input = InputData::<T>::decode(&input, &helper)?;
		let (address, to, value) = (context.address, input.dest, context.apparent_value);

		// Ensure the context address should be precompile address
		let transfer_addr = array_bytes::hex_try_into(TRANSFER_ADDR)
			.map_err(|_| helper.revert("invalid address"))?;
		ensure!(
			address == transfer_addr,
			helper.revert("Invalid context address")
		);

		let source = <T as darwinia_evm::Config>::IntoAccountId::into_account_id(address);
		T::RingAccountBasic::transfer(&source, &to, value)
			.map_err(|e| PrecompileFailure::Error { exit_status: e })?;

		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			cost: helper.used_gas(),
			output: Default::default(),
			logs: Default::default(),
		})
	}
}

#[derive(Debug, PartialEq, Eq)]
pub struct InputData<T: darwinia_evm::Config> {
	pub dest: AccountId<T>,
}

impl<T: darwinia_evm::Config> InputData<T> {
	pub fn decode(data: &[u8], helper: &PrecompileHelper<T>) -> Result<Self, PrecompileFailure> {
		if data.len() == 32 {
			let mut dest_bytes = [0u8; 32];
			dest_bytes.copy_from_slice(&data[0..32]);

			return Ok(InputData {
				dest: <T as frame_system::Config>::AccountId::decode(&mut dest_bytes.as_ref())
					.map_err(|_| helper.revert("Invalid destination address"))?,
			});
		}
		Err(helper.revert("Invalid input data length"))
	}
}
