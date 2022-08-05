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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

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
