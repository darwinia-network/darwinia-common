// SPDX-License-Identifier: Apache-2.0
// This file is part of Frontier.
//
// Copyright (c) 2020 Parity Technologies (UK) Ltd.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;
use codec::{Decode, Encode};
use core::marker::PhantomData;
use darwinia_relay_primitives::Relay;
use dp_evm::Precompile;
use evm::{Context, ExitError, ExitSucceed};
use frame_support::dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo};

/// The contract address: 0000000000000000000000000000000000000018
pub struct Util<T> {
	_marker: PhantomData<T>,
}

const SELECTOR_SIZE_BYTES: usize = 4;
impl<T> Precompile for Util<T>
where
	T: darwinia_s2s_issuing::Config,
	T::Call: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo + Encode,
	<T::Call as Dispatchable>::Origin: From<Option<T::AccountId>>,
	T::Call: From<darwinia_s2s_issuing::Call<T>>,
{
	fn execute(
		input: &[u8],
		_target_gas: Option<u64>,
		_context: &Context,
	) -> core::result::Result<(ExitSucceed, Vec<u8>, u64), ExitError> {
		if input.len() < SELECTOR_SIZE_BYTES {
			return Err(ExitError::Other("input length less than 4 bytes".into()));
		}
		let selector = &input[0..SELECTOR_SIZE_BYTES];
		let inner_call = match selector {
			_ if selector == <T as darwinia_s2s_issuing::Config>::BackingRelay::digest() => {
				darwinia_s2s_issuing::Call::<T>::dispatch_handle(input.to_vec())
			}
			_ => {
				return Err(ExitError::Other(
					"No wrapper method at selector given selector".into(),
				));
			}
		};
		let call: T::Call = inner_call.into();
		Ok((ExitSucceed::Stopped, call.encode(), 0))
	}
}
