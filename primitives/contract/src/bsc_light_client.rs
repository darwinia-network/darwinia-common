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

//! bsc light client encode and decode.

pub use ethabi::{Event, Log};

// --- crates.io ---
use ethereum_types::{H160, H256};
// --- darwinia-network ---
use ethabi::{param_type::ParamType, token::Token, Error, Result as AbiResult};
// --- paritytech ---
use sp_std::{convert::TryInto, prelude::*};

pub type MerkleProof = Vec<Vec<u8>>;

#[derive(Debug, PartialEq, Eq)]
pub struct BscStorageVerifyParams {
	pub lane_address: H160,
	pub account_proof: MerkleProof,
	pub storage_keys: Vec<H256>,
	pub storage_proofs: Vec<MerkleProof>,
}

impl BscStorageVerifyParams {
	pub fn decode(data: &[u8]) -> AbiResult<Self> {
		let tokens = ethabi::decode(
			&[
				ParamType::FixedBytes(20),
				ParamType::Array(Box::new(ParamType::Bytes)),
				ParamType::Array(Box::new(ParamType::FixedBytes(32))),
				ParamType::Array(Box::new(ParamType::Array(Box::new(ParamType::Bytes)))),
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
				Token::FixedBytes(lane_address),
				Token::Array(account_proof),
				Token::Array(storage_keys),
				Token::Array(storage_proofs),
			) => {
				let lane_address: [u8; 20] =
					lane_address.try_into().map_err(|_| Error::InvalidData)?;
				let account_proof: AbiResult<MerkleProof> = account_proof
					.iter()
					.map(|x| match x {
						Token::Bytes(proof) => Ok(proof.clone()),
						_ => Err(Error::InvalidData),
					})
					.collect();
				let storage_keys: AbiResult<Vec<H256>> = storage_keys
					.iter()
					.map(|x| match x {
						Token::FixedBytes(storage_key) => {
							let key: [u8; 32] = storage_key
								.clone()
								.try_into()
								.map_err(|_| Error::InvalidData)?;
							Ok(key.into())
						}
						_ => Err(Error::InvalidData),
					})
					.collect();
				let storage_proofs: AbiResult<Vec<MerkleProof>> = storage_proofs
					.iter()
					.map(|storage_proof| match storage_proof {
						Token::Array(proof) => proof
							.iter()
							.map(|x| match x {
								Token::Bytes(proof_item) => Ok(proof_item.clone()),
								_ => Err(Error::InvalidData),
							})
							.collect(),
						_ => Err(Error::InvalidData),
					})
					.collect();
				Ok(Self {
					lane_address: lane_address.into(),
					storage_keys: storage_keys?,
					account_proof: account_proof?,
					storage_proofs: storage_proofs?,
				})
			}
			_ => Err(Error::InvalidData),
		}
	}
}
