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

// --- crates.io ---
use ethereum_types::{Address as EthereumAddress, H160, U256};
// --- darwinia-network ---
use ethabi::{
	param_type::ParamType, token::Token, Bytes, Error, Function, Param, Result as AbiResult,
};
// --- paritytech ---
use sp_std::prelude::*;

pub struct MappingTokenFactory;

impl MappingTokenFactory {
	fn cross_receive() -> Function {
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
				name: "backing_address".into(),
				kind: ParamType::Address,
			},
			Param {
				name: "original_token".into(),
				kind: ParamType::Address,
			},
			Param {
				name: "backing_chain_name".into(),
				kind: ParamType::String,
			},
		];

		let outputs = vec![Param {
			name: "mapping_token".into(),
			kind: ParamType::Address,
		}];

		Function {
			name: "createERC20Contract".into(),
			inputs,
			outputs,
			constant: false,
		}
	}

	fn confirm_burn_and_remote_unlock() -> Function {
		let inputs = vec![
			Param {
				name: "messageId".into(),
				kind: ParamType::Bytes,
			},
			Param {
				name: "result".into(),
				kind: ParamType::Bool,
			},
		];

		Function {
			name: "confirmBurnAndRemoteUnlock".into(),
			inputs,
			outputs: vec![],
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
				Token::Address(token),
				Token::Address(recipient),
				Token::Uint(amount),
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
		backing_chain_name: &str,
	) -> AbiResult<Bytes> {
		let create = Self::create_erc20();
		create.encode_input(
			vec![
				Token::FixedBytes(event_receiver.to_vec()),
				Token::Uint(U256::from(token_type)),
				Token::String(name.into()),
				Token::String(symbol.into()),
				Token::Uint(U256::from(decimals)),
				Token::Address(backing),
				Token::Address(source),
				Token::String(backing_chain_name.into()),
			]
			.as_slice(),
		)
	}

	/// encode confirm burn and remote unlock deliver message function
	pub fn encode_confirm_burn_and_remote_unlock(
		message_id: Vec<u8>,
		result: bool,
	) -> AbiResult<Bytes> {
		let confirm = Self::confirm_burn_and_remote_unlock();
		confirm.encode_input(vec![Token::Bytes(message_id), Token::Bool(result)].as_slice())
	}

	/// get mapped token from source
	pub fn mapping_token() -> Function {
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
		mapping.encode_input(vec![Token::Address(backing), Token::Address(source)].as_slice())
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
	pub weight: u64,
	pub token_type: u32,
	pub backing: H160,
	pub sender: H160,
	pub source: H160,
	pub recipient: Vec<u8>,
	pub amount: U256,
	pub fee: U256,
}

impl TokenBurnInfo {
	pub fn encode(
		spec_version: u32,
		weight: u64,
		token_type: u32,
		backing: H160,
		sender: H160,
		source: H160,
		recipient: Vec<u8>,
		amount: U256,
		fee: U256,
	) -> Vec<u8> {
		ethabi::encode(&[
			Token::Uint(spec_version.into()),
			Token::Uint(weight.into()),
			Token::Uint(token_type.into()),
			Token::Address(backing),
			Token::Address(sender),
			Token::Address(source),
			Token::Bytes(recipient),
			Token::Uint(amount),
			Token::Uint(fee),
		])
	}

	pub fn decode(data: &[u8]) -> AbiResult<Self> {
		let tokens = ethabi::decode(
			&[
				ParamType::Uint(256),
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
			tokens[8].clone(),
		) {
			(
				Token::Uint(spec_version),
				Token::Uint(weight),
				Token::Uint(token_type),
				Token::Address(backing),
				Token::Address(sender),
				Token::Address(source),
				Token::Bytes(recipient),
				Token::Uint(amount),
				Token::Uint(fee),
			) => Ok(TokenBurnInfo {
				spec_version: spec_version.low_u32(),
				weight: weight.low_u64(),
				token_type: token_type.low_u32(),
				backing,
				sender,
				source,
				recipient,
				amount,
				fee,
			}),
			_ => Err(Error::InvalidData),
		}
	}
}
