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

#![cfg_attr(not(feature = "std"), no_std)]

// --- core ---
use core::marker::PhantomData;
// --- crates.io ---
use milagro_bls::{AggregatePublicKey, AggregateSignature, PublicKey, Signature};
// --- darwinia-network ---
use darwinia_evm_precompile_utils::{prelude::*, revert, PrecompileHelper};
// --- paritytech ---
use fp_evm::{
	Context, ExitRevert, ExitSucceed, Precompile, PrecompileFailure, PrecompileOutput,
	PrecompileResult,
};
use sp_std::vec::Vec;

#[selector]
enum Action {
	FastAggregateVerify = "fast_aggregate_verify(bytes[],bytes,bytes)",
}

pub struct BLS12381<T>(PhantomData<T>);

impl<T> Precompile for BLS12381<T>
where
	T: darwinia_evm::Config,
{
	fn execute(
		input: &[u8],
		target_gas: Option<u64>,
		context: &Context,
		is_static: bool,
	) -> PrecompileResult {
		let helper = PrecompileHelper::<T>::new(input, target_gas, context, is_static);
		let (selector, _) = helper.split_input()?;
		let action = Action::from_u32(selector)?;

		// Check state modifiers
		helper.check_state_modifier(StateMutability::View)?;

		let output = match action {
			Action::FastAggregateVerify => {
				let mut reader = helper.reader()?;
				reader.expect_arguments(3)?;
				let pubkeys = reader.read::<Vec<Bytes>>()?;
				let message = reader.read::<Bytes>()?;
				let signature = reader.read::<Bytes>()?;

				let sig = Signature::from_bytes(signature.as_bytes())
					.map_err(|_| revert("Invalid signature"))?;
				let agg_sig = AggregateSignature::from_signature(&sig);

				let public_keys_res: Result<Vec<PublicKey>, _> =
					pubkeys.iter().map(|bytes| PublicKey::from_bytes(bytes.as_bytes())).collect();

				if let Ok(keys) = public_keys_res {
					let agg_pub_key_res = AggregatePublicKey::into_aggregate(&keys)
						.map_err(|_| revert("Invalid aggregate"))?;

					agg_sig
						.fast_aggregate_verify_pre_aggregated(message.as_bytes(), &agg_pub_key_res)
				} else {
					return Err(revert("Invalid pubkeys"));
				}
			},
		};

		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			// TODO: https://github.com/darwinia-network/darwinia-common/issues/1261
			cost: helper.used_gas().saturating_add(100_000),
			output: EvmDataWriter::new().write(output).build(),
			logs: Default::default(),
		})
	}
}
