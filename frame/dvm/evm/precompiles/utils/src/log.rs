// SPDX-License-Identifier: Apache-2.0
// This file is part of Frontier.
//
// Copyright (c) 2020 Parity Technologies (UK) Ltd.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// --- darwinia-network ---
use crate::EvmResult;
// --- paritytech ---
use fp_evm::{ExitError, Log, PrecompileFailure};
use sp_core::{H160, H256};
use sp_std::{vec, vec::Vec};

/// Create a 0-topic log.
pub fn log0(address: impl Into<H160>, data: impl Into<Vec<u8>>) -> Log {
	Log { address: address.into(), topics: vec![], data: data.into() }
}

/// Create a 1-topic log.
#[must_use]
pub fn log1(address: impl Into<H160>, topic0: impl Into<H256>, data: impl Into<Vec<u8>>) -> Log {
	Log { address: address.into(), topics: vec![topic0.into()], data: data.into() }
}

/// Create a 2-topics log.
pub fn log2(
	address: impl Into<H160>,
	topic0: impl Into<H256>,
	topic1: impl Into<H256>,
	data: impl Into<Vec<u8>>,
) -> Log {
	Log { address: address.into(), topics: vec![topic0.into(), topic1.into()], data: data.into() }
}

/// Create a 3-topics log.
pub fn log3(
	address: impl Into<H160>,
	topic0: impl Into<H256>,
	topic1: impl Into<H256>,
	topic2: impl Into<H256>,
	data: impl Into<Vec<u8>>,
) -> Log {
	Log {
		address: address.into(),
		topics: vec![topic0.into(), topic1.into(), topic2.into()],
		data: data.into(),
	}
}

/// Create a 4-topics log.
pub fn log4(
	address: impl Into<H160>,
	topic0: impl Into<H256>,
	topic1: impl Into<H256>,
	topic2: impl Into<H256>,
	topic3: impl Into<H256>,
	data: impl Into<Vec<u8>>,
) -> Log {
	Log {
		address: address.into(),
		topics: vec![topic0.into(), topic1.into(), topic2.into(), topic3.into()],
		data: data.into(),
	}
}

pub fn log_costs(topics: usize, data_len: usize) -> EvmResult<u64> {
	// Cost calculation is copied from EVM code that is not publicly exposed by the crates.
	// https://github.com/rust-blockchain/evm/blob/master/gasometer/src/costs.rs#L148

	const G_LOG: u64 = 375;
	const G_LOGDATA: u64 = 8;
	const G_LOGTOPIC: u64 = 375;

	let topic_cost = G_LOGTOPIC
		.checked_mul(topics as u64)
		.ok_or(PrecompileFailure::Error { exit_status: ExitError::OutOfGas })?;

	let data_cost = G_LOGDATA
		.checked_mul(data_len as u64)
		.ok_or(PrecompileFailure::Error { exit_status: ExitError::OutOfGas })?;

	G_LOG
		.checked_add(topic_cost)
		.ok_or(PrecompileFailure::Error { exit_status: ExitError::OutOfGas })?
		.checked_add(data_cost)
		.ok_or(PrecompileFailure::Error { exit_status: ExitError::OutOfGas })
}
