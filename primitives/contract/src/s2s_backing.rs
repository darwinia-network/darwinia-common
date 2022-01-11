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

//! s2s backing encode and decode.

pub use ethabi::{Event, Log};

// --- crates.io ---
use ethabi::{
	param_type::ParamType, token::Token, Bytes, Function, Param, Result as AbiResult,
	StateMutability,
};

use ethereum_types::{H160, U256};
// --- paritytech ---
use bp_messages::{LaneId, MessageNonce};
use sp_std::vec;

pub struct Sub2SubBacking;

impl Sub2SubBacking {
	pub fn encode_unlock_from_remote(
		token: H160,
		recipient: H160,
		amount: U256,
	) -> AbiResult<Bytes> {
		let inputs = vec![
			Param {
				name: "token".into(),
				kind: ParamType::Address,
				internal_type: Some("address".into()),
			},
			Param {
				name: "recipient".into(),
				kind: ParamType::Address,
				internal_type: Some("address".into()),
			},
			Param {
				name: "amount".into(),
				kind: ParamType::Uint(256),
				internal_type: Some("uint256".into()),
			},
		];

		#[allow(deprecated)]
		Function {
			name: "unlockFromRemote".into(),
			inputs,
			outputs: vec![],
			constant: false,
			state_mutability: StateMutability::NonPayable,
		}
		.encode_input(
			vec![
				Token::Address(token),
				Token::Address(recipient),
				Token::Uint(amount),
			]
			.as_slice(),
		)
	}

	pub fn confirm_remote_lock_or_register(
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
			name: "confirmRemoteLockOrRegister".into(),
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
