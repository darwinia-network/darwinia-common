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

//! basic channel encode and decode.

pub use ethabi::{Event, Log};

// --- crates.io ---
use ethereum_types::{Address as EthereumAddress, H256};
// --- darwinia ---
use ethabi::token::Token;
use ethabi::{Error, EventParam, ParamType, RawLog, Result as AbiResult};
use ethereum_primitives::receipt::EthereumReceipt;
// --- paritytech ---
use codec::{Decode, Encode};
use sp_std::prelude::*;

#[derive(Encode, Decode, Clone, PartialEq, Eq)]
pub struct BasicOutboundMessage {
	pub target: EthereumAddress,
	pub nonce: u64,
	pub payload: Vec<u8>,
}

impl BasicOutboundMessage {
	pub fn encode(target: EthereumAddress, nonce: u64, payload: Vec<u8>) -> Vec<u8> {
		let res = ethabi::encode(&[
			Token::Address(target),
			Token::Uint(nonce.into()),
			Token::Bytes(payload.clone()),
		]);
		res
	}

	pub fn encode_messages(messages: &[Self]) -> Vec<u8> {
		let messages: Vec<Token> = messages
			.iter()
			.map(|message| {
				Token::Tuple(vec![
					Token::Address(message.target),
					Token::Uint(message.nonce.into()),
					Token::Bytes(message.payload.clone()),
				])
			})
			.collect();
		ethabi::encode(&vec![Token::Array(messages)])
	}
}

#[derive(Encode, Decode, Clone, PartialEq, Eq)]
pub struct MmrLeaf {
	pub parent_hash: H256,
	pub message_root: H256,
	pub block_number: u32,
}

impl MmrLeaf {
	pub fn new(parent_hash: &[u8], message_root: H256, block_number: u32) -> Self {
		Self {
			parent_hash: H256::from_slice(parent_hash),
			message_root,
			block_number,
		}
	}

	pub fn encode(&self) -> Vec<u8> {
		let res = ethabi::encode(&[
			Token::FixedBytes(self.parent_hash.0.to_vec()),
			Token::FixedBytes(self.message_root.0.to_vec()),
			Token::Uint(self.block_number.into()),
		]);
		res
	}
}

pub struct BasicInboundMessage {
	pub source: EthereumAddress,
	pub nonce: u64,
	pub payload: Vec<u8>,
}

impl BasicInboundMessage {
	pub fn channel_event() -> Event {
		Event {
			name: "Message".into(),
			inputs: vec![
				EventParam {
					name: "source".into(),
					kind: ParamType::Address,
					indexed: false,
				},
				EventParam {
					name: "nonce".into(),
					kind: ParamType::Uint(64),
					indexed: false,
				},
				EventParam {
					name: "payload".into(),
					kind: ParamType::Bytes,
					indexed: false,
				},
			],
			anonymous: false,
		}
	}

	pub fn parse_channel_event(
		source_channel: &EthereumAddress,
		receipt: &EthereumReceipt,
	) -> AbiResult<Self> {
		let event = Self::channel_event();
		let log_entry = receipt
			.logs
			.clone()
			.into_iter()
			.find(|x| &x.address == source_channel && x.topics[0] == event.signature())
			.ok_or(Error::InvalidData)?;

		let log = RawLog {
			topics: log_entry
				.topics
				.into_iter()
				.map(|t| -> H256 { t.into() })
				.collect(),
			data: log_entry.data.clone(),
		};
		let ethlog = event.parse_log(log)?;
		Ok(Self {
			source: ethlog.params[0]
				.value
				.clone()
				.into_address()
				.ok_or(Error::InvalidData)?,
			nonce: ethlog.params[1]
				.value
				.clone()
				.into_uint()
				.ok_or(Error::InvalidData)?
				.low_u64(),
			payload: ethlog.params[2]
				.value
				.clone()
				.into_bytes()
				.ok_or(Error::InvalidData)?,
		})
	}
}
