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

// --- core ---
use core::marker::PhantomData;
// --- crates ---
use codec::Encode;
use evm::{executor::PrecompileOutput, Context, ExitError, ExitSucceed};
// --- darwinia ---
use darwinia_support::evm::SELECTOR;
use dp_evm::Precompile;
// --- paritytech ---
use frame_support::dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo};

/// The contract address: 0000000000000000000000000000000000000018
pub struct DispatchCallEncoder<T> {
	_marker: PhantomData<T>,
}
impl<T> Precompile for DispatchCallEncoder<T>
where
	T: darwinia_ethereum_issuing::Config,
	T: darwinia_s2s_issuing::Config,
	T::Call: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo + Encode,
	<T::Call as Dispatchable>::Origin: From<Option<T::AccountId>>,
	T::Call: From<darwinia_s2s_issuing::Call<T>>,
	T::Call: From<darwinia_ethereum_issuing::Call<T>>,
{
	fn execute(
		input: &[u8],
		_target_gas: Option<u64>,
		_context: &Context,
	) -> core::result::Result<PrecompileOutput, ExitError> {
		if input.len() < SELECTOR {
			return Err(ExitError::Other("input length less than 4 bytes".into()));
		}
		let selector = &input[0..SELECTOR];
		let call: T::Call = match selector {
			_ if selector == <darwinia_s2s_issuing::Pallet<T>>::digest() => {
				darwinia_s2s_issuing::Call::<T>::asset_burn_event_handle(input.to_vec()).into()
			}
			_ if selector == <darwinia_ethereum_issuing::Pallet<T>>::digest() => {
				darwinia_ethereum_issuing::Call::<T>::asset_burn_event_handle(input.to_vec()).into()
			}
			_ => {
				return Err(ExitError::Other(
					"No wrapper method at selector given selector".into(),
				));
			}
		};
		// TODO: The cost should not be zero
		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Stopped,
			cost: 0,
			output: call.encode(),
			logs: Default::default(),
		})
	}
}
