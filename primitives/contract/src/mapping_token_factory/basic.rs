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

//! basic mapping token factory encode and decode.

pub use ethabi::{Event, Log};

// --- crates.io ---
use ethereum_types::{Address as EthereumAddress, U256};
// --- darwinia-network ---
use ethabi::{param_type::ParamType, token::Token, Bytes, Function, Param, Result as AbiResult};
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
			},
			Param {
				name: "original_token".into(),
				kind: ParamType::Address,
			},
		];

		let outputs = vec![Param {
			name: "mapping_token".into(),
			kind: ParamType::Address,
		}];

		Function {
			name: "mappingToken".into(),
			inputs,
			outputs,
			constant: true,
		}
		.encode_input(
			vec![
				Token::Address(backing_address),
				Token::Address(original_token),
			]
			.as_slice(),
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
			},
			Param {
				name: "name".into(),
				kind: ParamType::String,
			},
			Param {
				name: "symbol".into(),
				kind: ParamType::String,
			},
			Param {
				name: "decimals".into(),
				kind: ParamType::Uint(8),
			},
			Param {
				name: "backing_address".into(),
				kind: ParamType::Address,
			},
			Param {
				name: "original_token".into(),
				kind: ParamType::Address,
			},
		];

		let outputs = vec![Param {
			name: "token".into(),
			kind: ParamType::Address,
		}];

		Function {
			name: "newErc20Contract".into(),
			inputs,
			outputs,
			constant: false,
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
			},
			Param {
				name: "recipient".into(),
				kind: ParamType::Address,
			},
			Param {
				name: "amount".into(),
				kind: ParamType::Uint(256),
			},
		];

		Function {
			name: "issueMappingToken".into(),
			inputs,
			outputs: vec![],
			constant: false,
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
}
