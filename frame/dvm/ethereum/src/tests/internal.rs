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

// --- crates.io ---
use array_bytes::hex2bytes_unchecked;
use fp_evm::{ExitReason, ExitSucceed};
use std::str::FromStr;
// --- darwinia-network ---
use super::*;
use crate::{Config, InternalTransactHandler};
use darwinia_evm::AccountBasic;
use darwinia_support::evm::IntoH160;

fn legacy_root_unsigned_transaction() -> LegacyUnsignedTransaction {
	LegacyUnsignedTransaction {
		nonce: U256::zero(),
		gas_price: U256::from(1),
		gas_limit: U256::from(0x100000),
		action: ethereum::TransactionAction::Create,
		value: U256::zero(),
		input: hex2bytes_unchecked(TEST_CONTRACT_BYTECODE),
	}
}

#[test]
fn root_transact_invalid_origin_should_fail() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let t = legacy_root_unsigned_transaction().sign(&alice.private_key);
		// Deploy contract
		assert_ok!(Ethereum::execute(alice.address, &t.into(), None,));
		let contract_address = contract_address(alice.address, 0);
		let add: Vec<u8> = hex2bytes_unchecked(
			"1003e2d20000000000000000000000000000000000000000000000000000000000000002",
		);

		assert_noop!(
			Ethereum::root_transact(Origin::none(), contract_address, add.clone()),
			sp_runtime::traits::BadOrigin,
		);
	});
}

#[test]
fn root_transact_should_works() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let t = legacy_root_unsigned_transaction().sign(&alice.private_key);
		// Deploy contract
		assert_ok!(Ethereum::execute(alice.address, &t.into(), None,));
		let contract_address = contract_address(alice.address, 0);
		let add: Vec<u8> = hex2bytes_unchecked(
			"1003e2d20000000000000000000000000000000000000000000000000000000000000002",
		);
		assert_ok!(Ethereum::root_transact(Origin::root(), contract_address, add.clone()));

		let number: Vec<u8> = hex2bytes_unchecked("0x8381f58a");
		let result = Ethereum::read_only_call(contract_address, number.clone()).unwrap();
		assert_eq!(
			result,
			vec![
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 2
			]
		);
	});
}

#[test]
fn root_transact_invalid_data_should_fail() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let t = legacy_root_unsigned_transaction().sign(&alice.private_key);
		// Deploy contract
		assert_ok!(Ethereum::execute(alice.address, &t.into(), None,));
		let contract_address = contract_address(alice.address, 0);
		let wrong_add: Vec<u8> = hex2bytes_unchecked(
			"0003e2d20000000000000000000000000000000000000000000000000000000000000002",
		);
		assert_err!(
			Ethereum::root_transact(Origin::root(), contract_address, wrong_add),
			<Error<Test>>::InternalTransactionRevertError
		);
	});
}

#[test]
fn read_only_call_should_works() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let t = legacy_root_unsigned_transaction().sign(&alice.private_key);
		// Deploy contract
		assert_ok!(Ethereum::execute(alice.address, &t.into(), None,));
		let contract_address = contract_address(alice.address, 0);
		let foo: Vec<u8> = hex2bytes_unchecked("c2985578");

		// Call foo use pallet dvm address
		let result = Ethereum::read_only_call(contract_address, foo).unwrap();
		assert_eq!(
			result,
			vec![
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 1
			]
		);
		// Check nonce
		let source = <Test as Config>::PalletId::get().into_h160();
		assert_eq!(RingAccount::account_basic(&source).nonce, U256::from(0));
	});
}

#[test]
fn read_only_call_should_not_change_storages() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let t = legacy_root_unsigned_transaction().sign(&alice.private_key);
		// Deploy contract
		assert_ok!(Ethereum::execute(alice.address, &t.into(), None,));
		let contract_address = contract_address(alice.address, 0);
		let number: Vec<u8> = hex2bytes_unchecked("0x8381f58a");
		let add: Vec<u8> = hex2bytes_unchecked(
			"1003e2d20000000000000000000000000000000000000000000000000000000000000002",
		);

		// internal transaction has ability to change storage
		assert_ok!(Ethereum::internal_transact(contract_address, add.clone()));
		let result = Ethereum::read_only_call(contract_address, number.clone()).unwrap();
		assert_eq!(
			result,
			vec![
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 2
			]
		);
		let old_root = sp_io::storage::root();

		// read only call does not change storage
		assert_ok!(Ethereum::read_only_call(contract_address, add.clone()));
		let result = Ethereum::read_only_call(contract_address, number.clone()).unwrap();
		assert_eq!(
			result,
			vec![
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 2
			]
		);
		let new_root = sp_io::storage::root();
		assert_eq!(old_root, new_root);
	});
}

#[test]
fn internal_transact_dispatch_error() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let t = legacy_root_unsigned_transaction().sign(&alice.private_key);
		// Deploy contract
		assert_ok!(Ethereum::execute(alice.address, &t.into(), None,));
		let contract_address = contract_address(alice.address, 0);
		let mock_foo: Vec<u8> = hex2bytes_unchecked("00000000");
		let source = <Test as self::Config>::PalletId::get().into_h160();

		// Call foo use internal transaction
		assert_err!(
			Ethereum::internal_transact(contract_address, mock_foo),
			<Error<Test>>::InternalTransactionRevertError
		);
		assert_eq!(RingAccount::account_basic(&source).nonce, U256::from(1));
	});
}

#[test]
fn internal_transaction_nonce_increase() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let t = legacy_root_unsigned_transaction().sign(&alice.private_key);
		// Deploy contract
		assert_ok!(Ethereum::execute(alice.address, &t.into(), None,));
		let contract_address = contract_address(alice.address, 0);
		let foo: Vec<u8> = hex2bytes_unchecked("c2985578");
		let source = <Test as self::Config>::PalletId::get().into_h160();

		// Call foo use internal transaction
		assert_ok!(Ethereum::internal_transact(contract_address, foo.clone()));
		assert_eq!(RingAccount::account_basic(&source).nonce, U256::from(1));

		assert_ok!(Ethereum::internal_transact(contract_address, foo));
		assert_eq!(RingAccount::account_basic(&source).nonce, U256::from(2));
	});
}

#[test]
fn internal_transaction_should_works() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let t = legacy_root_unsigned_transaction().sign(&alice.private_key);
		// Deploy contract
		assert_ok!(Ethereum::execute(alice.address, &t.into(), None,));
		let contract_address = contract_address(alice.address, 0);
		let foo: Vec<u8> = hex2bytes_unchecked("c2985578");

		// Call foo use internal transaction
		assert_ok!(Ethereum::internal_transact(contract_address, foo.clone()));
		assert_eq!(System::event_count(), 2);
		System::assert_last_event(Event::Ethereum(crate::Event::Executed(
			<Test as self::Config>::PalletId::get().into_h160(),
			contract_address,
			H256::from_str("0xabdebc2d8a79e4c40d6d66c614bafc2be138d4fc0fd21e28d318f3a032cbee39")
				.unwrap(),
			ExitReason::Succeed(ExitSucceed::Returned),
		)));

		assert_ok!(Ethereum::internal_transact(contract_address, foo));
		assert_eq!(System::event_count(), 3);
		System::assert_last_event(Event::Ethereum(crate::Event::Executed(
			<Test as self::Config>::PalletId::get().into_h160(),
			contract_address,
			H256::from_str("0x2028ce5eef8d4531d4f955c9860b28f9e8cd596b17fea2326d2be49a8d3dc7ac")
				.unwrap(),
			ExitReason::Succeed(ExitSucceed::Returned),
		)));
	});
}

#[test]
fn test_pallet_id_to_dvm_address() {
	let (_, mut ext) = new_test_ext(1);
	ext.execute_with(|| {
		assert_eq!(
			<Test as self::Config>::PalletId::get().into_h160(),
			H160::from_str("0x6d6f646c6461722f64766d700000000000000000").unwrap()
		)
	})
}
