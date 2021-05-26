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
use codec::Decode;
use core::marker::PhantomData;
use darwinia_evm::{AddressMapping, GasWeightMapping};
use dp_evm::Precompile;
use evm::{Context, ExitError, ExitSucceed};
use frame_support::{
	dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
	weights::{DispatchClass, Pays},
};

// s2s issuing
// todo, move issuing contract to primitives
use darwinia_ethereum_issuing_contract::TokenBurnInfo;

/// The contract address: 0000000000000000000000000000000000000018
pub struct DispatchWrapper<T> {
	_marker: PhantomData<T>,
}

impl<T> Precompile for DispatchWrapper<T>
where
    T: darwinia_evm::Config + darwinia_s2s_issuing::Config,
    T::Call: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo + Decode,
    <T::Call as Dispatchable>::Origin: From<Option<T::AccountId>>,
    T::Call: From<darwinia_s2s_issuing::Call<T>>,
{
	fn execute(
		input: &[u8],
		target_gas: Option<u64>,
		context: &Context,
	) -> core::result::Result<(ExitSucceed, Vec<u8>, u64), ExitError> {
        const SELECTOR_SIZE_BYTES: usize = 4;

        if input.len() < 4 {
			return Err(ExitError::Other("input length less than 4 bytes".into()));
		}

        let inner_call = match input[0..SELECTOR_SIZE_BYTES] {
            [0x67, 0x74, 0x14, 0x8c] => Self::s2sissuing_cross_send(&input[SELECTOR_SIZE_BYTES..])?,
            _ => {
                return Err(ExitError::Other(
                        "No wrapper method at selector given selector".into(),
                        ));
            }
        };
        let call: T::Call = inner_call.into();
		let info = call.get_dispatch_info();

		let valid_call = info.pays_fee == Pays::Yes && info.class == DispatchClass::Normal;
		if !valid_call {
			return Err(ExitError::Other("invalid call".into()));
		}

		if let Some(gas) = target_gas {
			let valid_weight = info.weight <= T::GasWeightMapping::gas_to_weight(gas);
			if !valid_weight {
				return Err(ExitError::OutOfGas);
			}
		}

		let origin = T::AddressMapping::into_account_id(context.caller);

		match call.dispatch(Some(origin).into()) {
			Ok(post_info) => {
				let cost = T::GasWeightMapping::weight_to_gas(
					post_info.actual_weight.unwrap_or(info.weight),
				);
				Ok((ExitSucceed::Stopped, Default::default(), cost))
			}
			Err(_) => Err(ExitError::Other("dispatch execution failed".into())),
		}
	}
}

impl<T> DispatchWrapper<T>
where
	T: darwinia_s2s_issuing::Config,
	T::Call: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo + Decode,
	<T::Call as Dispatchable>::Origin: From<Option<T::AccountId>>,
    T::Call: From<darwinia_s2s_issuing::Call<T>>,
{
    fn s2sissuing_cross_send(input: &[u8]) -> Result<darwinia_s2s_issuing::Call<T>, ExitError> {
        let burn_info = TokenBurnInfo::decode(input)
            .map_err(|_| ExitError::Other("decode burninfo failed".into()))?;

		Ok(darwinia_s2s_issuing::Call::<T>::cross_send(
			burn_info.backing,
			burn_info.source,
			burn_info.recipient,
            burn_info.amount,
		))
	}
}
