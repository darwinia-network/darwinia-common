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

//! mapping token factory encode and decode.

pub use ethabi::{Event, Log};

// --- crates ---
use ethereum_types::{Address as EthereumAddress, H160, U256};
// --- github ---
use ethabi::{
	param_type::ParamType, token::Token, Bytes, Error, Function, Param, Result as AbiResult,
};

use sp_std::prelude::*;
use sp_std::vec;

pub struct MappingTokenFactory;

impl MappingTokenFactory {
	fn cross_receive() -> Function {
		let inputs = vec![
			Param {
				name: "token".into(),
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
			name: "crossReceive".into(),
			inputs,
			outputs: vec![],
			constant: false,
		}
	}

	fn create_erc20() -> Function {
		let inputs = vec![
			Param {
				name: "eventReceiver".into(),
				kind: ParamType::FixedBytes(4),
			},
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
				name: "backing".into(),
				kind: ParamType::Address,
			},
			Param {
				name: "source".into(),
				kind: ParamType::Address,
			},
		];

		let outputs = vec![Param {
			name: "token".into(),
			kind: ParamType::Address,
		}];

		Function {
			name: "createERC20Contract".into(),
			inputs,
			outputs,
			constant: false,
		}
	}

	/// encode mint function for erc20
	pub fn encode_cross_receive(
		token: EthereumAddress,
		recipient: EthereumAddress,
		amount: U256,
	) -> AbiResult<Bytes> {
		let receive = Self::cross_receive();
		receive.encode_input(
			vec![
				Token::Address(token.into()),
				Token::Address(recipient.into()),
				Token::Uint(amount.into()),
			]
			.as_slice(),
		)
	}

	/// encode erc20 create function
	pub fn encode_create_erc20(
		event_receiver: [u8; 4],
		token_type: u32,
		name: &str,
		symbol: &str,
		decimals: u8,
		backing: EthereumAddress,
		source: EthereumAddress,
	) -> AbiResult<Bytes> {
		let create = Self::create_erc20();
		create.encode_input(
			vec![
				Token::FixedBytes(event_receiver.to_vec()),
				Token::Uint(U256::from(token_type)),
				Token::String(name.into()),
				Token::String(symbol.into()),
				Token::Uint(U256::from(decimals)),
				Token::Address(backing.into()),
				Token::Address(source.into()),
			]
			.as_slice(),
		)
	}

	/// get mapped token from source
	pub fn mapping_token() -> Function {
		let inputs = vec![
			Param {
				name: "backing".into(),
				kind: ParamType::Address,
			},
			Param {
				name: "source".into(),
				kind: ParamType::Address,
			},
		];

		let outputs = vec![Param {
			name: "target".into(),
			kind: ParamType::Address,
		}];

		Function {
			name: "mappingToken".into(),
			inputs,
			outputs,
			constant: true,
		}
	}

	/// encode get mapping token info function
	pub fn encode_mapping_token(
		backing: EthereumAddress,
		source: EthereumAddress,
	) -> AbiResult<Bytes> {
		let mapping = Self::mapping_token();
		mapping.encode_input(
			vec![
				Token::Address(backing.into()),
				Token::Address(source.into()),
			]
			.as_slice(),
		)
	}
}

/// token register info
/// this is the response from darwinia after token registered
/// and would be sent to the outer chain
#[derive(Debug, PartialEq, Eq)]
pub struct TokenRegisterInfo(pub H160, pub H160, pub H160);

impl TokenRegisterInfo {
	pub fn decode(data: &[u8]) -> AbiResult<Self> {
		let tokens = ethabi::decode(
			&[ParamType::Address, ParamType::Address, ParamType::Address],
			&data,
		)?;
		match (tokens[0].clone(), tokens[1].clone(), tokens[2].clone()) {
			(Token::Address(backing), Token::Address(source), Token::Address(target)) => {
				Ok(TokenRegisterInfo(backing, source, target))
			}
			_ => Err(Error::InvalidData),
		}
	}
}

/// token burn info
/// this is the event proof from darwinia after some user burn their mapped token
/// and would be sent to the outer chain to unlock the source token
/// @backing: the backing address on the source chain
/// @source: the source token address
/// @recipient: the final receiver of the token to be unlocked on the source chain
/// @amount: the amount of the burned token
#[derive(Debug, PartialEq, Eq)]
pub struct TokenBurnInfo {
	pub spec_version: u32,
	pub token_type: u32,
	pub backing: H160,
	pub sender: H160,
	pub source: H160,
	pub recipient: Vec<u8>,
	pub amount: U256,
	pub fee: u128,
}

impl TokenBurnInfo {
	pub fn decode(data: &[u8]) -> AbiResult<Self> {
		let tokens = ethabi::decode(
			&[
				ParamType::Uint(256),
				ParamType::Uint(256),
				ParamType::Address,
				ParamType::Address,
				ParamType::Address,
				ParamType::Bytes,
				ParamType::Uint(256),
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
			tokens[6].clone(),
			tokens[7].clone(),
		) {
			(
				Token::Uint(spec_version),
				Token::Uint(token_type),
				Token::Address(backing),
				Token::Address(sender),
				Token::Address(source),
				Token::Bytes(recipient),
				Token::Uint(amount),
				Token::Uint(fee),
			) => Ok(TokenBurnInfo {
				spec_version: spec_version.low_u32(),
				token_type: token_type.low_u32(),
				backing,
				sender,
				source,
				recipient,
				amount,
				fee: fee.low_u128(),
			}),
			_ => Err(Error::InvalidData),
		}
	}
}
