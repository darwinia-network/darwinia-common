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
use codec::Encode;
use evm::{executor::PrecompileOutput, Context, ExitError, ExitSucceed};
use sha3::Digest;
// --- darwinia-network ---
use darwinia_support::{
	evm::IntoAccountId,
	s2s::{nonce_to_message_id, LatestMessageNoncer, RelayMessageSender},
};
use dp_contract::mapping_token_factory::s2s::{S2sRemoteUnlockInfo, S2sSendMessageParams};
use dp_evm::Precompile;
use dp_s2s::{CallParams, CreatePayload};
// --- paritytech ---
use frame_support::{
	dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
	sp_runtime::SaturatedConversion,
};
use sp_std::convert::TryInto;

const ACTION_LEN: usize = 4;

// ethereum<>darwinia actions
const E2D_BURN_AND_REMOTE_UNLOCK: &[u8] = b"e2d_burn_and_remote_unlock()";
const E2D_TOKEN_REGISTER_RESPONSE: &[u8] = b"e2d_token_register_response()";

// substrate<>substrate actions
const S2S_READ_LATEST_MESSAGE_ID: &[u8] = b"s2s_read_latest_message_id()";
const S2S_READ_LATEST_RECV_MESSAGE_ID: &[u8] = b"s2s_read_latest_recv_message_id()";
const S2S_ENCODE_REMOTE_UNLOCK_PAYLOAD: &[u8] = b"s2s_encode_remote_unlock_payload()";
const S2S_ENCODE_SEND_MESSAGE_CALL: &[u8] = b"s2s_encode_send_message_call()";

/// The contract address: 0000000000000000000000000000000000000018
pub struct Misc<T, S> {
	_marker: PhantomData<(T, S)>,
}

impl<T, S> Precompile for Misc<T, S>
where
	T: from_ethereum_issuing::Config,
	T: from_substrate_issuing::Config,
	T::Call: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo + Encode,
	<T::Call as Dispatchable>::Origin: From<Option<T::AccountId>>,
	T::Call: From<from_substrate_issuing::Call<T>>,
	T::Call: From<from_ethereum_issuing::Call<T>>,
	S: RelayMessageSender + LatestMessageNoncer,
{
	fn execute(
		input: &[u8],
		_target_gas: Option<u64>,
		context: &Context,
	) -> core::result::Result<PrecompileOutput, ExitError> {
		if input.len() < ACTION_LEN {
			return Err(ExitError::Other("input length less than 4 bytes".into()));
		}
		let action_digest = &input[0..ACTION_LEN];
		let action_params = &input[ACTION_LEN..];
		let output = match action_digest {
			_ if Self::match_digest(action_digest, E2D_BURN_AND_REMOTE_UNLOCK) => {
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
			_ if Self::match_digest(action_digest, S2S_READ_LATEST_MESSAGE_ID) => {
				let lane_id: [u8; 4] = action_params
					.try_into()
					.map_err(|_| ExitError::Other("decode lane id failed".into()))?;
				let nonce = <S as LatestMessageNoncer>::outbound_latest_generated_nonce(lane_id);
				nonce_to_message_id(&lane_id, nonce).to_vec()
			}
			_ if Self::match_digest(action_digest, S2S_READ_LATEST_RECV_MESSAGE_ID) => {
				let lane_id: [u8; 4] = action_params
					.try_into()
					.map_err(|_| ExitError::Other("decode lane id failed".into()))?;
				let nonce = <S as LatestMessageNoncer>::inbound_latest_received_nonce(lane_id);
				nonce_to_message_id(&lane_id, nonce).to_vec()
			}
			_ if Self::match_digest(action_digest, S2S_ENCODE_REMOTE_UNLOCK_PAYLOAD) => {
				let unlock_info = S2sRemoteUnlockInfo::abi_decode(&action_params)
					.map_err(|_| ExitError::Other("decode unlock info failed".into()))?;
				let payload = <T as from_substrate_issuing::Config>::OutboundPayload::create(
					T::IntoAccountId::into_account_id(context.caller),
					unlock_info.spec_version,
					unlock_info.weight,
					CallParams::S2sBackingPalletUnlockFromRemote(
						unlock_info.original_token,
						unlock_info.amount,
						unlock_info.recipient,
					),
				)
				.map_err(|_| ExitError::Other("encode remote unlock failed".into()))?;
				payload.encode()
			}
			_ if Self::match_digest(action_digest, S2S_ENCODE_SEND_MESSAGE_CALL) => {
				let params = S2sSendMessageParams::decode(&action_params)
					.map_err(|_| ExitError::Other("decode send message info failed".into()))?;
				<S as RelayMessageSender>::encode_send_message(
					params.pallet_index,
					params.lane_id,
					params.payload,
					params.fee.low_u128().saturated_into(),
				)
				.map_err(|_| ExitError::Other("encode send message call failed".into()))?
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

impl<T, S> Misc<T, S> {
	fn match_digest(digest: &[u8], expected_method: &[u8]) -> bool {
		&sha3::Keccak256::digest(expected_method)[..ACTION_LEN] == digest
	}
}
