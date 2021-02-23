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

use frame_support::traits::{Currency, ExistenceRequirement};
use sha3::Digest;
use sp_core::{H160, U256};
use sp_runtime::traits::UniqueSaturatedInto;
use sp_std::marker::PhantomData;
use sp_std::prelude::*;
use sp_std::vec::Vec;

use core::str::FromStr;
use darwinia_evm::{AddressMapping, Runner, Trait};
use darwinia_evm_primitives::Precompile;
use ethabi::{Function, Param, ParamType, Token};
use evm::{Context, ExitError, ExitSucceed};
use sp_std::borrow::ToOwned;

const DEPOSIT_FUNC: &[u8] = b"deposit(address,uint256)";
const KTON_PRECOMPILE: &str = "0000000000000000000000000000000000000016";
/// Kton Precompile Contract
///
/// The contract address: 0000000000000000000000000000000000000016
pub struct Kton<T: Trait> {
	_maker: PhantomData<T>,
}

impl<T: Trait> Precompile for Kton<T> {
	// The execute process of Kton precompile, the input data consists of four parts:
	// 1. Wkton solidity contract deployed address(20 bytes)
	// 2. The function selector(4 bytes)
	// 3. The deposit kton evm address(20 bytes)
	// 4. The deposit kton value(32 bytes)
	fn execute(
		input: &[u8],
		target_limit: Option<usize>,
		context: &Context,
	) -> core::result::Result<(ExitSucceed, Vec<u8>, usize), ExitError> {
		let helper = U256::from(10)
			.checked_pow(U256::from(9))
			.unwrap_or(U256::MAX);
		let deposit_func = &sha3::Keccak256::digest(&DEPOSIT_FUNC)[0..4];
		// decode input
		let input = InputData::decode(input)?;

		let con_caller = T::AddressMapping::into_account_id(context.caller);
		if hex::encode(input.func_selector) == hex::encode(&deposit_func) {
			// 1. Transfer kton from sender to kton erc20 contract
			let wkton_account_id = T::AddressMapping::into_account_id(input.wkton_address);
			let transfer_value = input.value.saturating_mul(helper);
			T::KtonCurrency::transfer(
				&con_caller,
				&wkton_account_id,
				transfer_value.low_u128().unique_saturated_into(),
				ExistenceRequirement::AllowDeath,
			)
			.map_err(|_| ExitError::Other("Transfer in Kton precompile failed".into()))?;

			// 2. Call wkton sol contract deposit
			let raw_input = make_deposit_input(input.address, input.value)?;
			let precompile_address = H160::from_str(KTON_PRECOMPILE).unwrap();
			T::Runner::call(
				precompile_address,
				input.wkton_address,
				raw_input.to_vec(),
				U256::zero(),
				target_limit.unwrap_or_default() as u32,
				None,
				None,
				T::config(),
			)
			.map_err(|_| ExitError::Other("Call in Kton precompile failed".into()))?;

			Ok((ExitSucceed::Returned, vec![], 20000))
		} else {
			Err(ExitError::Other(
				"Invalid func selector for kton precompile".into(),
			))
		}
	}
}

fn make_deposit_input(
	sp_address: sp_core::H160,
	sp_value: sp_core::U256,
) -> Result<Vec<u8>, ExitError> {
	// transfer address type
	let sp_address_bytes = sp_address.to_fixed_bytes();
	let eth_address = ethereum_types::H160::from_slice(&sp_address_bytes);
	// transfer value type
	let mut sp_value_bytes = [0u8; 32];
	sp_value.to_big_endian(&mut sp_value_bytes);
	let eth_value = ethereum_types::U256::from_big_endian(&sp_value_bytes);
	// encode process
	let func = Function {
		name: "deposit".to_owned(),
		inputs: vec![
			Param {
				name: "address".to_owned(),
				kind: ParamType::Address,
			},
			Param {
				name: "value".to_owned(),
				kind: ParamType::Uint(256),
			},
		],
		outputs: vec![],
		constant: false,
	};
	func.encode_input(&[Token::Address(eth_address), Token::Uint(eth_value)])
		.map_err(|_| ExitError::Other("Make deposit input error happened".into()))
}

#[derive(Debug, PartialEq, Eq)]
pub struct InputData {
	wkton_address: H160,
	func_selector: [u8; 4],
	address: H160,
	value: U256,
}

impl InputData {
	pub fn decode(data: &[u8]) -> Result<Self, ExitError> {
		if data.len() == 76 {
			let mut wkton_address_bytes = [0u8; 20];
			wkton_address_bytes.copy_from_slice(&data[0..20]);
			let wkton_address = H160::from_slice(&wkton_address_bytes);

			let mut func_selector_bytes = [0u8; 4];
			func_selector_bytes.copy_from_slice(&data[20..24]);

			let mut address_bytes = [0u8; 20];
			address_bytes.copy_from_slice(&data[24..44]);
			let address = H160::from_slice(&address_bytes);

			let mut value_bytes = [0u8; 32];
			value_bytes.copy_from_slice(&data[44..76]);
			let value = U256::from_big_endian(&value_bytes);

			return Ok(InputData {
				wkton_address,
				func_selector: func_selector_bytes,
				address,
				value,
			});
		}
		Err(ExitError::Other(
			"Invalid input data in kton precompile".into(),
		))
	}
}

#[cfg(test)]
mod testa {
	use super::*;
	use std::str::FromStr;

	#[test]
	fn test_make_input() {
		let mock_address =
			sp_core::H160::from_str("Aa01a1bEF0557fa9625581a293F3AA7770192632").unwrap();
		let mock_value_1 = sp_core::U256::from(30);
		let expected_str = "47e7ef24000000000000000000000000aa01a1bef0557fa9625581a293f3aa7770192632000000000000000000000000000000000000000000000000000000000000001e";

		let encoded_str = hex::encode(make_deposit_input(mock_address, mock_value_1).unwrap());
		assert_eq!(encoded_str, expected_str);

		let mock_value_2 = sp_core::U256::from(25);
		let encoded_str = hex::encode(make_deposit_input(mock_address, mock_value_2).unwrap());
		assert_ne!(encoded_str, expected_str);
	}
}
