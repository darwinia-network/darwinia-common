// This file is part of Substrate.

// Copyright (C) 2019-2020 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Consensus extension module tests for BABE consensus.

use super::*;
use codec::Decode;
use ethereum::TransactionSignature;
use frame_support::{assert_err, assert_noop, assert_ok, unsigned::ValidateUnsigned};
use mock::*;
use rustc_hex::{FromHex, ToHex};
use sp_runtime::transaction_validity::{InvalidTransaction, TransactionSource};
use std::str::FromStr;

// This ERC-20 contract mints the maximum amount of tokens to the contract creator.
// pragma solidity ^0.5.0;
// import "https://github.com/OpenZeppelin/openzeppelin-contracts/blob/v2.5.1/contracts/token/ERC20/ERC20.sol";
// contract MyToken is ERC20 {
//	 constructor() public { _mint(msg.sender, 2**256 - 1); }
// }
const ERC20_CONTRACT_BYTECODE: &str = "608060405234801561001057600080fd5b50610041337fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff61004660201b60201c565b610291565b600073ffffffffffffffffffffffffffffffffffffffff168273ffffffffffffffffffffffffffffffffffffffff1614156100e9576040517f08c379a000000000000000000000000000000000000000000000000000000000815260040180806020018281038252601f8152602001807f45524332303a206d696e7420746f20746865207a65726f20616464726573730081525060200191505060405180910390fd5b6101028160025461020960201b610c7c1790919060201c565b60028190555061015d816000808573ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020016000205461020960201b610c7c1790919060201c565b6000808473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020819055508173ffffffffffffffffffffffffffffffffffffffff16600073ffffffffffffffffffffffffffffffffffffffff167fddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef836040518082815260200191505060405180910390a35050565b600080828401905083811015610287576040517f08c379a000000000000000000000000000000000000000000000000000000000815260040180806020018281038252601b8152602001807f536166654d6174683a206164646974696f6e206f766572666c6f77000000000081525060200191505060405180910390fd5b8091505092915050565b610e3a806102a06000396000f3fe608060405234801561001057600080fd5b50600436106100885760003560e01c806370a082311161005b57806370a08231146101fd578063a457c2d714610255578063a9059cbb146102bb578063dd62ed3e1461032157610088565b8063095ea7b31461008d57806318160ddd146100f357806323b872dd146101115780633950935114610197575b600080fd5b6100d9600480360360408110156100a357600080fd5b81019080803573ffffffffffffffffffffffffffffffffffffffff16906020019092919080359060200190929190505050610399565b604051808215151515815260200191505060405180910390f35b6100fb6103b7565b6040518082815260200191505060405180910390f35b61017d6004803603606081101561012757600080fd5b81019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190803573ffffffffffffffffffffffffffffffffffffffff169060200190929190803590602001909291905050506103c1565b604051808215151515815260200191505060405180910390f35b6101e3600480360360408110156101ad57600080fd5b81019080803573ffffffffffffffffffffffffffffffffffffffff1690602001909291908035906020019092919050505061049a565b604051808215151515815260200191505060405180910390f35b61023f6004803603602081101561021357600080fd5b81019080803573ffffffffffffffffffffffffffffffffffffffff16906020019092919050505061054d565b6040518082815260200191505060405180910390f35b6102a16004803603604081101561026b57600080fd5b81019080803573ffffffffffffffffffffffffffffffffffffffff16906020019092919080359060200190929190505050610595565b604051808215151515815260200191505060405180910390f35b610307600480360360408110156102d157600080fd5b81019080803573ffffffffffffffffffffffffffffffffffffffff16906020019092919080359060200190929190505050610662565b604051808215151515815260200191505060405180910390f35b6103836004803603604081101561033757600080fd5b81019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190803573ffffffffffffffffffffffffffffffffffffffff169060200190929190505050610680565b6040518082815260200191505060405180910390f35b60006103ad6103a6610707565b848461070f565b6001905092915050565b6000600254905090565b60006103ce848484610906565b61048f846103da610707565b61048a85604051806060016040528060288152602001610d7060289139600160008b73ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020016000206000610440610707565b73ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002054610bbc9092919063ffffffff16565b61070f565b600190509392505050565b60006105436104a7610707565b8461053e85600160006104b8610707565b73ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060008973ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002054610c7c90919063ffffffff16565b61070f565b6001905092915050565b60008060008373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020549050919050565b60006106586105a2610707565b8461065385604051806060016040528060258152602001610de160259139600160006105cc610707565b73ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060008a73ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002054610bbc9092919063ffffffff16565b61070f565b6001905092915050565b600061067661066f610707565b8484610906565b6001905092915050565b6000600160008473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060008373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002054905092915050565b600033905090565b600073ffffffffffffffffffffffffffffffffffffffff168373ffffffffffffffffffffffffffffffffffffffff161415610795576040517f08c379a0000000000000000000000000000000000000000000000000000000008152600401808060200182810382526024815260200180610dbd6024913960400191505060405180910390fd5b600073ffffffffffffffffffffffffffffffffffffffff168273ffffffffffffffffffffffffffffffffffffffff16141561081b576040517f08c379a0000000000000000000000000000000000000000000000000000000008152600401808060200182810382526022815260200180610d286022913960400191505060405180910390fd5b80600160008573ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060008473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020819055508173ffffffffffffffffffffffffffffffffffffffff168373ffffffffffffffffffffffffffffffffffffffff167f8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925836040518082815260200191505060405180910390a3505050565b600073ffffffffffffffffffffffffffffffffffffffff168373ffffffffffffffffffffffffffffffffffffffff16141561098c576040517f08c379a0000000000000000000000000000000000000000000000000000000008152600401808060200182810382526025815260200180610d986025913960400191505060405180910390fd5b600073ffffffffffffffffffffffffffffffffffffffff168273ffffffffffffffffffffffffffffffffffffffff161415610a12576040517f08c379a0000000000000000000000000000000000000000000000000000000008152600401808060200182810382526023815260200180610d056023913960400191505060405180910390fd5b610a7d81604051806060016040528060268152602001610d4a602691396000808773ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002054610bbc9092919063ffffffff16565b6000808573ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002081905550610b10816000808573ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002054610c7c90919063ffffffff16565b6000808473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020819055508173ffffffffffffffffffffffffffffffffffffffff168373ffffffffffffffffffffffffffffffffffffffff167fddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef836040518082815260200191505060405180910390a3505050565b6000838311158290610c69576040517f08c379a00000000000000000000000000000000000000000000000000000000081526004018080602001828103825283818151815260200191508051906020019080838360005b83811015610c2e578082015181840152602081019050610c13565b50505050905090810190601f168015610c5b5780820380516001836020036101000a031916815260200191505b509250505060405180910390fd5b5060008385039050809150509392505050565b600080828401905083811015610cfa576040517f08c379a000000000000000000000000000000000000000000000000000000000815260040180806020018281038252601b8152602001807f536166654d6174683a206164646974696f6e206f766572666c6f77000000000081525060200191505060405180910390fd5b809150509291505056fe45524332303a207472616e7366657220746f20746865207a65726f206164647265737345524332303a20617070726f766520746f20746865207a65726f206164647265737345524332303a207472616e7366657220616d6f756e7420657863656564732062616c616e636545524332303a207472616e7366657220616d6f756e74206578636565647320616c6c6f77616e636545524332303a207472616e736665722066726f6d20746865207a65726f206164647265737345524332303a20617070726f76652066726f6d20746865207a65726f206164647265737345524332303a2064656372656173656420616c6c6f77616e63652062656c6f77207a65726fa265627a7a72315820c7a5ffabf642bda14700b2de42f8c57b36621af020441df825de45fd2b3e1c5c64736f6c63430005100032";
const WITHDRAW_DVM_ADDRESS: &str = "0000000000000000000000000000000000000015";
const WITH_DRAW_INPUT: &str = "723908ee9dc8e509d4b93251bd57f68c09bd9d04471c193fabd8f26c54284a4b";

fn default_erc20_creation_unsigned_transaction() -> UnsignedTransaction {
	UnsignedTransaction {
		nonce: U256::zero(),
		gas_price: U256::from(1),
		gas_limit: U256::from(0x100000),
		action: ethereum::TransactionAction::Create,
		value: U256::zero(),
		input: FromHex::from_hex(ERC20_CONTRACT_BYTECODE).unwrap(),
	}
}

fn default_withdraw_unsigned_transaction() -> UnsignedTransaction {
	UnsignedTransaction {
		nonce: U256::zero(),
		gas_price: U256::from(1),
		gas_limit: U256::from(0x100000),
		action: ethereum::TransactionAction::Call(H160::from_str(WITHDRAW_DVM_ADDRESS).unwrap()),
		value: U256::from(30000000000000000000u128),
		input: FromHex::from_hex(WITH_DRAW_INPUT).unwrap(),
	}
}

fn sign_transaction(account: &AccountInfo, unsign_tx: UnsignedTransaction) -> Transaction {
	unsign_tx.sign(&account.private_key)
}

#[test]
fn transaction_should_increment_nonce() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let t = sign_transaction(alice, default_erc20_creation_unsigned_transaction());
		assert_ok!(Ethereum::execute(
			alice.address,
			t.input,
			t.value,
			t.gas_limit,
			Some(t.gas_price),
			Some(t.nonce),
			t.action,
			None,
		));
		assert_eq!(
			<Test as darwinia_evm::Trait>::AccountBasicMapping::account_basic(&alice.address).nonce,
			U256::from(1)
		);
	});
}

#[test]
fn transaction_without_enough_gas_should_not_work() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let mut transaction =
			sign_transaction(alice, default_erc20_creation_unsigned_transaction());
		transaction.gas_price = U256::from(11_000_000);

		assert_err!(
			Ethereum::validate_unsigned(TransactionSource::External, &Call::transact(transaction)),
			InvalidTransaction::Payment
		);
	});
}

#[test]
fn transaction_with_invalid_nonce_should_not_work() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		// nonce is 0
		let mut transaction = default_erc20_creation_unsigned_transaction();
		transaction.nonce = U256::from(1);

		let signed = transaction.sign(&alice.private_key);

		assert_eq!(
			Ethereum::validate_unsigned(TransactionSource::External, &Call::transact(signed)),
			ValidTransactionBuilder::default()
				.and_provides((alice.address, U256::from(1)))
				.and_requires((alice.address, U256::from(0)))
				.build()
		);
		let t = sign_transaction(alice, default_erc20_creation_unsigned_transaction());

		// nonce is 1
		assert_ok!(Ethereum::execute(
			alice.address,
			t.input,
			t.value,
			t.gas_limit,
			Some(t.gas_price),
			Some(t.nonce),
			t.action,
			None,
		));

		transaction.nonce = U256::from(0);

		let signed2 = transaction.sign(&alice.private_key);

		assert_err!(
			Ethereum::validate_unsigned(TransactionSource::External, &Call::transact(signed2)),
			InvalidTransaction::Stale
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
		let t = sign_transaction(alice, default_erc20_creation_unsigned_transaction());
		assert_ok!(Ethereum::execute(
			alice.address,
			t.input,
			t.value,
			t.gas_limit,
			Some(t.gas_price),
			Some(t.nonce),
			t.action,
			None,
		));
		assert_eq!(
			Evm::account_storages(erc20_address, alice_storage_address),
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
			Origin::none(),
			sign_transaction(alice, default_erc20_creation_unsigned_transaction()),
		)
		.expect("Failed to execute transaction");

		// We verify the transaction happened with alice account.
		assert_eq!(
			Evm::account_storages(erc20_address, alice_storage_address),
			H256::from_str("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")
				.unwrap()
		)
	});
}

#[test]
fn invalid_signature_should_be_ignored() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	let mut transaction = sign_transaction(alice, default_erc20_creation_unsigned_transaction());
	transaction.signature = TransactionSignature::new(
		0x78,
		H256::from_slice(&[55u8; 32]),
		H256::from_slice(&[55u8; 32]),
	)
	.unwrap();
	ext.execute_with(|| {
		assert_noop!(
			Ethereum::transact(Origin::none(), transaction,),
			Error::<Test>::InvalidSignature
		);
	});
}

#[test]
fn contract_should_be_created_at_given_address() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	let erc20_address = contract_address(alice.address, 0);

	ext.execute_with(|| {
		let t = sign_transaction(alice, default_erc20_creation_unsigned_transaction());
		assert_ok!(Ethereum::execute(
			alice.address,
			t.input,
			t.value,
			t.gas_limit,
			Some(t.gas_price),
			Some(t.nonce),
			t.action,
			None,
		));
		assert_ne!(Evm::account_codes(erc20_address).len(), 0);
	});
}

#[test]
fn transaction_should_generate_correct_gas_used() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	let expected_gas = U256::from(891328);

	ext.execute_with(|| {
		let t = sign_transaction(alice, default_erc20_creation_unsigned_transaction());
		let (_, _, info) = Ethereum::execute(
			alice.address,
			t.input,
			t.value,
			t.gas_limit,
			Some(t.gas_price),
			Some(t.nonce),
			t.action,
			None,
		)
		.unwrap();

		match info {
			CallOrCreateInfo::Create(info) => {
				assert_eq!(info.used_gas, expected_gas);
			}
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
		let t = UnsignedTransaction {
			nonce: U256::zero(),
			gas_price: U256::from(1),
			gas_limit: U256::from(0x100000),
			action: ethereum::TransactionAction::Create,
			value: U256::zero(),
			input: FromHex::from_hex(contract).unwrap(),
		}
		.sign(&alice.private_key);
		assert_ok!(Ethereum::execute(
			alice.address,
			t.input,
			t.value,
			t.gas_limit,
			Some(t.gas_price),
			Some(t.nonce),
			t.action,
			None,
		));

		let contract_address: Vec<u8> =
			FromHex::from_hex("32dcab0ef3fb2de2fce1d2e0799d36239671f04a").unwrap();
		let foo: Vec<u8> = FromHex::from_hex("c2985578").unwrap();
		let bar: Vec<u8> = FromHex::from_hex("febb0f7e").unwrap();

		// calling foo will succeed
		let (_, _, info) = Ethereum::execute(
			alice.address,
			foo,
			U256::zero(),
			U256::from(1048576),
			Some(U256::from(1)),
			Some(U256::from(1)),
			TransactionAction::Call(H160::from_slice(&contract_address)),
			None,
		)
		.unwrap();
		match info {
			CallOrCreateInfo::Call(info) => {
				assert_eq!(
					info.value.to_hex::<String>(),
					"0000000000000000000000000000000000000000000000000000000000000001".to_owned()
				);
			}
			CallOrCreateInfo::Create(_) => panic!("expected call info"),
		}

		// calling should always succeed even if the inner EVM execution fails.
		Ethereum::execute(
			alice.address,
			bar,
			U256::zero(),
			U256::from(1048576),
			Some(U256::from(1)),
			Some(U256::from(2)),
			TransactionAction::Call(H160::from_slice(&contract_address)),
			None,
		)
		.ok()
		.unwrap();
	});
}

#[test]
fn withdraw_with_enough_balance() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let t = sign_transaction(alice, default_withdraw_unsigned_transaction());
		assert_ok!(Ethereum::execute(
			alice.address,
			t.input.clone(),
			t.value,
			t.gas_limit,
			None,
			Some(t.nonce),
			t.action,
			None,
		));

		// Check caller balance
		assert_eq!(
			<Test as darwinia_evm::Trait>::AccountBasicMapping::account_basic(&alice.address)
				.balance,
			U256::from(70_000_000_000_000_000_000u128)
		);
		// Check the target balance
		let input_bytes: Vec<u8> = FromHex::from_hex(WITH_DRAW_INPUT).unwrap();
		let dest = <Test as frame_system::Trait>::AccountId::decode(&mut &input_bytes[..]).unwrap();
		assert_eq!(
			<Test as Trait>::RingCurrency::free_balance(dest),
			30000000000
		);
	});
}

#[test]
fn withdraw_without_enough_balance_should_fail() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let mut unsigned_tx = default_withdraw_unsigned_transaction();
		unsigned_tx.value = U256::from(120000000000000000000u128);
		let t = sign_transaction(alice, unsigned_tx);

		let res = Ethereum::execute(
			alice.address,
			t.input,
			t.value,
			t.gas_limit,
			None,
			Some(t.nonce),
			t.action,
			None,
		);

		assert_err!(
			res,
			DispatchError::Module {
				index: 0,
				error: 0,
				message: Some("BalanceLow")
			}
		);

		// Check caller balance
		assert_eq!(
			<Test as darwinia_evm::Trait>::AccountBasicMapping::account_basic(&alice.address)
				.balance,
			U256::from(100000000000000000000u128)
		);
		// Check target balance
		let input_bytes: Vec<u8> = FromHex::from_hex(WITH_DRAW_INPUT).unwrap();
		let dest = <Test as frame_system::Trait>::AccountId::decode(&mut &input_bytes[..]).unwrap();
		assert_eq!(<Test as Trait>::RingCurrency::free_balance(&dest), 0);
	});
}
#[test]
fn withdraw_with_invalid_input_length_should_failed() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let t = sign_transaction(alice, default_withdraw_unsigned_transaction());
		// Invalid target address
		let mock_input = vec![0; 31];

		assert_ok!(Ethereum::execute(
			alice.address,
			mock_input,
			t.value,
			t.gas_limit,
			None,
			Some(t.nonce),
			t.action,
			None,
		));

		// Check caller balance
		assert_eq!(
			<Test as darwinia_evm::Trait>::AccountBasicMapping::account_basic(&alice.address)
				.balance,
			U256::from(100000000000000000000u128)
		);
		// Check target balance
		let input_bytes: Vec<u8> = FromHex::from_hex(WITH_DRAW_INPUT).unwrap();
		let dest = <Test as frame_system::Trait>::AccountId::decode(&mut &input_bytes[..]).unwrap();
		assert_eq!(<Test as Trait>::RingCurrency::free_balance(&dest), 0);
	});
}
