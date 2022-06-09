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

//! bls12-381 precompile function params encode and decode.

pub use ethabi::{Event, Log};

// --- darwinia-network ---
use ethabi::{param_type::ParamType, token::Token, Error, Result as AbiResult};
// --- paritytech ---
use sp_std::prelude::*;

const BLS_PUBKEY_LENGTH: usize = 48;
const BLS_SIGNATURE_LENGTH: usize = 96;

pub type BLSPubkey = Vec<u8>;

#[derive(Debug, PartialEq, Eq)]
pub struct FastAggregateVerifyParams {
	pub pubkeys: Vec<BLSPubkey>,
	pub message: Vec<u8>,
	pub signature: Vec<u8>,
}

impl FastAggregateVerifyParams {
	pub fn decode(data: &[u8]) -> AbiResult<Self> {
		let tokens = ethabi::decode(
			&[ParamType::Array(Box::new(ParamType::Bytes)), ParamType::Bytes, ParamType::Bytes],
			data,
		)?;
		match (tokens[0].clone(), tokens[1].clone(), tokens[2].clone()) {
			(Token::Array(pubkeys), Token::Bytes(message), Token::Bytes(signature)) => {
				let pubkeys: AbiResult<Vec<BLSPubkey>> = pubkeys
					.iter()
					.map(|key| match key {
						Token::Bytes(key) =>
							if key.len() == BLS_PUBKEY_LENGTH {
								Ok(key.clone())
							} else {
								Err(Error::InvalidData)
							},
						_ => Err(Error::InvalidData),
					})
					.collect();
				if signature.len() != BLS_SIGNATURE_LENGTH {
					return Err(Error::InvalidData);
				}
				Ok(Self { pubkeys: pubkeys?, message, signature })
			},
			_ => Err(Error::InvalidData),
		}
	}
}
