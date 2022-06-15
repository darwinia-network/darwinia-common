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

// --- core ---
use core::marker::PhantomData;
// --- crates.io ---
use codec::Encode;
// --- darwinia-network ---
use darwinia_evm_precompile_utils::{PrecompileHelper, StateMutability};
// --- paritytech ---
use fp_evm::{
	Context, ExitRevert, ExitSucceed, Precompile, PrecompileFailure, PrecompileOutput,
	PrecompileResult,
};

#[darwinia_evm_precompile_utils::selector]
enum Action {
	BurnAndRemoteUnlock = "burn_and_remote_unlock(uint32,address,address,address,bytes,uint256)",
	TokenRegisterResponse = "token_register_response(address,address,address)",
}

pub struct EthereumBridge<T> {
	_marker: PhantomData<T>,
}

impl<T> Precompile for EthereumBridge<T>
where
	T: from_ethereum_issuing::Config,
	T::Call: Encode,
	T::Call: From<from_ethereum_issuing::Call<T>>,
{
	fn execute(
		input: &[u8],
		target_gas: Option<u64>,
		context: &Context,
		is_static: bool,
	) -> PrecompileResult {
		let mut helper = PrecompileHelper::<T>::new(input, target_gas);
		let (selector, data) = helper.split_input()?;
		let action = Action::from_u32(selector)?;

		// Check state modifiers
		helper.check_state_modifier(context, is_static, StateMutability::View)?;

		let output = match action {
			Action::BurnAndRemoteUnlock => {
				helper.record_gas(0, 0)?;

				let call: T::Call =
					from_ethereum_issuing::Call::<T>::deposit_burn_token_event_from_precompile {
						input: data.to_vec(),
					}
					.into();
				call.encode()
			},
			Action::TokenRegisterResponse => {
				helper.record_gas(0, 0)?;

				let call: T::Call =
					from_ethereum_issuing::Call::<T>::register_response_from_contract {
						input: data.to_vec(),
					}
					.into();
				call.encode()
			},
		};

		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			cost: helper.used_gas(),
			output,
			logs: Default::default(),
		})
	}
}
