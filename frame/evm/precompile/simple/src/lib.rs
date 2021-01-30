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

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;
use core::cmp::min;
use darwinia_evm_primitives::LinearCostPrecompile;
use evm::{ExitError, ExitSucceed};

/// The identity precompile.
pub struct Identity;

impl LinearCostPrecompile for Identity {
	const BASE: usize = 15;
	const WORD: usize = 3;

	fn execute(input: &[u8], _: usize) -> core::result::Result<(ExitSucceed, Vec<u8>), ExitError> {
		Ok((ExitSucceed::Returned, input.to_vec()))
	}
}

/// The ecrecover precompile.
pub struct ECRecover;

impl LinearCostPrecompile for ECRecover {
	const BASE: usize = 3000;
	const WORD: usize = 0;

	fn execute(i: &[u8], _: usize) -> core::result::Result<(ExitSucceed, Vec<u8>), ExitError> {
		let mut input = [0u8; 128];
		input[..min(i.len(), 128)].copy_from_slice(&i[..min(i.len(), 128)]);

		let mut msg = [0u8; 32];
		let mut sig = [0u8; 65];

		msg[0..32].copy_from_slice(&input[0..32]);
		sig[0..32].copy_from_slice(&input[64..96]);
		sig[32..64].copy_from_slice(&input[96..128]);
		sig[64] = input[63];

		let pubkey = sp_io::crypto::secp256k1_ecdsa_recover(&sig, &msg)
			.map_err(|_| ExitError::Other("Public key recover failed".into()))?;
		let mut address = sp_io::hashing::keccak_256(&pubkey);
		address[0..12].copy_from_slice(&[0u8; 12]);

		Ok((ExitSucceed::Returned, address.to_vec()))
	}
}

/// The ripemd precompile.
pub struct Ripemd160;

impl LinearCostPrecompile for Ripemd160 {
	const BASE: usize = 600;
	const WORD: usize = 120;

	fn execute(
		input: &[u8],
		_cost: usize,
	) -> core::result::Result<(ExitSucceed, Vec<u8>), ExitError> {
		use ripemd160::Digest;

		let mut ret = [0u8; 32];
		ret[12..32].copy_from_slice(&ripemd160::Ripemd160::digest(input));
		Ok((ExitSucceed::Returned, ret.to_vec()))
	}
}

/// The sha256 precompile.
pub struct Sha256;

impl LinearCostPrecompile for Sha256 {
	const BASE: usize = 60;
	const WORD: usize = 12;

	fn execute(
		input: &[u8],
		_cost: usize,
	) -> core::result::Result<(ExitSucceed, Vec<u8>), ExitError> {
		let ret = sp_io::hashing::sha2_256(input);
		Ok((ExitSucceed::Returned, ret.to_vec()))
	}
}
