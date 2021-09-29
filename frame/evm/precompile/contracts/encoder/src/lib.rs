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
// --- crates.io ---
use codec::{Decode, Encode};
use evm::{executor::PrecompileOutput, Context, ExitError, ExitSucceed};
use sha3::Digest;
// --- darwinia-network ---
use darwinia_support::s2s::RelayMessageCaller;
use dp_evm::Precompile;
use from_substrate_issuing::EncodeCall;
// --- paritytech ---
use frame_support::{
    dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
    sp_runtime::SaturatedConversion,
};

use dp_contract::mapping_token_factory::s2s::{S2sRemoteUnlockInfo, S2sSendMessageParams};

const ACTION_LEN: usize = 4;

// ethereum<>darwinia actions
const E2D_BURN_ADN_REMOTE_UNLOCK: &[u8] = b"e2d_burn_and_remote_unlock()";
const E2D_TOKEN_REGISTER_RESPONSE: &[u8] = b"e2d_token_register_response()";

// substrate<>substrate actions
const S2S_READ_LATEST_MESSAGE_ID_METHOD: &[u8] = b"s2s_read_latest_message_id()";
const S2S_REMOTE_DISPATCH_CALL_PAYLOAD: &[u8] = b"s2s_encode_remote_unlock_payload()";
const S2S_SEND_REMOTE_DISPATCH_CALL: &[u8] = b"s2s_encode_send_message_call()";

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
		let action_digest = &input[0..ACTION_LEN];
		let action_params = &input[ACTION_LEN..];
		let output = match action_digest {
			_ if Self::match_digest(action_digest, E2D_BURN_ADN_REMOTE_UNLOCK) => {
				let call: T::Call =
					from_ethereum_issuing::Call::<T>::deposit_burn_token_event_from_precompile(
						action_params.to_vec(),
					)
					.into();
				call.encode()
			}
			_ if Self::match_digest(action_digest, E2D_TOKEN_REGISTER_RESPONSE) => {
				let call: T::Call =
					from_ethereum_issuing::Call::<T>::register_response_from_contract(
						action_params.to_vec(),
					)
					.into();
				call.encode()
			}
			_ if Self::match_digest(action_digest, S2S_READ_LATEST_MESSAGE_ID_METHOD) => {
				<T as from_substrate_issuing::Config>::MessageSender::latest_token_message_id()
					.to_vec()
			}
			_ if Self::match_digest(action_digest, S2S_REMOTE_DISPATCH_CALL_PAYLOAD) => {
				let unlock_info = S2sRemoteUnlockInfo::decode(&action_params)
					.map_err(|_| ExitError::Other("decode unlock info failed".into()))?;
				let payload =
					<T as from_substrate_issuing::Config>::CallEncoder::encode_remote_unlock(
						unlock_info,
					)
					.map_err(|_| ExitError::Other("encode remote unlock failed".into()))?;
				payload.encode()
			}
			_ if Self::match_digest(action_digest, S2S_SEND_REMOTE_DISPATCH_CALL) => {
				let params = S2sSendMessageParams::decode(&action_params)
					.map_err(|_| ExitError::Other("decode send message info failed".into()))?;
				let payload = <T as from_substrate_issuing::Config>::OutboundPayload::decode(
					&mut params.payload.as_slice(),
				)
				.map_err(|_| ExitError::Other("decode send message info failed".into()))?;
				let call: T::Call = from_substrate_issuing::Call::<T>::send_message(
					payload,
					params.fee.saturated_into(),
				)
				.into();
				call.encode()
			}
			_ => {
				return Err(ExitError::Other("No valid pallet digest found".into()));
			}
		};
		// estimate a cost for this encoder process
		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Stopped,
			cost: 20000,
			output,
			logs: Default::default(),
		})
	}
}

impl<T> DispatchCallEncoder<T> {
	fn match_digest(digest: &[u8], expected_method: &[u8]) -> bool {
		&sha3::Keccak256::digest(expected_method)[..ACTION_LEN] == digest
	}
}
