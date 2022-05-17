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
use array_bytes::{bytes2hex, hex2bytes_unchecked};
use codec::Decode;
use ethabi::{Function, Param, ParamType, StateMutability, Token};
use std::str::FromStr;
// --- paritytech ---
use fp_evm::CallOrCreateInfo;
use frame_support::{assert_err, assert_ok};
use sp_core::{H160, U256};
use sp_runtime::DispatchError;
// --- darwinia-network ---
use crate::tests::mock::*;
use darwinia_ethereum::Transaction;
use darwinia_evm::AccountBasic;
use darwinia_evm_precompile_utils::{
	test_helper::{AccountInfo, LegacyUnsignedTransaction},
	PrecompileHelper,
};
use darwinia_support::evm::{decimal_convert, IntoAccountId, TRANSFER_ADDR};

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
	ring_withdraw_unsigned_transaction().sign_with_chain_id(&account.private_key, 42)
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
			// gas fee: 21512
			decimal_convert(70_000_000_000, None).saturating_sub(U256::from(21512))
		);
		// Check the dest balance
		let input_bytes: Vec<u8> = hex2bytes_unchecked(WITH_DRAW_INPUT);
		let dest =
			<Test as frame_system::Config>::AccountId::decode(&mut &input_bytes[..]).unwrap();
		assert_eq!(
			<Test as darwinia_ethereum::Config>::RingCurrency::free_balance(dest.clone()),
			30_000_000_000
		);

		let transfer_account_id = <Test as darwinia_evm::Config>::IntoAccountId::into_account_id(
			H160::from_str(TRANSFER_ADDR).unwrap(),
		);
		System::assert_has_event(Event::Ethereum(darwinia_ethereum::Event::DVMTransfer(
			transfer_account_id,
			dest,
			decimal_convert(30_000_000_000, None),
		)));
	});
}

#[test]
fn ring_currency_withdraw_not_enough_balance_should_fail() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let mut transaction = ring_withdraw_unsigned_transaction();
		transaction.value = decimal_convert(120_000_000_000, None);
		let t = transaction.sign_with_chain_id(&alice.private_key, 42);
		assert_err!(
			Ethereum::execute(alice.address, &t.into(), None,),
			DispatchError::Module { index: 4, error: 0, message: Some("BalanceLow") }
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
		assert_eq!(<Test as darwinia_ethereum::Config>::RingCurrency::free_balance(&dest), 0);
	});
}

const WKTON_ADDRESS: &str = "32dcab0ef3fb2de2fce1d2e0799d36239671f04a";
// WKTON Contract: https://github.com/darwinia-network/darwinia-bridge-sol/blob/master/contracts/tokens/contracts/WKTON.sol
const WKTON_CONTRACT_BYTECODE: &str = "60c0604052600d60808190527f5772617070656420434b544f4e0000000000000000000000000000000000000060a090815261003e91600091906100b4565b506040805180820190915260068082527f57434b544f4e00000000000000000000000000000000000000000000000000006020909201918252610083916001916100b4565b5060028054601260ff199091161761010060a860020a0319166115001790553480156100ae57600080fd5b5061014f565b828054600181600116156101000203166002900490600052602060002090601f016020900481019282601f106100f557805160ff1916838001178555610122565b82800160010185558215610122579182015b82811115610122578251825591602001919060010190610107565b5061012e929150610132565b5090565b61014c91905b8082111561012e5760008155600101610138565b90565b6108d78061015e6000396000f3006080604052600436106100b95763ffffffff7c0100000000000000000000000000000000000000000000000000000000600035041663040cf02081146100be57806306fdde03146100db578063095ea7b31461016557806318160ddd1461019d57806323b872dd146101c4578063313ce567146101ee57806347e7ef241461021957806370a082311461023d57806395d89b411461025e578063a9059cbb14610273578063b548602014610297578063dd62ed3e146102c8575b600080fd5b3480156100ca57600080fd5b506100d96004356024356102ef565b005b3480156100e757600080fd5b506100f06104a7565b6040805160208082528351818301528351919283929083019185019080838360005b8381101561012a578181015183820152602001610112565b50505050905090810190601f1680156101575780820380516001836020036101000a031916815260200191505b509250505060405180910390f35b34801561017157600080fd5b50610189600160a060020a0360043516602435610535565b604080519115158252519081900360200190f35b3480156101a957600080fd5b506101b261059b565b60408051918252519081900360200190f35b3480156101d057600080fd5b50610189600160a060020a03600435811690602435166044356105a1565b3480156101fa57600080fd5b506102036106d5565b6040805160ff9092168252519081900360200190f35b34801561022557600080fd5b506100d9600160a060020a03600435166024356106de565b34801561024957600080fd5b506101b2600160a060020a03600435166107fa565b34801561026a57600080fd5b506100f061080c565b34801561027f57600080fd5b50610189600160a060020a0360043516602435610866565b3480156102a357600080fd5b506102ac61087a565b60408051600160a060020a039092168252519081900360200190f35b3480156102d457600080fd5b506101b2600160a060020a036004358116906024351661088e565b3360009081526004602052604081205482111561030b57600080fd5b6003805483900390553360009081526004602081905260408083208054869003905560025481517f776974686472617728627974657333322c75696e7432353629000000000000008152825190819003601901812063ffffffff7c0100000000000000000000000000000000000000000000000000000000918290049081169091028252938101889052602481018790529151610100909104600160a060020a031693604480840193919291829003018183875af192505050905080151561043457604080517f08c379a000000000000000000000000000000000000000000000000000000000815260206004820152601660248201527f574b544f4e3a2057495448445241575f4641494c454400000000000000000000604482015290519081900360640190fd5b60408051838152905160009133917fddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef9181900360200190a360408051838152905184917fa4dfdde26c326c8cced668e6a665f4efc3f278bdc9101cdedc4f725abd63a1ee919081900360200190a2505050565b6000805460408051602060026001851615610100026000190190941693909304601f8101849004840282018401909252818152929183018282801561052d5780601f106105025761010080835404028352916020019161052d565b820191906000526020600020905b81548152906001019060200180831161051057829003601f168201915b505050505081565b336000818152600560209081526040808320600160a060020a038716808552908352818420869055815186815291519394909390927f8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925928290030190a350600192915050565b60035490565b600160a060020a0383166000908152600460205260408120548211156105c657600080fd5b600160a060020a03841633148015906106045750600160a060020a038416600090815260056020908152604080832033845290915290205460001914155b1561066457600160a060020a038416600090815260056020908152604080832033845290915290205482111561063957600080fd5b600160a060020a03841660009081526005602090815260408083203384529091529020805483900390555b600160a060020a03808516600081815260046020908152604080832080548890039055938716808352918490208054870190558351868152935191937fddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef929081900390910190a35060019392505050565b60025460ff1681565b6002546101009004600160a060020a0316331461075c57604080517f08c379a000000000000000000000000000000000000000000000000000000000815260206004820152601160248201527f574b544f4e3a205045524d495353494f4e000000000000000000000000000000604482015290519081900360640190fd5b6003805482019055600160a060020a0382166000818152600460209081526040808320805486019055805185815290517fddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef929181900390910190a3604080518281529051600160a060020a038416917fe1fffcc4923d04b559f4d29a8bfc6cda04eb5b0d3c460751c2402c5c5cc9109c919081900360200190a25050565b60046020526000908152604090205481565b60018054604080516020600284861615610100026000190190941693909304601f8101849004840282018401909252818152929183018282801561052d5780601f106105025761010080835404028352916020019161052d565b60006108733384846105a1565b9392505050565b6002546101009004600160a060020a031681565b6005602090815260009283526040808420909152908252902054815600a165627a7a7230582020cd4921c934cee383654530f0b5a3882f5b1fd106f71058e1a06af9d2843bee0029";

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
	func.encode_input(&[Token::Address(address), Token::Uint(value)]).unwrap()
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
	.sign_with_chain_id(&sender.private_key, 42);

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
	wkton_unsigned_transaction().sign_with_chain_id(&account.private_key, 42)
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
fn kton_make_call_works() {
	let (_, mut ext) = new_test_ext(1);

	ext.execute_with(|| {
		let helper = PrecompileHelper::<Test>::new(&[], Some(100));
		let mock_address =
			H160::from_str("Aa01a1bEF0557fa9625581a293F3AA7770192632").unwrap();
		let mock_value = U256::from(30);
		let expected_str = "0x47e7ef24000000000000000000000000aa01a1bef0557fa9625581a293f3aa7770192632000000000000000000000000000000000000000000000000000000000000001e";
		let encoded_str =
			bytes2hex("0x", crate::kton::Kton::<Test>::make_call_data(mock_address, mock_value, &helper).unwrap());
		assert_eq!(encoded_str, expected_str);

		let mock_value = sp_core::U256::from(25);
		let encoded_str =
			bytes2hex("0x", crate::kton::Kton::<Test>::make_call_data(mock_address, mock_value, &helper).unwrap());
			assert_ne!(encoded_str, expected_str);
	});
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
		let transfer_1 = decimal_convert(30_000_000_000, None);
		let t =
			transfer_and_call_transaction(H160::from_str(WKTON_ADDRESS).unwrap(), transfer_1, 1)
				.sign_with_chain_id(&alice.private_key, 42);
		assert_ok!(Ethereum::execute(alice.address, &t.into(), None,));
		assert_eq!(KtonAccount::account_basic(&alice.address).balance, origin - transfer_1);
		assert_eq!(query_contract_balance(alice, 2), transfer_1);
		let alice_account_id =
			<Test as darwinia_evm::Config>::IntoAccountId::into_account_id(alice.address);
		let wkton_account_id = <Test as darwinia_evm::Config>::IntoAccountId::into_account_id(
			H160::from_str(WKTON_ADDRESS).unwrap(),
		);
		System::assert_has_event(Event::Ethereum(darwinia_ethereum::Event::KtonDVMTransfer(
			alice_account_id.clone(),
			wkton_account_id.clone(),
			transfer_1,
		)));

		// Transfer and call 2
		let transfer_2 = decimal_convert(20_000_000_000, None);
		let t =
			transfer_and_call_transaction(H160::from_str(WKTON_ADDRESS).unwrap(), transfer_2, 3)
				.sign_with_chain_id(&alice.private_key, 42);
		assert_ok!(Ethereum::execute(alice.address, &t.into(), None,));
		assert_eq!(
			KtonAccount::account_basic(&alice.address).balance,
			origin - transfer_1 - transfer_2
		);
		assert_eq!(query_contract_balance(alice, 4), transfer_1 + transfer_2);
		System::assert_has_event(Event::Ethereum(darwinia_ethereum::Event::KtonDVMTransfer(
			alice_account_id,
			wkton_account_id,
			transfer_2,
		)));
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
		// send_kton_transfer_and_call_tx(alice, H160::from_str(WKTON_ADDRESS).unwrap(), transfer,
		// 1);
		let t = transfer_and_call_transaction(H160::from_str(WKTON_ADDRESS).unwrap(), transfer, 1)
			.sign_with_chain_id(&alice.private_key, 42);
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
	func.encode_input(&[Token::FixedBytes(to), Token::Uint(value)]).unwrap()
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
			.sign_with_chain_id(&alice.private_key, 42);
		assert_ok!(Ethereum::execute(alice.address, &t.into(), None,));

		// withdraw
		let input_bytes: Vec<u8> = hex2bytes_unchecked(
			"0x64766d3a00000000000000aa01a1bef0557fa9625581a293f3aa777019263256",
		);
		let withdraw = decimal_convert(10_000_000_000, None);
		let t = kton_withdraw_unsigned_transaction(input_bytes.clone(), withdraw, U256::from(2))
			.sign_with_chain_id(&alice.private_key, 42);
		assert_ok!(Ethereum::execute(alice.address, &t.into(), None,));

		let to = <Test as frame_system::Config>::AccountId::decode(&mut &input_bytes[..]).unwrap();
		assert_eq!(KtonAccount::account_balance(&to), withdraw);
		assert_eq!(query_contract_balance(alice, 3), transfer - withdraw);

		let wkton_account_id = <Test as darwinia_evm::Config>::IntoAccountId::into_account_id(
			H160::from_str(WKTON_ADDRESS).unwrap(),
		);
		System::assert_has_event(Event::Ethereum(darwinia_ethereum::Event::KtonDVMTransfer(
			wkton_account_id,
			to,
			withdraw,
		)));
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
			.sign_with_chain_id(&alice.private_key, 42);
		assert_ok!(Ethereum::execute(alice.address, &t.into(), None,));
		assert_eq!(query_contract_balance(alice, 2), transfer);

		// withdraw moro than transfered
		let input_bytes: Vec<u8> = hex2bytes_unchecked(
			"0x64766d3a00000000000000aa01a1bef0557fa9625581a293f3aa777019263256",
		);
		let withdraw = decimal_convert(50_000_000_000, None);
		let t = kton_withdraw_unsigned_transaction(input_bytes.clone(), withdraw, U256::from(3))
			.sign_with_chain_id(&alice.private_key, 42);
		assert_ok!(Ethereum::execute(alice.address, &t.into(), None,));

		// Check balances
		let to = <Test as frame_system::Config>::AccountId::decode(&mut &input_bytes[..]).unwrap();
		assert_eq!(KtonAccount::account_balance(&to), U256::from(0));
		assert_eq!(query_contract_balance(alice, 4), transfer);
	});
}
