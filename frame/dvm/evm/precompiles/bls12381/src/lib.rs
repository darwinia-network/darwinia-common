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

#[darwinia_evm_precompile_utils::selector]
enum Action {
	FastAggregateVerify = "fast_aggregate_verify(bytes[],bytes,bytes)",
}

pub struct BLS12381<T> {
	_market: PhantomData<T>,
}

impl<T> Precompile for StateStorage<T>
where
	T: darwinia_evm::Config,
{
	fn execute(
		input: &[u8],
		target_gas: Option<u64>,
		context: &Context,
		is_static: bool,
	) -> PrecompileResult {
		let mut helper = PrecompileHelper::<T>::new(input, target_gas);
		let (selector, data) = helper.split_input()?;
		let action = Action::from_u32(selector)?;

		// Check state modifiers
		helper.check_state_modifier(context, is_static, StateMutability::View)?;

		let output = match action {
			Action::FastAggregateVerify => {
				// Pure function (r:0 w:0)
				// helper.record_gas(0, 0)?;

				let params = FastAggregateVerifyParams::decode(data)
					.map_err(|_| helper.revert("Invalid input"))?;

				let sig = Signature::from_bytes(&params.signature[..]);
				if let Err(_e) = sig {
					helper.revert("Invalid signature");
				}

				let agg_sig = AggregateSignature::from_signature(&sig.unwrap());

				let public_keys_res: Result<Vec<milagro_bls::PublicKey>, _> = params
					.pubkeys
					.iter()
					.map(|bytes| milagro_bls::PublicKey::from_bytes_unchecked(&bytes.0))
					.collect();

				if let Err(_e) = public_keys_res {
					match _e {
						AmclError::InvalidPoint => helper.revert("Invalid point"),
						_ => helper.revert("Invalid pubkeys"),
					}
				}

				let agg_pub_key_res = AggregatePublicKey::into_aggregate(&public_keys_res.unwrap());
				if let Err(_e) = agg_pub_key_res {
					helper.revert("Invalid aggrete public keys");
				}

				agg_sig.fast_aggregate_verify_pre_aggregated(
					&params.message.as_bytes,
					&agg_pub_key_res.unwrap(),
				);
			},
		};

		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			cost: helper.used_gas(),
			output: abi_encode_bool(&output.unwrap_or_default()),
			logs: Default::default(),
		})
	}
}
