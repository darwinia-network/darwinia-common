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

//! basic mapping token factory encode and decode.

pub use ethabi::{Event, Log};

// --- crates.io ---
use ethabi::{
	param_type::ParamType, token::Token, Bytes, Function, Param, Result as AbiResult,
	StateMutability,
};
use ethereum_types::{Address as EthereumAddress, U256};
// --- paritytech ---
use sp_std::vec;

pub struct BasicMappingTokenFactory;

impl BasicMappingTokenFactory {
	/// encode get mapping token info function
	pub fn encode_mapping_token(
		backing_address: EthereumAddress,
		original_token: EthereumAddress,
	) -> AbiResult<Bytes> {
		let inputs = vec![
			Param {
				name: "backing_address".into(),
				kind: ParamType::Address,
				internal_type: Some("address".into()),
			},
			Param {
				name: "original_token".into(),
				kind: ParamType::Address,
				internal_type: Some("address".into()),
			},
		];

		let outputs = vec![Param {
			name: "mapping_token".into(),
			kind: ParamType::Address,
			internal_type: Some("address".into()),
		}];

		#[allow(deprecated)]
		Function {
			name: "mappingToken".into(),
			inputs,
			outputs,
			constant: true,
			state_mutability: StateMutability::View,
		}
		.encode_input(
			vec![Token::Address(backing_address), Token::Address(original_token)].as_slice(),
		)
	}

	/// encode create new erc20 function
	pub fn encode_create_erc20(
		token_type: u32,
		name: &str,
		symbol: &str,
		decimals: u8,
		backing_address: EthereumAddress,
		original_token: EthereumAddress,
	) -> AbiResult<Bytes> {
		let inputs = vec![
			Param {
				name: "tokenType".into(),
				kind: ParamType::Uint(32),
				internal_type: Some("uint32".into()),
			},
			Param {
				name: "name".into(),
				kind: ParamType::String,
				internal_type: Some("string".into()),
			},
			Param {
				name: "symbol".into(),
				kind: ParamType::String,
				internal_type: Some("string".into()),
			},
			Param {
				name: "decimals".into(),
				kind: ParamType::Uint(8),
				internal_type: Some("uint8".into()),
			},
			Param {
				name: "backing_address".into(),
				kind: ParamType::Address,
				internal_type: Some("address".into()),
			},
			Param {
				name: "original_token".into(),
				kind: ParamType::Address,
				internal_type: Some("address".into()),
			},
		];

		let outputs = vec![Param {
			name: "token".into(),
			kind: ParamType::Address,
			internal_type: Some("address".into()),
		}];

		#[allow(deprecated)]
		Function {
			name: "newErc20Contract".into(),
			inputs,
			outputs,
			constant: false,
			state_mutability: StateMutability::NonPayable,
		}
		.encode_input(
			vec![
				Token::Uint(U256::from(token_type)),
				Token::String(name.into()),
				Token::String(symbol.into()),
				Token::Uint(U256::from(decimals)),
				Token::Address(backing_address),
				Token::Address(original_token),
			]
			.as_slice(),
		)
	}

	/// encode issuing function for erc20
	pub fn encode_issue_erc20(
		token: EthereumAddress,
		recipient: EthereumAddress,
		amount: U256,
	) -> AbiResult<Bytes> {
		let inputs = vec![
			Param {
				name: "mapping_token".into(),
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
			name: "issueMappingToken".into(),
			inputs,
			outputs: vec![],
			constant: false,
			state_mutability: StateMutability::NonPayable,
		}
		.encode_input(
			vec![Token::Address(token), Token::Address(recipient), Token::Uint(amount)].as_slice(),
		)
	}
}
