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

#![cfg_attr(not(feature = "std"), no_std)]

// --- core ---
use core::marker::PhantomData;
// --- crates.io ---
use evm::{executor::PrecompileOutput, Context, ExitError, ExitSucceed};
// --- darwinia-network ---
use darwinia_bridge_bsc::StorageVerifier;
use darwinia_evm_precompile_utils::{selector, DvmInputParser};
use dp_contract::{abi_util::abi_encode_bytes32, bsc_light_client::BscStorageVerifyParams};
use dp_evm::Precompile;
use ethereum_primitives::storage::EthereumStorageProof;

#[selector]
enum Action {
	VerifyStorageProof = "verify_storage_proof(bytes32,address,bytes[],bytes[])",
}

/// The contract address: 0000000000000000000000000000000000000020
pub struct BscBridge<T> {
	_marker: PhantomData<T>,
}

impl<T> Precompile for BscBridge<T>
where
	T: StorageVerifier<[u8; 32]>,
{
	fn execute(
		input: &[u8],
		_target_gas: Option<u64>,
		_context: &Context,
	) -> core::result::Result<PrecompileOutput, ExitError> {
		let dvm_parser = DvmInputParser::new(&input)?;
		let output = match Action::from_u32(dvm_parser.selector)? {
			Action::VerifyStorageProof => {
				let params = BscStorageVerifyParams::decode(dvm_parser.input)
					.map_err(|_| ExitError::Other("decode storage verify info failed".into()))?;
				let proof = EthereumStorageProof::new(
					params.lane_address,
					params.storage_key,
					params.account_proof,
					params.storage_proof,
				);
				let storage_value = <T as StorageVerifier<[u8; 32]>>::verify_storage(&proof)
					.map_err(|_| ExitError::Other("verify storage proof failed".into()))?;
				abi_encode_bytes32(storage_value)
			}
		};

		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Stopped,
			cost: 20000,
			output,
			logs: Default::default(),
		})
	}
}
