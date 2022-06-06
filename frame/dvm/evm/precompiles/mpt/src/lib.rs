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
// --- crates.io ---
use evm::ExitRevert;
// --- darwinia-network ---
use darwinia_evm_precompile_utils::{PrecompileHelper, StateMutability};
use dp_contract::{
	abi_util::{abi_encode_array_bytes, abi_encode_bytes},
	mpt::{MPTMultiStorageVerifyParams, MPTSingleStorageVerifyParams},
};
use ethereum_primitives::{
	error::{Error::Proof as StorageProofError, ProofError},
	storage::{EthereumStorage, EthereumStorageProof},
};
// --- paritytech ---
use fp_evm::{
	Context, ExitSucceed, Precompile, PrecompileFailure, PrecompileOutput, PrecompileResult,
};
use sp_std::{vec, vec::Vec};

#[darwinia_evm_precompile_utils::selector]
enum Action {
	// account, account_proof, storage_key, storage_proof
	VerfiySingleStorageProof =
		"verify_single_storage_proof(bytes32,address,bytes[],bytes32,bytes[])",
	// account, account_proof, storage_keys, storage_proofs
	VerifyMultiStorageProof =
		"verify_multi_storage_proof(bytes32,address,bytes[],bytes32[],bytes[][])",
}

const VERIFY_SINGLE_STORAGE_GAS: usize = 50_000;

pub struct MPT<T> {
	_marker: PhantomData<T>,
}

impl<T> Precompile for MPT<T>
where
	T: darwinia_evm::Config,
{
	fn execute(
		input: &[u8],
		target_gas: Option<u64>,
		context: &Context,
		is_static: bool,
	) -> PrecompileResult {
		let helper = PrecompileHelper::<T>::new(input, target_gas);
		let (selector, data) = helper.split_input()?;
		let action = Action::from_u32(selector)?;

		// Check state modifiers
		helper.check_state_modifier(context, is_static, StateMutability::View)?;

		let mut multiplier: usize = 1;

		let output = match action {
			Action::VerfiySingleStorageProof => {
				let params = MPTSingleStorageVerifyParams::decode(data)
					.map_err(|_| helper.revert("decode single storage verify info failed"))?;
				let proof = EthereumStorageProof::new(
					params.lane_address,
					params.storage_key,
					params.account_proof,
					params.storage_proof,
				);
				let storage_value =
					EthereumStorage::<Vec<u8>>::verify_storage_proof(params.state_root, &proof)
						.map_err(|_| helper.revert("verify single storage proof failed"))?;
				abi_encode_bytes(storage_value.0.as_slice())
			},
			Action::VerifyMultiStorageProof => {
				let params = MPTMultiStorageVerifyParams::decode(data)
					.map_err(|_| helper.revert("decode multi storage verify info failed"))?;
				let key_size = params.storage_keys.len();
				if key_size != params.storage_proofs.len() {
					return Err(helper.revert("storage keys not match storage proofs"));
				}
				multiplier = key_size;

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
							params.state_root,
							&proof,
						);
						match storage_value {
							Ok(value) => {
								return Ok(value.0.clone());
							},
							Err(err) => {
								// if the key not exist, we should return Zero Value due to the
								// Zero Value will not be stored.
								if err == StorageProofError(ProofError::TrieKeyNotExist) {
									return Ok(vec![]);
								} else {
									return Err(helper.revert("verfiy storage failed"));
								}
							},
						}
					})
					.collect();
				abi_encode_array_bytes(storage_values?)
			},
		};

		let cost = multiplier
			.checked_mul(VERIFY_SINGLE_STORAGE_GAS)
			.ok_or(helper.revert("Calculate cost error"))?;

		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			cost: cost as u64,
			output,
			logs: Default::default(),
		})
	}
}
