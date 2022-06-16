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

#[cfg(test)]
mod tests;

pub mod kton;
pub mod ring;
pub mod util;

// --- paritytech ---
use fp_evm::{Context, Precompile, PrecompileResult};
use sp_std::marker::PhantomData;
// --- darwinia-network ---
use darwinia_evm_precompile_utils::PrecompileHelper;
use kton::{Action, Kton};
use ring::RingBack;

/// Transfer Precompile Contract, used to support the exchange of KTON and RING transfer.
pub enum Transfer<T> {
	/// Transfer RING back from DVM to Darwinia
	RingTransfer,
	/// Transfer KTON between Darwinia and DVM contract
	KtonTransfer,
	_Impossible(PhantomData<T>),
}
impl<T: darwinia_ethereum::Config> Precompile for Transfer<T> {
	fn execute(
		input: &[u8],
		target_gas: Option<u64>,
		context: &Context,
		is_static: bool,
	) -> PrecompileResult {
		let helper = PrecompileHelper::<T>::new(input, target_gas);
		let (selector, _data) = helper.split_input()?;

		match Action::from_u32(selector) {
			Ok(Action::TransferAndCall) | Ok(Action::Withdraw) =>
				<Kton<T>>::transfer(&input, target_gas, context, is_static),
			_ => <RingBack<T>>::transfer(&input, target_gas, context, is_static),
		}
	}
}
