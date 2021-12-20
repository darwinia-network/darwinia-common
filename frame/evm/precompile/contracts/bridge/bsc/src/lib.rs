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
use sp_std::vec::Vec;
// --- darwinia-network ---
use darwinia_evm_precompile_utils::{selector, DvmInputParser};
use dp_contract::{abi_util::abi_encode_array_bytes32, bsc_light_client::BscStorageVerifyParams};
use dp_evm::Precompile;
use ethereum_primitives::{
	storage::{EthereumStorage, EthereumStorageProof},
	H256,
};

#[selector]
enum Action {
	// account, account_proof, storage_keys, storage_proofs
	VerifyStorageProof = "verify_storage_proof(address,bytes[],bytes32[],bytes[][])",
}

/// The contract address: 0000000000000000000000000000000000000026
pub struct BscBridge<T> {
	_marker: PhantomData<T>,
}

impl<T> Precompile for BscBridge<T>
where
	T: darwinia_bridge_bsc::Config,
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
				let latest_header = darwinia_bridge_bsc::Pallet::<T>::latest_bsc_header();
				if params.storage_keys.len() != params.storage_proofs.len() {
					return Err(ExitError::Other(
						"storage keys not match storage proofs".into(),
					));
				}
				let storage_values: Result<Vec<[u8; 32]>, _> = (0..params.storage_keys.len())
					.map(|idx| {
						let storage_key = params.storage_keys[idx];
						let storage_proof = params.storage_proofs[idx].clone();
						let proof = EthereumStorageProof::new(
							params.lane_address,
							storage_key,
							params.account_proof.clone(),
							storage_proof,
						);
						let storage_value = EthereumStorage::<H256>::verify_storage_proof(
							latest_header.state_root,
							&proof,
						)
						.map_err(|_| ExitError::Other("verify storage proof failed".into()))?;
						Ok(storage_value.0.into())
					})
					.collect();
				abi_encode_array_bytes32(storage_values?)
			}
		};

		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Stopped,
			cost: 10000 * params.storage_keys.len(),
			output,
			logs: Default::default(),
		})
	}
}
