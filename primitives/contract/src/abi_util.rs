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

//! the primitive types of contract encode and decode.

use ethabi::{param_type::ParamType, token::Token, Error, Result as AbiResult};
use sp_std::{convert::TryInto, prelude::*};

pub fn abi_decode_bytes4(data: &[u8]) -> AbiResult<[u8; 4]> {
	let tokens = ethabi::decode(&[ParamType::FixedBytes(4)], &data)?;
	if let Token::FixedBytes(decoded) = tokens[0].clone() {
		let decoded: [u8; 4] = decoded.try_into().map_err(|_| Error::InvalidData)?;
		return Ok(decoded);
	}
	Err(Error::InvalidData)
}

pub fn abi_encode_bytes4(data: [u8; 4]) -> Vec<u8> {
	ethabi::encode(&[Token::FixedBytes(data.to_vec())])
}

pub fn abi_decode_bytes(data: &[u8]) -> AbiResult<Vec<u8>> {
	let tokens = ethabi::decode(&[ParamType::Bytes], &data)?;
	if let Token::Bytes(decoded) = tokens[0].clone() {
		return Ok(decoded);
	}
	Err(Error::InvalidData)
}

pub fn abi_encode_bytes(data: &[u8]) -> Vec<u8> {
	ethabi::encode(&[Token::Bytes(data.to_vec())])
}

pub fn abi_encode_u64(data: u64) -> Vec<u8> {
	ethabi::encode(&[Token::Uint(data.into())])
}

pub fn abi_encode_bytes32(data: [u8; 32]) -> Vec<u8> {
	ethabi::encode(&[Token::FixedBytes(data.to_vec())])
}

pub fn abi_encode_array_bytes32(data: Vec<[u8; 32]>) -> Vec<u8> {
	let array = data.iter().map(|v| Token::FixedBytes(v.to_vec())).collect();
	ethabi::encode(&[Token::Array(array)])
}

pub fn abi_encode_array_bytes(data: Vec<Vec<u8>>) -> Vec<u8> {
	let array = data.iter().map(|v| Token::Bytes(v.to_vec())).collect();
	ethabi::encode(&[Token::Array(array)])
}
