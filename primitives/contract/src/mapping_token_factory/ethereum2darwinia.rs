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

//! ethereum to darwinia mapping token factory encode and decode.

pub use ethabi::{Event, Log};

// --- crates.io ---
use ethereum_types::{H160, U256};
// --- darwinia-network ---
use ethabi::{param_type::ParamType, token::Token, Error, Result as AbiResult};
// --- paritytech ---
use sp_std::prelude::*;

/// token register response info
/// this is the response from darwinia after token registered
/// and would be sent to the outer chain
#[derive(Debug, PartialEq, Eq)]
pub struct TokenRegisterInfo(pub H160, pub H160, pub H160);

impl TokenRegisterInfo {
	pub fn decode(data: &[u8]) -> AbiResult<Self> {
		let tokens =
			ethabi::decode(&[ParamType::Address, ParamType::Address, ParamType::Address], &data)?;
		match (tokens[0].clone(), tokens[1].clone(), tokens[2].clone()) {
			(Token::Address(backing), Token::Address(source), Token::Address(target)) =>
				Ok(TokenRegisterInfo(backing, source, target)),
			_ => Err(Error::InvalidData),
		}
	}
}

/// token remote unlock request info
/// this is the event proof from darwinia after some user burn their mapped token
/// and would be sent to the outer chain to unlock the source token
/// @token_type: the type of original_token(native, erc20, etc)
/// @backing_address: the backing address on the source chain
/// @sender: the user who burn the mapping token and request unlock remote
/// @original_token: the source token address
/// @recipient: the final receiver of the token to be unlocked on the source chain
/// @amount: the amount of the burned token
#[derive(Debug, PartialEq, Eq)]
pub struct E2dRemoteUnlockInfo {
	pub token_type: u32,
	pub backing_address: H160,
	pub sender: H160,
	pub original_token: H160,
	pub recipient: H160,
	pub amount: U256,
}

impl E2dRemoteUnlockInfo {
	pub fn encode(
		token_type: u32,
		backing_address: H160,
		sender: H160,
		original_token: H160,
		recipient: H160,
		amount: U256,
	) -> Vec<u8> {
		ethabi::encode(&[
			Token::Uint(token_type.into()),
			Token::Address(backing_address),
			Token::Address(sender),
			Token::Address(original_token),
			Token::Address(recipient),
			Token::Uint(amount),
		])
	}

	pub fn decode(data: &[u8]) -> AbiResult<Self> {
		let tokens = ethabi::decode(
			&[
				ParamType::Uint(256),
				ParamType::Address,
				ParamType::Address,
				ParamType::Address,
				ParamType::Address,
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
				Token::Uint(token_type),
				Token::Address(backing_address),
				Token::Address(sender),
				Token::Address(original_token),
				Token::Address(recipient),
				Token::Uint(amount),
			) => Ok(Self {
				token_type: token_type.low_u32(),
				backing_address,
				sender,
				original_token,
				recipient,
				amount,
			}),
			_ => Err(Error::InvalidData),
		}
	}
}
