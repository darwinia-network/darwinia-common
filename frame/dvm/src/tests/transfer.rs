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
use crate::Config;
use array_bytes::{bytes2hex, hex2bytes_unchecked};
use codec::Decode;
use darwinia_evm::AccountBasic;
use darwinia_support::evm::{decimal_convert, TRANSFER_ADDR};
use ethabi::{Function, Param, ParamType, StateMutability, Token};
use sp_runtime::DispatchError;

const WITH_DRAW_INPUT: &str = "723908ee9dc8e509d4b93251bd57f68c09bd9d04471c193fabd8f26c54284a4b";
fn ring_withdraw_unsigned_transaction() -> LegacyUnsignedTransaction {
	LegacyUnsignedTransaction {
		nonce: U256::zero(),
		gas_price: U256::from(1),
		gas_limit: U256::from(0x100000),
		action: ethereum::TransactionAction::Call(H160::from_str(TRANSFER_ADDR).unwrap()),
		value: decimal_convert(30_000_000_000, None),
		input: hex2bytes_unchecked(WITH_DRAW_INPUT),
	}
}

fn ring_withdraw_creation_transaction(account: &AccountInfo) -> Transaction {
	ring_withdraw_unsigned_transaction().sign(&account.private_key)
}

#[test]
fn ring_currency_withdraw_with_enough_balance() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let t = ring_withdraw_creation_transaction(alice);
		assert_ok!(Ethereum::execute(alice.address, &t.into(), None,));

		// Check caller balance
		assert_eq!(
			RingAccount::account_basic(&alice.address).balance,
			// gas fee: 41512
			decimal_convert(70_000_000_000, None).saturating_sub(U256::from(41512))
		);
		// Check the dest balance
		let input_bytes: Vec<u8> = hex2bytes_unchecked(WITH_DRAW_INPUT);
		let dest =
			<Test as frame_system::Config>::AccountId::decode(&mut &input_bytes[..]).unwrap();
		assert_eq!(
			<Test as Config>::RingCurrency::free_balance(dest),
			30_000_000_000
		);
	});
}

#[test]
fn ring_currency_withdraw_not_enough_balance_should_fail() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let mut transaction = ring_withdraw_unsigned_transaction();
		transaction.value = decimal_convert(120_000_000_000, None);
		let t = transaction.sign(&alice.private_key);
		assert_err!(
			Ethereum::execute(alice.address, &t.into(), None,),
			DispatchError::Module {
				index: 4,
				error: 0,
				message: Some("BalanceLow")
			}
		);

		// Check caller balance
		assert_eq!(
			RingAccount::account_basic(&alice.address).balance,
			decimal_convert(100_000_000_000, None),
		);
		// Check target balance
		let input_bytes: Vec<u8> = hex2bytes_unchecked(WITH_DRAW_INPUT);
		let dest =
			<Test as frame_system::Config>::AccountId::decode(&mut &input_bytes[..]).unwrap();
		assert_eq!(<Test as Config>::RingCurrency::free_balance(&dest), 0);
	});
}

const WKTON_ADDRESS: &str = "32dcab0ef3fb2de2fce1d2e0799d36239671f04a";
const WKTON_CONTRACT_BYTECODE: &str = "60806040526040805190810160405280600d81526020017f5772617070656420434b544f4e00000000000000000000000000000000000000815250600090805190602001906200005192919062000112565b506040805190810160405280600681526020017f57434b544f4e0000000000000000000000000000000000000000000000000000815250600190805190602001906200009f92919062000112565b506012600260006101000a81548160ff021916908360ff1602179055506015600260016101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff1602179055503480156200010b57600080fd5b50620001c1565b828054600181600116156101000203166002900490600052602060002090601f016020900481019282601f106200015557805160ff191683800117855562000186565b8280016001018555821562000186579182015b828111156200018557825182559160200191906001019062000168565b5b50905062000195919062000199565b5090565b620001be91905b80821115620001ba576000816000905550600101620001a0565b5090565b90565b61100280620001d16000396000f3006080604052600436106100ba576000357c0100000000000000000000000000000000000000000000000000000000900463ffffffff168063040cf020146100bf57806306fdde03146100fa578063095ea7b31461018a57806318160ddd146101ef57806323b872dd1461021a578063313ce5671461029f57806347e7ef24146102d057806370a082311461031d57806395d89b4114610374578063a9059cbb14610404578063b548602014610469578063dd62ed3e146104c0575b600080fd5b3480156100cb57600080fd5b506100f8600480360381019080803560001916906020019092919080359060200190929190505050610537565b005b34801561010657600080fd5b5061010f6107ec565b6040518080602001828103825283818151815260200191508051906020019080838360005b8381101561014f578082015181840152602081019050610134565b50505050905090810190601f16801561017c5780820380516001836020036101000a031916815260200191505b509250505060405180910390f35b34801561019657600080fd5b506101d5600480360381019080803573ffffffffffffffffffffffffffffffffffffffff1690602001909291908035906020019092919050505061088a565b604051808215151515815260200191505060405180910390f35b3480156101fb57600080fd5b5061020461097c565b6040518082815260200191505060405180910390f35b34801561022657600080fd5b50610285600480360381019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190803573ffffffffffffffffffffffffffffffffffffffff16906020019092919080359060200190929190505050610986565b604051808215151515815260200191505060405180910390f35b3480156102ab57600080fd5b506102b4610cd3565b604051808260ff1660ff16815260200191505060405180910390f35b3480156102dc57600080fd5b5061031b600480360381019080803573ffffffffffffffffffffffffffffffffffffffff16906020019092919080359060200190929190505050610ce6565b005b34801561032957600080fd5b5061035e600480360381019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190505050610ec0565b6040518082815260200191505060405180910390f35b34801561038057600080fd5b50610389610ed8565b6040518080602001828103825283818151815260200191508051906020019080838360005b838110156103c95780820151818401526020810190506103ae565b50505050905090810190601f1680156103f65780820380516001836020036101000a031916815260200191505b509250505060405180910390f35b34801561041057600080fd5b5061044f600480360381019080803573ffffffffffffffffffffffffffffffffffffffff16906020019092919080359060200190929190505050610f76565b604051808215151515815260200191505060405180910390f35b34801561047557600080fd5b5061047e610f8b565b604051808273ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200191505060405180910390f35b3480156104cc57600080fd5b50610521600480360381019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190803573ffffffffffffffffffffffffffffffffffffffff169060200190929190505050610fb1565b6040518082815260200191505060405180910390f35b600081600460003373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020541015151561058757600080fd5b8160036000828254039250508190555081600460003373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060008282540392505081905550600260019054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1660405180807f776974686472617728627974657333322c75696e743235362900000000000000815250601901905060405180910390207c0100000000000000000000000000000000000000000000000000000000900484846040518363ffffffff167c0100000000000000000000000000000000000000000000000000000000028152600401808360001916600019168152602001828152602001925050506000604051808303816000875af1925050509050801515610745576040517f08c379a00000000000000000000000000000000000000000000000000000000081526004018080602001828103825260168152602001807f574b544f4e3a2057495448445241575f4641494c45440000000000000000000081525060200191505060405180910390fd5b600073ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff167fddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef846040518082815260200191505060405180910390a382600019167fa4dfdde26c326c8cced668e6a665f4efc3f278bdc9101cdedc4f725abd63a1ee836040518082815260200191505060405180910390a2505050565b60008054600181600116156101000203166002900480601f0160208091040260200160405190810160405280929190818152602001828054600181600116156101000203166002900480156108825780601f1061085757610100808354040283529160200191610882565b820191906000526020600020905b81548152906001019060200180831161086557829003601f168201915b505050505081565b600081600560003373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060008573ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020819055508273ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff167f8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925846040518082815260200191505060405180910390a36001905092915050565b6000600354905090565b600081600460008673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002054101515156109d657600080fd5b3373ffffffffffffffffffffffffffffffffffffffff168473ffffffffffffffffffffffffffffffffffffffff1614158015610aae57507fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff600560008673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060003373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020016000205414155b15610bc95781600560008673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060003373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020016000205410151515610b3e57600080fd5b81600560008673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060003373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020600082825403925050819055505b81600460008673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020016000206000828254039250508190555081600460008573ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020600082825401925050819055508273ffffffffffffffffffffffffffffffffffffffff168473ffffffffffffffffffffffffffffffffffffffff167fddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef846040518082815260200191505060405180910390a3600190509392505050565b600260009054906101000a900460ff1681565b600260019054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff16141515610dab576040517f08c379a00000000000000000000000000000000000000000000000000000000081526004018080602001828103825260118152602001807f574b544f4e3a205045524d495353494f4e00000000000000000000000000000081525060200191505060405180910390fd5b8060036000828254019250508190555080600460008473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020600082825401925050819055508173ffffffffffffffffffffffffffffffffffffffff16600073ffffffffffffffffffffffffffffffffffffffff167fddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef836040518082815260200191505060405180910390a38173ffffffffffffffffffffffffffffffffffffffff167fe1fffcc4923d04b559f4d29a8bfc6cda04eb5b0d3c460751c2402c5c5cc9109c826040518082815260200191505060405180910390a25050565b60046020528060005260406000206000915090505481565b60018054600181600116156101000203166002900480601f016020809104026020016040519081016040528092919081815260200182805460018160011615610100020316600290048015610f6e5780601f10610f4357610100808354040283529160200191610f6e565b820191906000526020600020905b815481529060010190602001808311610f5157829003601f168201915b505050505081565b6000610f83338484610986565b905092915050565b600260019054906101000a900473ffffffffffffffffffffffffffffffffffffffff1681565b60056020528160005260406000206020528060005260406000206000915091505054815600a165627a7a72305820e2f50a774ba846fa1c029233d81ae94557ebb22046bdc94b10c813c83a2c94660029";

fn transfer_and_call(address: H160, value: U256) -> Vec<u8> {
	#[allow(deprecated)]
	let func = Function {
		name: "transfer_and_call".to_owned(),
		inputs: vec![
			Param {
				name: "address".to_owned(),
				kind: ParamType::Address,
				internal_type: Some("address".into()),
			},
			Param {
				name: "value".to_owned(),
				kind: ParamType::Uint(256),
				internal_type: Some("uint256".into()),
			},
		],
		outputs: vec![],
		constant: false,
		state_mutability: StateMutability::NonPayable,
	};
	func.encode_input(&[Token::Address(address), Token::Uint(value)])
		.unwrap()
}

fn contract_balance_encode(address: H160) -> Vec<u8> {
	#[allow(deprecated)]
	let func = Function {
		name: "balanceOf".to_owned(),
		inputs: vec![Param {
			name: "address".to_owned(),
			kind: ParamType::Address,
			internal_type: Some("address".into()),
		}],
		outputs: vec![],
		constant: true,
		state_mutability: StateMutability::NonPayable,
	};
	func.encode_input(&[Token::Address(address)]).unwrap()
}

fn query_contract_balance(sender: &AccountInfo, nonce: u64) -> U256 {
	let t = LegacyUnsignedTransaction {
		nonce: U256::from(nonce),
		gas_price: U256::from(1),
		gas_limit: U256::from(0x300000),
		action: ethereum::TransactionAction::Call(H160::from_str(WKTON_ADDRESS).unwrap()),
		value: U256::from(0),
		input: hex2bytes_unchecked(bytes2hex("0x", contract_balance_encode(sender.address))),
	}
	.sign(&sender.private_key);

	if let Ok((_, _, res)) = Ethereum::execute(sender.address, &t.into(), None) {
		match res {
			CallOrCreateInfo::Call(info) => return U256::from_big_endian(&info.value),
			CallOrCreateInfo::Create(_) => return U256::default(),
		};
	}
	U256::default()
}

fn wkton_unsigned_transaction() -> LegacyUnsignedTransaction {
	LegacyUnsignedTransaction {
		nonce: U256::zero(),
		gas_price: U256::from(1),
		gas_limit: U256::from(0x100000),
		action: ethereum::TransactionAction::Create,
		value: U256::zero(),
		input: hex2bytes_unchecked(WKTON_CONTRACT_BYTECODE),
	}
}
fn wkton_creation_transaction(account: &AccountInfo) -> Transaction {
	wkton_unsigned_transaction().sign(&account.private_key)
}

fn transfer_and_call_transaction(
	address: H160,
	value: U256,
	nonce: u64,
) -> LegacyUnsignedTransaction {
	LegacyUnsignedTransaction {
		nonce: U256::from(nonce),
		gas_price: U256::from(1),
		gas_limit: U256::from(0x300000),
		action: ethereum::TransactionAction::Call(H160::from_str(TRANSFER_ADDR).unwrap()),
		value: U256::from(0),
		input: transfer_and_call(address, value),
	}
}

#[test]
fn kton_currency_transfer_and_call_works() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let origin = decimal_convert(70_000_000_000, None);
		KtonAccount::mutate_account_basic_balance(&alice.address, origin);
		assert_eq!(KtonAccount::account_basic(&alice.address).balance, origin);

		// Deploy WKTON contract
		let t = wkton_creation_transaction(alice);
		assert_ok!(Ethereum::execute(alice.address, &t.into(), None,));

		// Transfer and call 1
		let transfer = decimal_convert(30_000_000_000, None);
		let t = transfer_and_call_transaction(H160::from_str(WKTON_ADDRESS).unwrap(), transfer, 1)
			.sign(&alice.private_key);
		assert_ok!(Ethereum::execute(alice.address, &t.into(), None,));
		assert_eq!(
			KtonAccount::account_basic(&alice.address).balance,
			origin - transfer
		);
		assert_eq!(query_contract_balance(alice, 2), transfer);

		// Transfer and call 2
		let t = transfer_and_call_transaction(H160::from_str(WKTON_ADDRESS).unwrap(), transfer, 3)
			.sign(&alice.private_key);
		assert_ok!(Ethereum::execute(alice.address, &t.into(), None,));
		assert_eq!(
			KtonAccount::account_basic(&alice.address).balance,
			origin - transfer - transfer
		);
		assert_eq!(query_contract_balance(alice, 4), transfer + transfer);
	});
}

#[test]
fn kton_currency_transfer_and_call_out_of_fund() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let origin = decimal_convert(70_000_000_000, None);
		KtonAccount::mutate_account_basic_balance(&alice.address, origin);

		// Deploy WKTON contract
		let t = wkton_creation_transaction(alice);
		assert_ok!(Ethereum::execute(alice.address, &t.into(), None,));

		// Transfer and call
		let transfer = decimal_convert(90_000_000_000, None);
		// send_kton_transfer_and_call_tx(alice, H160::from_str(WKTON_ADDRESS).unwrap(), transfer, 1);
		let t = transfer_and_call_transaction(H160::from_str(WKTON_ADDRESS).unwrap(), transfer, 1)
			.sign(&alice.private_key);
		assert_ok!(Ethereum::execute(alice.address, &t.into(), None,));

		// Check balances
		assert_eq!(KtonAccount::account_basic(&alice.address).balance, origin);
		assert_eq!(query_contract_balance(alice, 2), U256::from(0));
	});
}

fn withdraw_encode(to: Vec<u8>, value: U256) -> Vec<u8> {
	#[allow(deprecated)]
	let func = Function {
		name: "withdraw".to_owned(),
		inputs: vec![
			Param {
				name: "to".to_owned(),
				kind: ParamType::FixedBytes(32),
				internal_type: Some("bytes32".into()),
			},
			Param {
				name: "value".to_owned(),
				kind: ParamType::Uint(256),
				internal_type: Some("uint256".into()),
			},
		],
		outputs: vec![],
		constant: false,
		state_mutability: StateMutability::NonPayable,
	};
	func.encode_input(&[Token::FixedBytes(to), Token::Uint(value)])
		.unwrap()
}

fn kton_withdraw_unsigned_transaction(
	to_id: Vec<u8>,
	value: U256,
	nonce: U256,
) -> LegacyUnsignedTransaction {
	LegacyUnsignedTransaction {
		nonce,
		gas_price: U256::from(1),
		gas_limit: U256::from(0x300000),
		action: ethereum::TransactionAction::Call(H160::from_str(WKTON_ADDRESS).unwrap()),
		value: U256::from(0),
		input: withdraw_encode(to_id, value),
	}
}

#[test]
fn kton_currency_withdraw() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let origin = decimal_convert(70_000_000_000, None);
		KtonAccount::mutate_account_basic_balance(&alice.address, origin);

		// Deploy WKTON contract
		let t = wkton_creation_transaction(alice);
		assert_ok!(Ethereum::execute(alice.address, &t.into(), None,));

		// Transfer and call
		let transfer = decimal_convert(30_000_000_000, None);
		let t = transfer_and_call_transaction(H160::from_str(WKTON_ADDRESS).unwrap(), transfer, 1)
			.sign(&alice.private_key);
		assert_ok!(Ethereum::execute(alice.address, &t.into(), None,));

		// withdraw
		let input_bytes: Vec<u8> = hex2bytes_unchecked(
			"0x64766d3a00000000000000aa01a1bef0557fa9625581a293f3aa777019263256",
		);
		let withdraw = decimal_convert(10_000_000_000, None);
		let t = kton_withdraw_unsigned_transaction(input_bytes.clone(), withdraw, U256::from(2))
			.sign(&alice.private_key);
		assert_ok!(Ethereum::execute(alice.address, &t.into(), None,));

		let to = <Test as frame_system::Config>::AccountId::decode(&mut &input_bytes[..]).unwrap();
		assert_eq!(KtonAccount::account_balance(&to), withdraw);
		assert_eq!(query_contract_balance(alice, 3), transfer - withdraw);
	});
}

#[test]
fn kton_currency_withdraw_out_of_fund() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let origin = decimal_convert(70_000_000_000, None);
		KtonAccount::mutate_account_basic_balance(&alice.address, origin);

		// Deploy WKTON contract
		let t = wkton_creation_transaction(alice);
		assert_ok!(Ethereum::execute(alice.address, &t.into(), None,));

		// Transfer and call
		let transfer = decimal_convert(30_000_000_000, None);
		let t = transfer_and_call_transaction(H160::from_str(WKTON_ADDRESS).unwrap(), transfer, 1)
			.sign(&alice.private_key);
		assert_ok!(Ethereum::execute(alice.address, &t.into(), None,));
		assert_eq!(query_contract_balance(alice, 2), transfer);

		// withdraw moro than transfered
		let input_bytes: Vec<u8> = hex2bytes_unchecked(
			"0x64766d3a00000000000000aa01a1bef0557fa9625581a293f3aa777019263256",
		);
		let withdraw = decimal_convert(50_000_000_000, None);
		let t = kton_withdraw_unsigned_transaction(input_bytes.clone(), withdraw, U256::from(3))
			.sign(&alice.private_key);
		assert_ok!(Ethereum::execute(alice.address, &t.into(), None,));

		// Check balances
		let to = <Test as frame_system::Config>::AccountId::decode(&mut &input_bytes[..]).unwrap();
		assert_eq!(KtonAccount::account_balance(&to), U256::from(0));
		assert_eq!(query_contract_balance(alice, 4), transfer);
	});
}
