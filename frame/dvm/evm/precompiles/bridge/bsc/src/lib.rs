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

#![cfg_attr(not(feature = "std"), no_std)]

// --- core ---
use core::marker::PhantomData;
// --- darwinia-network ---
use darwinia_evm_precompile_utils::{selector, DvmInputParser};
use dp_contract::{
	abi_util::{abi_encode_array_bytes, abi_encode_bytes},
	bsc_light_client::{BscMultiStorageVerifyParams, BscSingleStorageVerifyParams},
};
use ethereum_primitives::{
	error::{Error::Proof as StorageProofError, ProofError},
	storage::{EthereumStorage, EthereumStorageProof},
};
// --- paritytech ---
use fp_evm::{
	Context, ExitError, ExitSucceed, Precompile, PrecompileFailure, PrecompileOutput,
	PrecompileResult,
};
use sp_std::{vec, vec::Vec};

#[selector]
enum Action {
	// account, account_proof, storage_key, storage_proof
	VerfiySingleStorageProof = "verify_single_storage_proof(address,bytes[],bytes32,bytes[])",
	// account, account_proof, storage_keys, storage_proofs
	VerifyMultiStorageProof = "verify_multi_storage_proof(address,bytes[],bytes32[],bytes[][])",
}

const MAX_MULTI_STORAGEKEY_SIZE: usize = 32;

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
		_is_static: bool,
	) -> PrecompileResult {
		let dvm_parser = DvmInputParser::new(input)?;
		let (output, cost) = match Action::from_u32(dvm_parser.selector)? {
			Action::VerfiySingleStorageProof => {
				let params =
					BscSingleStorageVerifyParams::decode(dvm_parser.input).map_err(|_| {
						PrecompileFailure::Error {
							exit_status: ExitError::Other(
								"decode single storage verify info failed".into(),
							),
						}
					})?;
				let finalized_header = darwinia_bridge_bsc::Pallet::<T>::finalized_checkpoint();
				let proof = EthereumStorageProof::new(
					params.lane_address,
					params.storage_key,
					params.account_proof,
					params.storage_proof,
				);
				let storage_value = EthereumStorage::<Vec<u8>>::verify_storage_proof(
					finalized_header.state_root,
					&proof,
				)
				.map_err(|_| PrecompileFailure::Error {
					exit_status: ExitError::Other("verify single storage proof failed".into()),
				})?;
				(abi_encode_bytes(storage_value.0.as_slice()), 10000u64)
			}
			Action::VerifyMultiStorageProof => {
				let params =
					BscMultiStorageVerifyParams::decode(dvm_parser.input).map_err(|_| {
						PrecompileFailure::Error {
							exit_status: ExitError::Other(
								"decode multi storage verify info failed".into(),
							),
						}
					})?;
				let finalized_header = darwinia_bridge_bsc::Pallet::<T>::finalized_checkpoint();
				let key_size = params.storage_keys.len();
				if key_size != params.storage_proofs.len() {
					return Err(PrecompileFailure::Error {
						exit_status: ExitError::Other(
							"storage keys not match storage proofs".into(),
						),
					});
				}
				if key_size > MAX_MULTI_STORAGEKEY_SIZE {
					return Err(PrecompileFailure::Error {
						exit_status: ExitError::Other("storage keys size too large".into()),
					});
				}
				let storage_values: Result<Vec<Vec<u8>>, _> = (0..key_size)
					.map(|idx| {
						let storage_key = params.storage_keys[idx];
						let storage_proof = params.storage_proofs[idx].clone();
						let proof = EthereumStorageProof::new(
							params.lane_address,
							storage_key,
							params.account_proof.clone(),
							storage_proof,
						);
						let storage_value = EthereumStorage::<Vec<u8>>::verify_storage_proof(
							finalized_header.state_root,
							&proof,
						);
						match storage_value {
							Ok(value) => {
								return Ok(value.0.clone());
							}
							Err(err) => {
								// if the key not exist, we should return Zero Value due to the
								// Zero Value will not be stored.
								if err == StorageProofError(ProofError::TrieKeyNotExist) {
									return Ok(vec![]);
								} else {
									return Err(PrecompileFailure::Error {
										exit_status: ExitError::Other(
											"verfiy storage failed".into(),
										),
									});
								}
							}
						}
					})
					.collect();
				(
					abi_encode_array_bytes(storage_values?),
					10000 * key_size as u64,
				)
			}
		};

		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			cost,
			output,
			logs: Default::default(),
		})
	}
}
