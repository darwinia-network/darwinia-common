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

extern crate alloc;
use alloc::vec::Vec;

// --- core ---
use core::marker::PhantomData;
// --- crates.io ---
use codec::Encode;
use evm::{executor::PrecompileOutput, Context, ExitError, ExitSucceed};
use sha3::Digest;
// --- darwinia-network ---
use darwinia_support::s2s::RelayMessageCaller;
use dp_evm::Precompile;
// --- paritytech ---
use frame_support::dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo};

const PALLET_DIG_LEN: usize = 4;
const METHOD_DIG_LEN: usize = 4;
const ACTION_LEN: usize = PALLET_DIG_LEN + METHOD_DIG_LEN;
const BURN_AND_REMOTE_UNLOCK_METHOD: &[u8] = b"burn_and_remote_unlock()";
const TOKEN_REGISTER_RESPONSE_METHOD: &[u8] = b"token_register_response()";
const READ_LATEST_MESSAGE_ID_METHOD: &[u8] = b"read_latest_message_id()";

// TODO rename this precompile contract
/// The contract address: 0000000000000000000000000000000000000018
pub struct DispatchCallEncoder<T> {
	_marker: PhantomData<T>,
}
impl<T> Precompile for DispatchCallEncoder<T>
where
	T: from_ethereum_issuing::Config,
	T: from_substrate_issuing::Config,
	T::Call: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo + Encode,
	<T::Call as Dispatchable>::Origin: From<Option<T::AccountId>>,
	T::Call: From<from_substrate_issuing::Call<T>>,
	T::Call: From<from_ethereum_issuing::Call<T>>,
{
	fn execute(
		input: &[u8],
		_target_gas: Option<u64>,
		_context: &Context,
	) -> core::result::Result<PrecompileOutput, ExitError> {
		if input.len() < ACTION_LEN {
			return Err(ExitError::Other("input length less than 4 bytes".into()));
		}
		let pallet_digest = &input[0..PALLET_DIG_LEN];
		let method_digest = &input[PALLET_DIG_LEN - 1..ACTION_LEN];
		let output = match pallet_digest {
			_ if pallet_digest == <from_substrate_issuing::Pallet<T>>::digest() => {
				match method_digest {
					_ if method_digest == &sha3::Keccak256::digest(BURN_AND_REMOTE_UNLOCK_METHOD)[..METHOD_DIG_LEN] => {
						let call: T::Call =
							from_substrate_issuing::Call::<T>::asset_burn_event_handle(
								input.to_vec(),
							)
							.into();
						call.encode()
					}
					// this method comes from mapping-token-factory, we ignore it by a empty method
					_ if method_digest == &sha3::Keccak256::digest(TOKEN_REGISTER_RESPONSE_METHOD)[..METHOD_DIG_LEN] => Vec::new(),
					_ if method_digest == READ_LATEST_MESSAGE_ID_METHOD => {
						<T as from_substrate_issuing::Config>::MessageSender::latest_message_id()
							.to_vec()
					}
					_ => {
						return Err(ExitError::Other(
							"No such method in pallet substrate issuing".into(),
						));
					}
				}
			}
			_ if pallet_digest == <from_ethereum_issuing::Pallet<T>>::digest() => {
				let call: T::Call = match method_digest {
					_ if method_digest == &sha3::Keccak256::digest(TOKEN_REGISTER_RESPONSE_METHOD)[..METHOD_DIG_LEN] => {
						from_ethereum_issuing::Call::<T>::register_response_from_contract(
							input.to_vec(),
						)
						.into()
					}
					_ if method_digest == &sha3::Keccak256::digest(BURN_AND_REMOTE_UNLOCK_METHOD)[..METHOD_DIG_LEN] => {
						from_ethereum_issuing::Call::<T>::burn_and_remote_unlock(input.to_vec())
							.into()
					}
					_ => {
						return Err(ExitError::Other(
							"No such method in pallet ethereum issuing".into(),
						));
					}
				};
				call.encode()
			}
			_ => {
				return Err(ExitError::Other("No valid pallet digest found".into()));
			}
		};
		// TODO: The cost should not be zero
		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Stopped,
			cost: 0,
			output,
			logs: Default::default(),
		})
	}
}
