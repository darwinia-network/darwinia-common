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

//! substrate to substrate mapping token factory encode and decode.

pub use ethabi::{Event, Log};

// --- crates.io ---
use ethereum_types::{H160, U256};
// --- darwinia-network ---
use ethabi::{
	param_type::ParamType, token::Token, Bytes, Error, Function, Param, Result as AbiResult,
	StateMutability,
};
// --- paritytech ---
use bp_messages::{LaneId, MessageNonce};
use sp_std::{convert::TryInto, prelude::*};

pub struct Sub2SubMappingTokenFactory;

impl Sub2SubMappingTokenFactory {
	/// encode confirm burn and remote unlock deliver message function
	pub fn encode_confirm_burn_and_remote_unlock(
		lane_id: &LaneId,
		message_nonce: MessageNonce,
		result: bool,
	) -> AbiResult<Bytes> {
		let inputs = vec![
			Param {
				name: "lane_id".into(),
				kind: ParamType::FixedBytes(4),
				internal_type: Some("bytes4".into()),
			},
			Param {
				name: "nonce".into(),
				kind: ParamType::Uint(64),
				internal_type: Some("uint64".into()),
			},
			Param {
				name: "result".into(),
				kind: ParamType::Bool,
				internal_type: Some("bool".into()),
			},
		];

		#[allow(deprecated)]
		Function {
			name: "confirmBurnAndRemoteUnlock".into(),
			inputs,
			outputs: vec![],
			constant: false,
			state_mutability: StateMutability::NonPayable,
		}
		.encode_input(
			vec![
				Token::FixedBytes(lane_id.to_vec()),
				Token::Uint(message_nonce.into()),
				Token::Bool(result),
			]
			.as_slice(),
		)
	}
}

/// S2sRemoteUnlockInfo
/// this is the unlock message from mapping-token-factory after some user burn their mapped token
/// and would be sent to the outer chain to unlock the original token
/// @spec_version: the remote chain's spec_version
/// @weight: the remote dispatch call's weight
/// @token_type: the type of original_token(native, erc20, etc)
/// @original_token: the origin token address
/// @recipient: the final receiver of the token to be unlocked on the source chain
/// @amount: the amount of the unlocked token
#[derive(Debug, PartialEq, Eq)]
pub struct S2sRemoteUnlockInfo {
	pub spec_version: u32,
	pub weight: u64,
	pub token_type: u32,
	pub original_token: H160,
	pub amount: U256,
	pub recipient: Vec<u8>,
}

impl S2sRemoteUnlockInfo {
	pub fn abi_encode(
		spec_version: u32,
		weight: u64,
		token_type: u32,
		original_token: H160,
		recipient: Vec<u8>,
		amount: U256,
	) -> Vec<u8> {
		ethabi::encode(&[
			Token::Uint(spec_version.into()),
			Token::Uint(weight.into()),
			Token::Uint(token_type.into()),
			Token::Address(original_token),
			Token::Bytes(recipient),
			Token::Uint(amount),
		])
	}

	pub fn abi_decode(data: &[u8]) -> AbiResult<Self> {
		let tokens = ethabi::decode(
			&[
				ParamType::Uint(256),
				ParamType::Uint(256),
				ParamType::Uint(256),
				ParamType::Address,
				ParamType::Bytes,
				ParamType::Uint(256),
			],
			&data,
		)?;
		match (
			tokens[0].clone(),
			tokens[1].clone(),
			tokens[2].clone(),
			tokens[3].clone(),
			tokens[4].clone(),
			tokens[5].clone(),
		) {
			(
				Token::Uint(spec_version),
				Token::Uint(weight),
				Token::Uint(token_type),
				Token::Address(original_token),
				Token::Bytes(recipient),
				Token::Uint(amount),
			) => Ok(Self {
				spec_version: spec_version.low_u32(),
				weight: weight.low_u64(),
				token_type: token_type.low_u32(),
				original_token,
				amount,
				recipient,
			}),
			_ => Err(Error::InvalidData),
		}
	}
}

#[derive(Debug, PartialEq, Eq)]
pub struct S2sCallWrapper {
	pub spec_version: u32,
	pub weight: u64,
	pub opaque_call: Vec<u8>,
}

impl S2sCallWrapper {
	pub fn abi_decode(data: &[u8]) -> AbiResult<Self> {
		let tokens = ethabi::decode(
			&[ParamType::Uint(32), ParamType::Uint(64), ParamType::Bytes],
			&data,
		)?;
		match (tokens[0].clone(), tokens[1].clone(), tokens[2].clone()) {
			(Token::Uint(spec_version), Token::Uint(weight), Token::Bytes(opaque_call)) => Ok(Self {
				spec_version: spec_version.as_u32(),
				weight: weight.as_u64(),
				opaque_call,
			}),
			_ => Err(Error::InvalidData),
		}
	}
}

#[derive(Debug, PartialEq, Eq)]
pub struct S2sSendMessageParams {
	pub pallet_index: u32,
	pub lane_id: LaneId,
	pub payload: Vec<u8>,
	pub fee: U256,
}

impl S2sSendMessageParams {
	pub fn decode(data: &[u8]) -> AbiResult<Self> {
		let tokens = ethabi::decode(
			&[
				ParamType::Uint(32),
				ParamType::FixedBytes(4),
				ParamType::Bytes,
				ParamType::Uint(256),
			],
			&data,
		)?;
		match (
			tokens[0].clone(),
			tokens[1].clone(),
			tokens[2].clone(),
			tokens[3].clone(),
		) {
			(
				Token::Uint(pallet_index),
				Token::FixedBytes(lane_id),
				Token::Bytes(payload),
				Token::Uint(fee),
			) => {
				let lane_id: LaneId = lane_id.try_into().map_err(|_| Error::InvalidData)?;
				Ok(Self {
					pallet_index: pallet_index.low_u32(),
					lane_id,
					payload,
					fee,
				})
			}
			_ => Err(Error::InvalidData),
		}
	}
}
