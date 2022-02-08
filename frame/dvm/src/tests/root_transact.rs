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

use super::*;
use array_bytes::{bytes2hex, hex2bytes_unchecked};
use std::str::FromStr;

#[test]
fn root_transact_invalid_origin_should_fail() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let t = LegacyUnsignedTransaction {
			nonce: U256::zero(),
			gas_price: U256::from(1),
			gas_limit: U256::from(0x100000),
			action: ethereum::TransactionAction::Create,
			value: U256::zero(),
			input: hex2bytes_unchecked(TEST_CONTRACT_BYTECODE),
		}
		.sign(&alice.private_key);
		// Deploy contract
		assert_ok!(Ethereum::execute(alice.address, &t, None,));
		let contract_address: H160 =
			array_bytes::hex_into_unchecked("32dcab0ef3fb2de2fce1d2e0799d36239671f04a");
		let add: Vec<u8> = hex2bytes_unchecked(
			"1003e2d20000000000000000000000000000000000000000000000000000000000000002",
		);

		assert_noop!(
			Ethereum::root_transact(Origin::none(), contract_address, add.clone()),
			sp_runtime::traits::BadOrigin,
		);
	});
}

// #[test]
// fn root_transact_should_works() {
// 	let (pairs, mut ext) = new_test_ext(1);
// 	let alice = &pairs[0];

// 	ext.execute_with(|| {
// 		let t = UnsignedTransaction {
// 			nonce: U256::zero(),
// 			gas_price: U256::from(1),
// 			gas_limit: U256::from(0x100000),
// 			action: ethereum::TransactionAction::Create,
// 			value: U256::zero(),
// 			input: hex2bytes_unchecked(TEST_CONTRACT_BYTECODE),
// 		}
// 		.sign(&alice.private_key);
// 		// Deploy contract
// 		assert_ok!(Ethereum::execute(
// 			alice.address,
// 			t.input,
// 			t.value,
// 			t.gas_limit,
// 			Some(t.gas_price),
// 			Some(t.nonce),
// 			t.action,
// 			None,
// 		));
// 		let contract_address: H160 =
// 			array_bytes::hex_into_unchecked("32dcab0ef3fb2de2fce1d2e0799d36239671f04a");
// 		let number: Vec<u8> = hex2bytes_unchecked("0x8381f58a");
// 		let add: Vec<u8> = hex2bytes_unchecked(
// 			"1003e2d20000000000000000000000000000000000000000000000000000000000000002",
// 		);

// 		assert_ok!(Ethereum::root_transact(
// 			Origin::root(),
// 			contract_address,
// 			add.clone()
// 		));

// 		let result = Ethereum::read_only_call(contract_address, number.clone()).unwrap();
// 		assert_eq!(
// 			result,
// 			vec![
// 				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
// 				0, 0, 0, 2
// 			]
// 		);
// 	});
// }

// #[test]
// fn root_transact_invalid_data_should_fail() {
// 	let (pairs, mut ext) = new_test_ext(1);
// 	let alice = &pairs[0];

// 	ext.execute_with(|| {
// 		let t = UnsignedTransaction {
// 			nonce: U256::zero(),
// 			gas_price: U256::from(1),
// 			gas_limit: U256::from(0x100000),
// 			action: ethereum::TransactionAction::Create,
// 			value: U256::zero(),
// 			input: hex2bytes_unchecked(TEST_CONTRACT_BYTECODE),
// 		}
// 		.sign(&alice.private_key);
// 		// Deploy contract
// 		assert_ok!(Ethereum::execute(
// 			alice.address,
// 			t.input,
// 			t.value,
// 			t.gas_limit,
// 			Some(t.gas_price),
// 			Some(t.nonce),
// 			t.action,
// 			None,
// 		));
// 		let contract_address: H160 =
// 			array_bytes::hex_into_unchecked("32dcab0ef3fb2de2fce1d2e0799d36239671f04a");
// 		let wrong_add: Vec<u8> = hex2bytes_unchecked(
// 			"0003e2d20000000000000000000000000000000000000000000000000000000000000002",
// 		);

// 		assert_err!(
// 			Ethereum::root_transact(Origin::root(), contract_address, wrong_add),
// 			<Error<Test>>::InternalTransactionRevertError
// 		);
// 	});
// }
