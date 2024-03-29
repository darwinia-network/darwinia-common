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
use array_bytes::{bytes2hex, hex2bytes};
// --- darwinia-network ---
use super::*;

fn eip2930_erc20_creation_unsigned_transaction() -> EIP2930UnsignedTransaction {
	EIP2930UnsignedTransaction {
		nonce: U256::zero(),
		gas_price: U256::from(1),
		gas_limit: U256::from(0x100000),
		action: ethereum::TransactionAction::Create,
		value: U256::zero(),
		input: hex2bytes(ERC20_CONTRACT_BYTECODE).unwrap(),
	}
}

fn eip2930_erc20_creation_transaction(account: &AccountInfo) -> Transaction {
	eip2930_erc20_creation_unsigned_transaction().sign(&account.private_key, None)
}

#[test]
fn transaction_should_increment_nonce() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let t = eip2930_erc20_creation_transaction(alice);
		assert_ok!(Ethereum::execute(alice.address, &t.into(), None,));
		assert_eq!(EVM::account_basic(&alice.address).nonce, U256::from(1));
	});
}

#[test]
fn transaction_without_enough_gas_should_not_work() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let mut transaction = eip2930_erc20_creation_transaction(alice);
		match &mut transaction {
			Transaction::EIP2930(t) => t.gas_price = U256::from(11_000_000),
			_ => {},
		}

		let call = crate::Call::<Test>::transact { transaction };
		let source = call.check_self_contained().unwrap().unwrap();

		assert_err!(call.validate_self_contained(&source).unwrap(), InvalidTransaction::Payment);
	});
}

#[test]
fn transaction_with_to_low_nonce_should_not_work() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		// nonce is 0
		let mut transaction = eip2930_erc20_creation_unsigned_transaction();
		transaction.nonce = U256::from(1);

		let signed = transaction.sign(&alice.private_key, None);
		let call = crate::Call::<Test>::transact { transaction: signed };
		let source = call.check_self_contained().unwrap().unwrap();

		assert_eq!(
			call.validate_self_contained(&source).unwrap(),
			ValidTransactionBuilder::default()
				.and_provides((alice.address, U256::from(1)))
				.priority(0u64)
				.and_requires((alice.address, U256::from(0)))
				.build()
		);

		let t = eip2930_erc20_creation_transaction(alice);

		// nonce is 1
		assert_ok!(Ethereum::execute(alice.address, &t.into(), None,));

		transaction.nonce = U256::from(0);

		let signed2 = transaction.sign(&alice.private_key, None);
		let call2 = crate::Call::<Test>::transact { transaction: signed2 };
		let source2 = call2.check_self_contained().unwrap().unwrap();

		assert_err!(call2.validate_self_contained(&source2).unwrap(), InvalidTransaction::Stale);
	});
}

#[test]
fn transaction_with_to_hight_nonce_should_fail_in_block() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let mut transaction = eip2930_erc20_creation_unsigned_transaction();
		transaction.nonce = U256::one();

		let signed = transaction.sign(&alice.private_key, None);
		let call = crate::Call::<Test>::transact { transaction: signed };
		let source = call.check_self_contained().unwrap().unwrap();
		let extrinsic = fp_self_contained::CheckedExtrinsic::<_, _, SignedExtra, _> {
			signed: fp_self_contained::CheckedSignature::SelfContained(source),
			function: Call::Ethereum(call),
		};
		use frame_support::weights::GetDispatchInfo as _;
		let dispatch_info = extrinsic.get_dispatch_info();
		assert_err!(
			extrinsic.apply::<Test>(&dispatch_info, 0),
			TransactionValidityError::Invalid(InvalidTransaction::Future)
		);
	});
}

#[test]
fn transaction_with_invalid_chain_id_should_fail_in_block() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let transaction =
			eip2930_erc20_creation_unsigned_transaction().sign(&alice.private_key, Some(1));

		let call = crate::Call::<Test>::transact { transaction };
		let source = call.check_self_contained().unwrap().unwrap();
		let extrinsic = fp_self_contained::CheckedExtrinsic::<_, _, SignedExtra, _> {
			signed: fp_self_contained::CheckedSignature::SelfContained(source),
			function: Call::Ethereum(call),
		};
		use frame_support::weights::GetDispatchInfo as _;
		let dispatch_info = extrinsic.get_dispatch_info();
		assert_err!(
			extrinsic.apply::<Test>(&dispatch_info, 0),
			TransactionValidityError::Invalid(InvalidTransaction::Custom(
				crate::TransactionValidationError::InvalidChainId as u8,
			))
		);
	});
}

#[test]
fn contract_constructor_should_get_executed() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];
	let erc20_address = contract_address(alice.address, 0);
	let alice_storage_address = storage_address(alice.address, H256::zero());

	ext.execute_with(|| {
		let t = eip2930_erc20_creation_transaction(alice);

		assert_ok!(Ethereum::execute(alice.address, &t.into(), None,));
		assert_eq!(
			EVM::account_storages(erc20_address, alice_storage_address),
			H256::from_str("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")
				.unwrap()
		)
	});
}

#[test]
fn source_should_be_derived_from_signature() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	let erc20_address = contract_address(alice.address, 0);
	let alice_storage_address = storage_address(alice.address, H256::zero());

	ext.execute_with(|| {
		Ethereum::transact(
			RawOrigin::EthereumTransaction(alice.address).into(),
			eip2930_erc20_creation_transaction(alice),
		)
		.expect("Failed to execute transaction");

		// We verify the transaction happened with alice account.
		assert_eq!(
			EVM::account_storages(erc20_address, alice_storage_address),
			H256::from_str("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")
				.unwrap()
		)
	});
}

#[test]
fn contract_should_be_created_at_given_address() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	let erc20_address = contract_address(alice.address, 0);

	ext.execute_with(|| {
		let t = eip2930_erc20_creation_transaction(alice);
		assert_ok!(Ethereum::execute(alice.address, &t.into(), None,));
		assert_ne!(EVM::account_codes(erc20_address).len(), 0);
	});
}

#[test]
fn transaction_should_generate_correct_gas_used() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	let expected_gas = U256::from(893928);

	ext.execute_with(|| {
		let t = eip2930_erc20_creation_transaction(alice);
		let (_, _, info) = Ethereum::execute(alice.address, &t.into(), None).unwrap();

		match info {
			CallOrCreateInfo::Create(info) => {
				assert_eq!(info.used_gas, expected_gas);
			},
			CallOrCreateInfo::Call(_) => panic!("expected create info"),
		}
	});
}

#[test]
fn call_should_handle_errors() {
	// 	pragma solidity ^0.6.6;
	// 	contract Test {
	// 		function foo() external pure returns (bool) {
	// 			return true;
	// 		}
	// 		function bar() external pure {
	// 			require(false, "error_msg");
	// 		}
	// 	}
	let contract: &str = "608060405234801561001057600080fd5b50610113806100206000396000f3fe6080604052348015600f57600080fd5b506004361060325760003560e01c8063c2985578146037578063febb0f7e146057575b600080fd5b603d605f565b604051808215151515815260200191505060405180910390f35b605d6068565b005b60006001905090565b600060db576040517f08c379a00000000000000000000000000000000000000000000000000000000081526004018080602001828103825260098152602001807f6572726f725f6d7367000000000000000000000000000000000000000000000081525060200191505060405180910390fd5b56fea2646970667358221220fde68a3968e0e99b16fabf9b2997a78218b32214031f8e07e2c502daf603a69e64736f6c63430006060033";

	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let t = EIP2930UnsignedTransaction {
			nonce: U256::zero(),
			gas_price: U256::from(1),
			gas_limit: U256::from(0x100000),
			action: ethereum::TransactionAction::Create,
			value: U256::zero(),
			input: hex2bytes(contract).unwrap(),
		}
		.sign(&alice.private_key, None);
		assert_ok!(Ethereum::execute(alice.address, &t.into(), None,));

		let contract_address: Vec<u8> =
			hex2bytes("0x32dcab0ef3fb2de2fce1d2e0799d36239671f04a").unwrap();
		let foo: Vec<u8> = hex2bytes("0xc2985578").unwrap();
		let bar: Vec<u8> = hex2bytes("0xfebb0f7e").unwrap();

		let t2 = EIP2930UnsignedTransaction {
			nonce: U256::from(1),
			gas_price: U256::from(1),
			gas_limit: U256::from(0x100000),
			action: TransactionAction::Call(H160::from_slice(&contract_address)),
			value: U256::zero(),
			input: foo,
		}
		.sign(&alice.private_key, None);

		// calling foo will succeed
		let (_, _, info) = Ethereum::execute(alice.address, &t2.into(), None).unwrap();

		match info {
			CallOrCreateInfo::Call(info) => {
				assert_eq!(
					bytes2hex("", &info.value),
					"0000000000000000000000000000000000000000000000000000000000000001".to_owned()
				);
			},
			CallOrCreateInfo::Create(_) => panic!("expected call info"),
		}

		let t3 = EIP2930UnsignedTransaction {
			nonce: U256::from(2),
			gas_price: U256::from(1),
			gas_limit: U256::from(0x100000),
			action: TransactionAction::Call(H160::from_slice(&contract_address)),
			value: U256::zero(),
			input: bar,
		}
		.sign(&alice.private_key, None);

		// calling should always succeed even if the inner EVM execution fails.
		Ethereum::execute(alice.address, &t3.into(), None).ok().unwrap();
	});
}
