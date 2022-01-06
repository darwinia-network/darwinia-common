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

//! ethereum backing encode and decode.

pub use ethabi::{Event, Log};

// --- crates.io ---
use ethabi::{param_type::ParamType, Error, EventParam, RawLog, Result as AbiResult};
use ethereum_types::{Address as EthereumAddress, U256};
// --- darwinia-network ---
use ethereum_primitives::receipt::EthereumReceipt;
// --- paritytech ---
use sp_std::prelude::*;

pub struct EthereumBacking;

pub struct EthereumRegisterEvent {
	pub token_address: EthereumAddress,
	pub name: Vec<u8>,
	pub symbol: Vec<u8>,
	pub decimals: U256,
	pub fee: U256,
}

pub struct EthereumLockEvent {
	pub sender: EthereumAddress,
	pub source: EthereumAddress,
	pub mapping_token: EthereumAddress,
	pub amount: U256,
	pub recipient: EthereumAddress,
	pub fee: U256,
}

impl EthereumBacking {
	/// this New Register Event comes from the outer chains
	/// @params token: source erc20 token address
	/// @params name:  source erc20 token name
	/// @params symbol: source erc20 token symbol, which will added by m to generate mapped token
	/// @params decimals: source erc20 token decimals
	/// @params fee: register fee from the outer chain to darwinia
	pub fn register_event() -> Event {
		Event {
			name: "NewTokenRegistered".into(),
			inputs: vec![
				EventParam {
					name: "token".into(),
					kind: ParamType::Address,
					indexed: true,
				},
				EventParam {
					name: "name".into(),
					kind: ParamType::String,
					indexed: false,
				},
				EventParam {
					name: "symbol".into(),
					kind: ParamType::String,
					indexed: false,
				},
				EventParam {
					name: "decimals".into(),
					kind: ParamType::Uint(8),
					indexed: false,
				},
				EventParam {
					name: "fee".into(),
					kind: ParamType::Uint(256),
					indexed: false,
				},
			],
			anonymous: false,
		}
	}

	/// this Token Lock Event comes from the outer chains
	/// @params token: source erc20 token address
	/// @params target:  mapped erc20 token address
	/// @params amount: transfer amount of the token
	/// @params recipient: the receiver on darwinia of the asset
	/// @params fee: transfer fee from the outer chain to darwinia
	pub fn locking_event() -> Event {
		Event {
			name: "BackingLock".into(),
			inputs: vec![
				EventParam {
					name: "sender".into(),
					kind: ParamType::Address,
					indexed: true,
				},
				EventParam {
					name: "source".into(),
					kind: ParamType::Address,
					indexed: false,
				},
				EventParam {
					name: "target".into(),
					kind: ParamType::Address,
					indexed: false,
				},
				EventParam {
					name: "amount".into(),
					kind: ParamType::Uint(256),
					indexed: false,
				},
				EventParam {
					name: "recipient".into(),
					kind: ParamType::Address,
					indexed: false,
				},
				EventParam {
					name: "fee".into(),
					kind: ParamType::Uint(256),
					indexed: false,
				},
			],
			anonymous: false,
		}
	}

	/// parse token register event
	fn parse_event(
		log_event: Event,
		receipt: &EthereumReceipt,
		backing_address: &EthereumAddress,
	) -> AbiResult<Log> {
		let log_entry = receipt
			.as_legacy_receipt()
			.logs
			.iter()
			.find(|x| &x.address == backing_address && x.topics[0] == log_event.signature())
			.cloned()
			.ok_or(Error::InvalidData)?;
		let log = RawLog {
			topics: log_entry.topics.into_iter().collect(),
			data: log_entry.data,
		};
		log_event.parse_log(log)
	}

	fn log_params2address(log: &Log, idx: usize) -> AbiResult<EthereumAddress> {
		log.params[idx]
			.value
			.clone()
			.into_address()
			.ok_or(Error::InvalidData)
	}

	fn log_params2string(log: &Log, idx: usize) -> AbiResult<Vec<u8>> {
		Ok(log.params[idx]
			.value
			.clone()
			.into_string()
			.ok_or(Error::InvalidData)?
			.as_bytes()
			.to_vec())
	}

	fn log_params2uint(log: &Log, idx: usize) -> AbiResult<U256> {
		log.params[idx]
			.value
			.clone()
			.into_uint()
			.ok_or(Error::InvalidData)
	}

	pub fn parse_register_event(
		receipt: &EthereumReceipt,
		backing_address: &EthereumAddress,
	) -> AbiResult<EthereumRegisterEvent> {
		let log_event = Self::register_event();
		let ethlog = Self::parse_event(log_event, receipt, backing_address)?;
		Ok(EthereumRegisterEvent {
			token_address: Self::log_params2address(&ethlog, 0)?,
			name: Self::log_params2string(&ethlog, 1)?,
			symbol: Self::log_params2string(&ethlog, 2)?,
			decimals: Self::log_params2uint(&ethlog, 3)?,
			fee: Self::log_params2uint(&ethlog, 4)?,
		})
	}

	pub fn parse_locking_event(
		receipt: &EthereumReceipt,
		backing_address: &EthereumAddress,
	) -> AbiResult<EthereumLockEvent> {
		let log_event = Self::locking_event();
		let ethlog = Self::parse_event(log_event, receipt, backing_address)?;
		Ok(EthereumLockEvent {
			sender: Self::log_params2address(&ethlog, 0)?,
			source: Self::log_params2address(&ethlog, 1)?,
			mapping_token: Self::log_params2address(&ethlog, 2)?,
			amount: Self::log_params2uint(&ethlog, 3)?,
			recipient: Self::log_params2address(&ethlog, 4)?,
			fee: Self::log_params2uint(&ethlog, 5)?,
		})
	}
}
