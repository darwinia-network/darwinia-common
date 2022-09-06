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
use dp_contract::{abi_util::abi_encode_bool, bls12381::FastAggregateVerifyParams};
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

const BLS_PUBKEY_LENGTH: usize = 48;
const BLS_SIGNATURE_LENGTH: usize = 96;

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

#[cfg(test)]
mod test {
	use darwinia_evm_precompile_utils::prelude::{Bytes, EvmDataReader};
	use dp_contract::bls12381::FastAggregateVerifyParams;
	use ethabi::{param_type::ParamType, token::Token, Error, Result as AbiResult};

	#[test]
	fn test_encode_decode() {
		let mock_pubkey_1 = vec![1; 48];
		let mock_pubkey_2 = vec![2; 48];
		let mock_pubkey_3 = vec![2; 48];
		let mock_message = vec![4; 10];
		let mock_sinature = vec![5; 96];

		let encoded = ethabi::encode(&[
			Token::Array(vec![
				Token::Bytes(mock_pubkey_1.clone()),
				Token::Bytes(mock_pubkey_2.clone()),
				Token::Bytes(mock_pubkey_3.clone()),
			]),
			Token::Bytes(mock_message.clone()),
			Token::Bytes(mock_sinature.clone()),
		]);

		let mut reader = EvmDataReader::new(&encoded);
		let pubkeys = reader.read::<Vec<Bytes>>().unwrap();
		let message = reader.read::<Bytes>().unwrap();
		let signature = reader.read::<Bytes>().unwrap();
		assert_eq!(pubkeys, vec![Bytes(mock_pubkey_1), Bytes(mock_pubkey_2), Bytes(mock_pubkey_3)]);
		assert_eq!(mock_message, message.0);
		assert_eq!(mock_sinature, signature.0);
	}
}
