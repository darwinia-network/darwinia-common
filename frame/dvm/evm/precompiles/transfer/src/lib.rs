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

// --- core ---
use core::marker::PhantomData;
// --- paritytech ---
use fp_evm::{Context, ExitRevert, Precompile, PrecompileFailure, PrecompileResult};
// --- darwinia-network ---
use darwinia_evm_precompile_utils::{prelude::*, revert, PrecompileHelper};

#[selector]
#[derive(Eq, PartialEq)]
pub enum Action {
	TransferAndCall = "transfer_and_call(address,uint256)",
	Withdraw = "withdraw(bytes32,uint256)",
}

/// Transfer Precompile Contract, only for the exchange of KTON now.
pub struct Transfer<T>(PhantomData<T>);
impl<T: darwinia_evm::Config> Precompile for Transfer<T> {
	fn execute(
		input: &[u8],
		target_gas: Option<u64>,
		context: &Context,
		is_static: bool,
	) -> PrecompileResult {
		let helper = PrecompileHelper::<T>::new(input, target_gas, context, is_static);
		let (selector, _) = helper.split_input()?;
		let action = Action::from_u32(selector)?;

		// Check state modifiers
		helper.check_state_modifier(StateMutability::NonPayable)?;

		match action {
			Action::TransferAndCall => Err(revert("This precompile is deprecated")),
			Action::Withdraw => Err(revert("This precompile is deprecated")),
		}
	}
}
