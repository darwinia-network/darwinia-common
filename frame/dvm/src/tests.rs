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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

// --- crates ---
use array_bytes::{bytes2hex, hex2bytes_unchecked};
use codec::Decode;
use ethabi::{Function, Param, ParamType, Token};
use ethereum::TransactionSignature;
use std::str::FromStr;
// --- darwinia ---
use crate::{
	account_basic::{RemainBalanceOp, RingRemainBalance},
	Call, *,
};
use darwinia_evm::AddressMapping;
use darwinia_support::evm::TRANSFER_ADDR;
use mock::*;
// --- substrate ---
use frame_support::{assert_err, assert_noop, assert_ok, unsigned::ValidateUnsigned};
use sp_runtime::transaction_validity::{InvalidTransaction, TransactionSource};

// This ERC-20 contract mints the maximum amount of tokens to the contract creator.
// pragma solidity ^0.5.0;
// import "https://github.com/OpenZeppelin/openzeppelin-contracts/blob/v2.5.1/contracts/token/ERC20/ERC20.sol";
// contract MyToken is ERC20 {
//	 constructor() public { _mint(msg.sender, 2**256 - 1); }
// }
const ERC20_CONTRACT_BYTECODE: &str = "608060405234801561001057600080fd5b50610041337fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff61004660201b60201c565b610291565b600073ffffffffffffffffffffffffffffffffffffffff168273ffffffffffffffffffffffffffffffffffffffff1614156100e9576040517f08c379a000000000000000000000000000000000000000000000000000000000815260040180806020018281038252601f8152602001807f45524332303a206d696e7420746f20746865207a65726f20616464726573730081525060200191505060405180910390fd5b6101028160025461020960201b610c7c1790919060201c565b60028190555061015d816000808573ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020016000205461020960201b610c7c1790919060201c565b6000808473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020819055508173ffffffffffffffffffffffffffffffffffffffff16600073ffffffffffffffffffffffffffffffffffffffff167fddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef836040518082815260200191505060405180910390a35050565b600080828401905083811015610287576040517f08c379a000000000000000000000000000000000000000000000000000000000815260040180806020018281038252601b8152602001807f536166654d6174683a206164646974696f6e206f766572666c6f77000000000081525060200191505060405180910390fd5b8091505092915050565b610e3a806102a06000396000f3fe608060405234801561001057600080fd5b50600436106100885760003560e01c806370a082311161005b57806370a08231146101fd578063a457c2d714610255578063a9059cbb146102bb578063dd62ed3e1461032157610088565b8063095ea7b31461008d57806318160ddd146100f357806323b872dd146101115780633950935114610197575b600080fd5b6100d9600480360360408110156100a357600080fd5b81019080803573ffffffffffffffffffffffffffffffffffffffff16906020019092919080359060200190929190505050610399565b604051808215151515815260200191505060405180910390f35b6100fb6103b7565b6040518082815260200191505060405180910390f35b61017d6004803603606081101561012757600080fd5b81019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190803573ffffffffffffffffffffffffffffffffffffffff169060200190929190803590602001909291905050506103c1565b604051808215151515815260200191505060405180910390f35b6101e3600480360360408110156101ad57600080fd5b81019080803573ffffffffffffffffffffffffffffffffffffffff1690602001909291908035906020019092919050505061049a565b604051808215151515815260200191505060405180910390f35b61023f6004803603602081101561021357600080fd5b81019080803573ffffffffffffffffffffffffffffffffffffffff16906020019092919050505061054d565b6040518082815260200191505060405180910390f35b6102a16004803603604081101561026b57600080fd5b81019080803573ffffffffffffffffffffffffffffffffffffffff16906020019092919080359060200190929190505050610595565b604051808215151515815260200191505060405180910390f35b610307600480360360408110156102d157600080fd5b81019080803573ffffffffffffffffffffffffffffffffffffffff16906020019092919080359060200190929190505050610662565b604051808215151515815260200191505060405180910390f35b6103836004803603604081101561033757600080fd5b81019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190803573ffffffffffffffffffffffffffffffffffffffff169060200190929190505050610680565b6040518082815260200191505060405180910390f35b60006103ad6103a6610707565b848461070f565b6001905092915050565b6000600254905090565b60006103ce848484610906565b61048f846103da610707565b61048a85604051806060016040528060288152602001610d7060289139600160008b73ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020016000206000610440610707565b73ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002054610bbc9092919063ffffffff16565b61070f565b600190509392505050565b60006105436104a7610707565b8461053e85600160006104b8610707565b73ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060008973ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002054610c7c90919063ffffffff16565b61070f565b6001905092915050565b60008060008373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020549050919050565b60006106586105a2610707565b8461065385604051806060016040528060258152602001610de160259139600160006105cc610707565b73ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060008a73ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002054610bbc9092919063ffffffff16565b61070f565b6001905092915050565b600061067661066f610707565b8484610906565b6001905092915050565b6000600160008473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060008373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002054905092915050565b600033905090565b600073ffffffffffffffffffffffffffffffffffffffff168373ffffffffffffffffffffffffffffffffffffffff161415610795576040517f08c379a0000000000000000000000000000000000000000000000000000000008152600401808060200182810382526024815260200180610dbd6024913960400191505060405180910390fd5b600073ffffffffffffffffffffffffffffffffffffffff168273ffffffffffffffffffffffffffffffffffffffff16141561081b576040517f08c379a0000000000000000000000000000000000000000000000000000000008152600401808060200182810382526022815260200180610d286022913960400191505060405180910390fd5b80600160008573ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060008473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020819055508173ffffffffffffffffffffffffffffffffffffffff168373ffffffffffffffffffffffffffffffffffffffff167f8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925836040518082815260200191505060405180910390a3505050565b600073ffffffffffffffffffffffffffffffffffffffff168373ffffffffffffffffffffffffffffffffffffffff16141561098c576040517f08c379a0000000000000000000000000000000000000000000000000000000008152600401808060200182810382526025815260200180610d986025913960400191505060405180910390fd5b600073ffffffffffffffffffffffffffffffffffffffff168273ffffffffffffffffffffffffffffffffffffffff161415610a12576040517f08c379a0000000000000000000000000000000000000000000000000000000008152600401808060200182810382526023815260200180610d056023913960400191505060405180910390fd5b610a7d81604051806060016040528060268152602001610d4a602691396000808773ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002054610bbc9092919063ffffffff16565b6000808573ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002081905550610b10816000808573ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002054610c7c90919063ffffffff16565b6000808473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020819055508173ffffffffffffffffffffffffffffffffffffffff168373ffffffffffffffffffffffffffffffffffffffff167fddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef836040518082815260200191505060405180910390a3505050565b6000838311158290610c69576040517f08c379a00000000000000000000000000000000000000000000000000000000081526004018080602001828103825283818151815260200191508051906020019080838360005b83811015610c2e578082015181840152602081019050610c13565b50505050905090810190601f168015610c5b5780820380516001836020036101000a031916815260200191505b509250505060405180910390fd5b5060008385039050809150509392505050565b600080828401905083811015610cfa576040517f08c379a000000000000000000000000000000000000000000000000000000000815260040180806020018281038252601b8152602001807f536166654d6174683a206164646974696f6e206f766572666c6f77000000000081525060200191505060405180910390fd5b809150509291505056fe45524332303a207472616e7366657220746f20746865207a65726f206164647265737345524332303a20617070726f766520746f20746865207a65726f206164647265737345524332303a207472616e7366657220616d6f756e7420657863656564732062616c616e636545524332303a207472616e7366657220616d6f756e74206578636565647320616c6c6f77616e636545524332303a207472616e736665722066726f6d20746865207a65726f206164647265737345524332303a20617070726f76652066726f6d20746865207a65726f206164647265737345524332303a2064656372656173656420616c6c6f77616e63652062656c6f77207a65726fa265627a7a72315820c7a5ffabf642bda14700b2de42f8c57b36621af020441df825de45fd2b3e1c5c64736f6c63430005100032";
const WKTON_CONTRACT_BYTECODE: &str = "60806040526040805190810160405280600d81526020017f5772617070656420434b544f4e00000000000000000000000000000000000000815250600090805190602001906200005192919062000112565b506040805190810160405280600681526020017f57434b544f4e0000000000000000000000000000000000000000000000000000815250600190805190602001906200009f92919062000112565b506012600260006101000a81548160ff021916908360ff1602179055506015600260016101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff1602179055503480156200010b57600080fd5b50620001c1565b828054600181600116156101000203166002900490600052602060002090601f016020900481019282601f106200015557805160ff191683800117855562000186565b8280016001018555821562000186579182015b828111156200018557825182559160200191906001019062000168565b5b50905062000195919062000199565b5090565b620001be91905b80821115620001ba576000816000905550600101620001a0565b5090565b90565b61100280620001d16000396000f3006080604052600436106100ba576000357c0100000000000000000000000000000000000000000000000000000000900463ffffffff168063040cf020146100bf57806306fdde03146100fa578063095ea7b31461018a57806318160ddd146101ef57806323b872dd1461021a578063313ce5671461029f57806347e7ef24146102d057806370a082311461031d57806395d89b4114610374578063a9059cbb14610404578063b548602014610469578063dd62ed3e146104c0575b600080fd5b3480156100cb57600080fd5b506100f8600480360381019080803560001916906020019092919080359060200190929190505050610537565b005b34801561010657600080fd5b5061010f6107ec565b6040518080602001828103825283818151815260200191508051906020019080838360005b8381101561014f578082015181840152602081019050610134565b50505050905090810190601f16801561017c5780820380516001836020036101000a031916815260200191505b509250505060405180910390f35b34801561019657600080fd5b506101d5600480360381019080803573ffffffffffffffffffffffffffffffffffffffff1690602001909291908035906020019092919050505061088a565b604051808215151515815260200191505060405180910390f35b3480156101fb57600080fd5b5061020461097c565b6040518082815260200191505060405180910390f35b34801561022657600080fd5b50610285600480360381019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190803573ffffffffffffffffffffffffffffffffffffffff16906020019092919080359060200190929190505050610986565b604051808215151515815260200191505060405180910390f35b3480156102ab57600080fd5b506102b4610cd3565b604051808260ff1660ff16815260200191505060405180910390f35b3480156102dc57600080fd5b5061031b600480360381019080803573ffffffffffffffffffffffffffffffffffffffff16906020019092919080359060200190929190505050610ce6565b005b34801561032957600080fd5b5061035e600480360381019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190505050610ec0565b6040518082815260200191505060405180910390f35b34801561038057600080fd5b50610389610ed8565b6040518080602001828103825283818151815260200191508051906020019080838360005b838110156103c95780820151818401526020810190506103ae565b50505050905090810190601f1680156103f65780820380516001836020036101000a031916815260200191505b509250505060405180910390f35b34801561041057600080fd5b5061044f600480360381019080803573ffffffffffffffffffffffffffffffffffffffff16906020019092919080359060200190929190505050610f76565b604051808215151515815260200191505060405180910390f35b34801561047557600080fd5b5061047e610f8b565b604051808273ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200191505060405180910390f35b3480156104cc57600080fd5b50610521600480360381019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190803573ffffffffffffffffffffffffffffffffffffffff169060200190929190505050610fb1565b6040518082815260200191505060405180910390f35b600081600460003373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020541015151561058757600080fd5b8160036000828254039250508190555081600460003373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060008282540392505081905550600260019054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1660405180807f776974686472617728627974657333322c75696e743235362900000000000000815250601901905060405180910390207c0100000000000000000000000000000000000000000000000000000000900484846040518363ffffffff167c0100000000000000000000000000000000000000000000000000000000028152600401808360001916600019168152602001828152602001925050506000604051808303816000875af1925050509050801515610745576040517f08c379a00000000000000000000000000000000000000000000000000000000081526004018080602001828103825260168152602001807f574b544f4e3a2057495448445241575f4641494c45440000000000000000000081525060200191505060405180910390fd5b600073ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff167fddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef846040518082815260200191505060405180910390a382600019167fa4dfdde26c326c8cced668e6a665f4efc3f278bdc9101cdedc4f725abd63a1ee836040518082815260200191505060405180910390a2505050565b60008054600181600116156101000203166002900480601f0160208091040260200160405190810160405280929190818152602001828054600181600116156101000203166002900480156108825780601f1061085757610100808354040283529160200191610882565b820191906000526020600020905b81548152906001019060200180831161086557829003601f168201915b505050505081565b600081600560003373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060008573ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020819055508273ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff167f8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925846040518082815260200191505060405180910390a36001905092915050565b6000600354905090565b600081600460008673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002054101515156109d657600080fd5b3373ffffffffffffffffffffffffffffffffffffffff168473ffffffffffffffffffffffffffffffffffffffff1614158015610aae57507fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff600560008673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060003373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020016000205414155b15610bc95781600560008673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060003373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020016000205410151515610b3e57600080fd5b81600560008673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060003373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020600082825403925050819055505b81600460008673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020016000206000828254039250508190555081600460008573ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020600082825401925050819055508273ffffffffffffffffffffffffffffffffffffffff168473ffffffffffffffffffffffffffffffffffffffff167fddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef846040518082815260200191505060405180910390a3600190509392505050565b600260009054906101000a900460ff1681565b600260019054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff16141515610dab576040517f08c379a00000000000000000000000000000000000000000000000000000000081526004018080602001828103825260118152602001807f574b544f4e3a205045524d495353494f4e00000000000000000000000000000081525060200191505060405180910390fd5b8060036000828254019250508190555080600460008473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001908152602001600020600082825401925050819055508173ffffffffffffffffffffffffffffffffffffffff16600073ffffffffffffffffffffffffffffffffffffffff167fddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef836040518082815260200191505060405180910390a38173ffffffffffffffffffffffffffffffffffffffff167fe1fffcc4923d04b559f4d29a8bfc6cda04eb5b0d3c460751c2402c5c5cc9109c826040518082815260200191505060405180910390a25050565b60046020528060005260406000206000915090505481565b60018054600181600116156101000203166002900480601f016020809104026020016040519081016040528092919081815260200182805460018160011615610100020316600290048015610f6e5780601f10610f4357610100808354040283529160200191610f6e565b820191906000526020600020905b815481529060010190602001808311610f5157829003601f168201915b505050505081565b6000610f83338484610986565b905092915050565b600260019054906101000a900473ffffffffffffffffffffffffffffffffffffffff1681565b60056020528160005260406000206020528060005260406000206000915091505054815600a165627a7a72305820e2f50a774ba846fa1c029233d81ae94557ebb22046bdc94b10c813c83a2c94660029";
const WITH_DRAW_INPUT: &str = "723908ee9dc8e509d4b93251bd57f68c09bd9d04471c193fabd8f26c54284a4b";
const WKTON_ADDRESS: &str = "32dcab0ef3fb2de2fce1d2e0799d36239671f04a";

fn creation_contract(code: &str, nonce: u64) -> UnsignedTransaction {
	UnsignedTransaction {
		nonce: U256::from(nonce),
		gas_price: U256::from(1),
		gas_limit: U256::from(0x100000),
		action: ethereum::TransactionAction::Create,
		value: U256::zero(),
		input: hex2bytes_unchecked(code),
	}
}

fn default_withdraw_unsigned_transaction() -> UnsignedTransaction {
	UnsignedTransaction {
		nonce: U256::zero(),
		gas_price: U256::from(1),
		gas_limit: U256::from(0x100000),
		action: ethereum::TransactionAction::Call(H160::from_str(TRANSFER_ADDR).unwrap()),
		value: U256::from(30000000000000000000u128),
		input: hex2bytes_unchecked(WITH_DRAW_INPUT),
	}
}

fn deploy_wkton_contract(account: &AccountInfo, code: &str, nonce: u64) {
	let raw_tx = creation_contract(code, nonce);
	let t = sign_transaction(account, raw_tx);
	assert_ok!(Ethereum::execute(
		account.address,
		t.input,
		t.value,
		t.gas_limit,
		Some(t.gas_price),
		Some(t.nonce),
		t.action,
		None,
	));
}

fn send_kton_transfer_and_call_tx(sender: &AccountInfo, address: H160, value: U256, nonce: u64) {
	let raw_tx = UnsignedTransaction {
		nonce: U256::from(nonce),
		gas_price: U256::from(1),
		gas_limit: U256::from(0x300000),
		action: ethereum::TransactionAction::Call(H160::from_str(TRANSFER_ADDR).unwrap()),
		value: U256::from(0),
		input: transfer_and_call(address, value),
	};
	let t = sign_transaction(sender, raw_tx);
	assert_ok!(Ethereum::execute(
		sender.address,
		t.input.clone(),
		t.value,
		t.gas_limit,
		None,
		Some(t.nonce),
		t.action,
		None,
	));
}

fn send_kton_withdraw_tx(sender: &AccountInfo, to_id: Vec<u8>, value: U256, nonce: u64) {
	let data = withdraw(to_id, value);
	let raw_tx = UnsignedTransaction {
		nonce: U256::from(nonce),
		gas_price: U256::from(1),
		gas_limit: U256::from(0x300000),
		action: ethereum::TransactionAction::Call(H160::from_str(WKTON_ADDRESS).unwrap()),
		value: U256::from(0),
		input: data,
	};
	let t = sign_transaction(sender, raw_tx);
	assert_ok!(Ethereum::execute(
		sender.address,
		t.input.clone(),
		t.value,
		t.gas_limit,
		None,
		Some(t.nonce),
		t.action,
		None,
	));
}

fn get_wkton_balance(sender: &AccountInfo, nonce: u64) -> U256 {
	let raw_tx = UnsignedTransaction {
		nonce: U256::from(nonce),
		gas_price: U256::from(1),
		gas_limit: U256::from(0x300000),
		action: ethereum::TransactionAction::Call(H160::from_str(WKTON_ADDRESS).unwrap()),
		value: U256::from(0),
		input: hex2bytes_unchecked(bytes2hex("0x", wkton_balance_input(sender.address))),
	};
	let t = sign_transaction(sender, raw_tx);
	if let Ok((_, _, res)) = Ethereum::execute(
		sender.address,
		t.input.clone(),
		t.value,
		t.gas_limit,
		None,
		Some(t.nonce),
		t.action,
		None,
	) {
		match res {
			CallOrCreateInfo::Call(info) => return U256::from_big_endian(&info.value),
			CallOrCreateInfo::Create(_) => return U256::default(),
		};
	}
	U256::default()
}

fn wkton_balance_input(address: H160) -> Vec<u8> {
	let func = Function {
		name: "balanceOf".to_owned(),
		inputs: vec![Param {
			name: "address".to_owned(),
			kind: ParamType::Address,
		}],
		outputs: vec![],
		constant: false,
	};
	func.encode_input(&[Token::Address(address)]).unwrap()
}

fn transfer_and_call(address: H160, value: U256) -> Vec<u8> {
	let func = Function {
		name: "transfer_and_call".to_owned(),
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
	func.encode_input(&[Token::Address(address), Token::Uint(value)])
		.unwrap()
}

fn withdraw(to: Vec<u8>, value: U256) -> Vec<u8> {
	let func = Function {
		name: "withdraw".to_owned(),
		inputs: vec![
			Param {
				name: "to".to_owned(),
				kind: ParamType::FixedBytes(32),
			},
			Param {
				name: "value".to_owned(),
				kind: ParamType::Uint(256),
			},
		],
		outputs: vec![],
		constant: false,
	};
	func.encode_input(&[Token::FixedBytes(to), Token::Uint(value)])
		.unwrap()
}

fn sign_transaction(account: &AccountInfo, unsign_tx: UnsignedTransaction) -> Transaction {
	unsign_tx.sign(&account.private_key)
}

macro_rules! assert_balance {
	($evm_address:expr, $balance:expr, $left:expr, $right:expr) => {
		let account_id =
			<Test as darwinia_evm::Config>::AddressMapping::into_account_id($evm_address);
		assert_eq!(
			<Test as darwinia_evm::Config>::RingAccountBasic::account_basic(&$evm_address).balance,
			$balance
		);
		assert_eq!(
			<Test as darwinia_evm::Config>::RingCurrency::free_balance(&account_id),
			$left
		);
		assert_eq!(
			<RingRemainBalance as RemainBalanceOp<Test, u64>>::remaining_balance(&account_id),
			$right
		);
	};
}

#[test]
fn transaction_should_increment_nonce() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let t = sign_transaction(alice, creation_contract(ERC20_CONTRACT_BYTECODE, 0));
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
			<Test as darwinia_evm::Config>::RingAccountBasic::account_basic(&alice.address).nonce,
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
			sign_transaction(alice, creation_contract(ERC20_CONTRACT_BYTECODE, 0));
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
		let mut transaction = creation_contract(ERC20_CONTRACT_BYTECODE, 0);
		transaction.nonce = U256::from(1);

		let signed = transaction.sign(&alice.private_key);

		assert_eq!(
			Ethereum::validate_unsigned(TransactionSource::External, &Call::transact(signed)),
			ValidTransactionBuilder::default()
				.and_provides((alice.address, U256::from(1)))
				.priority(1048576 as u64)
				.and_requires((alice.address, U256::from(0)))
				.build()
		);
		let t = sign_transaction(alice, creation_contract(ERC20_CONTRACT_BYTECODE, 0));

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
		let t = sign_transaction(alice, creation_contract(ERC20_CONTRACT_BYTECODE, 0));
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
			Origin::none(),
			sign_transaction(alice, creation_contract(ERC20_CONTRACT_BYTECODE, 0)),
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
fn invalid_signature_should_be_ignored() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	let mut transaction = sign_transaction(alice, creation_contract(ERC20_CONTRACT_BYTECODE, 0));
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
		let t = sign_transaction(alice, creation_contract(ERC20_CONTRACT_BYTECODE, 0));
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
		assert_ne!(EVM::account_codes(erc20_address).len(), 0);
	});
}

#[test]
fn transaction_should_generate_correct_gas_used() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	let expected_gas = U256::from(891328);

	ext.execute_with(|| {
		let t = sign_transaction(alice, creation_contract(ERC20_CONTRACT_BYTECODE, 0));
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
			input: hex2bytes_unchecked(contract),
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
			hex2bytes_unchecked("32dcab0ef3fb2de2fce1d2e0799d36239671f04a");
		let foo: Vec<u8> = hex2bytes_unchecked("c2985578");
		let bar: Vec<u8> = hex2bytes_unchecked("febb0f7e");

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
					bytes2hex("0x", info.value),
					"0x0000000000000000000000000000000000000000000000000000000000000001".to_owned()
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
			<Test as darwinia_evm::Config>::RingAccountBasic::account_basic(&alice.address).balance,
			U256::from(70_000_000_000_000_000_000u128)
		);
		// Check the target balance
		let input_bytes: Vec<u8> = hex2bytes_unchecked(WITH_DRAW_INPUT);
		let dest =
			<Test as frame_system::Config>::AccountId::decode(&mut &input_bytes[..]).unwrap();
		assert_eq!(
			<Test as Config>::RingCurrency::free_balance(dest),
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
				index: 4,
				error: 0,
				message: Some("BalanceLow")
			}
		);

		// Check caller balance
		assert_eq!(
			<Test as darwinia_evm::Config>::RingAccountBasic::account_basic(&alice.address).balance,
			U256::from(100000000000000000000u128)
		);
		// Check target balance
		let input_bytes: Vec<u8> = hex2bytes_unchecked(WITH_DRAW_INPUT);
		let dest =
			<Test as frame_system::Config>::AccountId::decode(&mut &input_bytes[..]).unwrap();
		assert_eq!(<Test as Config>::RingCurrency::free_balance(&dest), 0);
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
			<Test as darwinia_evm::Config>::RingAccountBasic::account_basic(&alice.address).balance,
			U256::from(100000000000000000000u128)
		);
		// Check target balance
		let input_bytes: Vec<u8> = hex2bytes_unchecked(WITH_DRAW_INPUT);
		let dest =
			<Test as frame_system::Config>::AccountId::decode(&mut &input_bytes[..]).unwrap();
		assert_eq!(<Test as Config>::RingCurrency::free_balance(&dest), 0);
	});
}

#[test]
fn test_kton_transfer_and_call_works() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		// Give alice some kton token
		<Test as darwinia_evm::Config>::KtonAccountBasic::mutate_account_basic_balance(
			&alice.address,
			70_000_000_000_000_000_000u128.into(),
		);

		// Create wkton contract
		deploy_wkton_contract(alice, WKTON_CONTRACT_BYTECODE, 0);
		assert_eq!(
			<Test as darwinia_evm::Config>::KtonAccountBasic::account_basic(&alice.address).balance,
			U256::from(70_000_000_000_000_000_000u128)
		);

		// Transfer and call
		send_kton_transfer_and_call_tx(
			alice,
			H160::from_str(WKTON_ADDRESS).unwrap(),
			30_000_000_000_000_000_000u128.into(),
			1,
		);
		assert_eq!(
			<Test as darwinia_evm::Config>::KtonAccountBasic::account_basic(&alice.address).balance,
			U256::from(40_000_000_000_000_000_000u128)
		);
		assert_eq!(
			get_wkton_balance(alice, 2),
			U256::from(30_000_000_000_000_000_000u128)
		);

		// Transfer and call
		send_kton_transfer_and_call_tx(
			alice,
			H160::from_str(WKTON_ADDRESS).unwrap(),
			30_000_000_000_000_000_000u128.into(),
			3,
		);
		assert_eq!(
			<Test as darwinia_evm::Config>::KtonAccountBasic::account_basic(&alice.address).balance,
			U256::from(10_000_000_000_000_000_000u128)
		);
		assert_eq!(
			get_wkton_balance(alice, 4),
			U256::from(60_000_000_000_000_000_000u128)
		);
	});
}

#[test]
fn test_kton_transfer_and_call_out_of_fund() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		// Give alice some kton token
		<Test as darwinia_evm::Config>::KtonAccountBasic::mutate_account_basic_balance(
			&alice.address,
			70_000_000_000_000_000_000u128.into(),
		);

		// Create wkton contract
		deploy_wkton_contract(alice, WKTON_CONTRACT_BYTECODE, 0);
		assert_eq!(
			<Test as darwinia_evm::Config>::KtonAccountBasic::account_basic(&alice.address).balance,
			U256::from(70_000_000_000_000_000_000u128)
		);

		// Transfer and call
		send_kton_transfer_and_call_tx(
			alice,
			H160::from_str(WKTON_ADDRESS).unwrap(),
			90_000_000_000_000_000_000u128.into(),
			1,
		);
		assert_eq!(
			<Test as darwinia_evm::Config>::KtonAccountBasic::account_basic(&alice.address).balance,
			U256::from(70_000_000_000_000_000_000u128)
		);
		assert_eq!(get_wkton_balance(alice, 2), U256::from(0));
	});
}

#[test]
fn test_kton_withdraw() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		// Give alice some kton token
		<Test as darwinia_evm::Config>::KtonAccountBasic::mutate_account_basic_balance(
			&alice.address,
			70_000_000_000_000_000_000u128.into(),
		);
		// Create wkton contract
		deploy_wkton_contract(alice, WKTON_CONTRACT_BYTECODE, 0);
		assert_eq!(
			<Test as darwinia_evm::Config>::KtonAccountBasic::account_basic(&alice.address).balance,
			U256::from(70_000_000_000_000_000_000u128)
		);

		// Transfer and call
		send_kton_transfer_and_call_tx(
			alice,
			H160::from_str(WKTON_ADDRESS).unwrap(),
			30_000_000_000_000_000_000u128.into(),
			1,
		);
		assert_eq!(
			<Test as darwinia_evm::Config>::KtonAccountBasic::account_basic(&alice.address).balance,
			U256::from(40_000_000_000_000_000_000u128)
		);
		assert_eq!(
			get_wkton_balance(alice, 2),
			U256::from(30_000_000_000_000_000_000u128)
		);

		// withdraw
		let input_bytes: Vec<u8> = hex2bytes_unchecked(
			"0x64766d3a00000000000000aa01a1bef0557fa9625581a293f3aa777019263256",
		);
		send_kton_withdraw_tx(
			alice,
			input_bytes.clone(),
			U256::from(10_000_000_000_000_000_000u128),
			3,
		);
		let to_id =
			<Test as frame_system::Config>::AccountId::decode(&mut &input_bytes[..]).unwrap();
		assert_eq!(
			<Test as darwinia_evm::Config>::KtonAccountBasic::account_balance(&to_id),
			U256::from(10_000_000_000_000_000_000u128)
		);
		assert_eq!(
			get_wkton_balance(alice, 4),
			U256::from(20_000_000_000_000_000_000u128)
		);
	});
}

#[test]
fn test_kton_withdraw_out_of_fund() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		// Give alice some kton token
		<Test as darwinia_evm::Config>::KtonAccountBasic::mutate_account_basic_balance(
			&alice.address,
			70_000_000_000_000_000_000u128.into(),
		);
		// Create wkton contract
		deploy_wkton_contract(alice, WKTON_CONTRACT_BYTECODE, 0);
		assert_eq!(
			<Test as darwinia_evm::Config>::KtonAccountBasic::account_basic(&alice.address).balance,
			U256::from(70_000_000_000_000_000_000u128)
		);

		// Transfer and call
		send_kton_transfer_and_call_tx(
			alice,
			H160::from_str(WKTON_ADDRESS).unwrap(),
			30_000_000_000_000_000_000u128.into(),
			1,
		);
		assert_eq!(
			<Test as darwinia_evm::Config>::KtonAccountBasic::account_basic(&alice.address).balance,
			U256::from(40_000_000_000_000_000_000u128)
		);
		assert_eq!(
			get_wkton_balance(alice, 2),
			U256::from(30_000_000_000_000_000_000u128)
		);

		// withdraw
		let input_bytes: Vec<u8> = hex2bytes_unchecked(
			"0x64766d3a00000000000000aa01a1bef0557fa9625581a293f3aa777019263256",
		);
		send_kton_withdraw_tx(
			alice,
			input_bytes.clone(),
			U256::from(70_000_000_000_000_000_000u128),
			3,
		);
		let to_id =
			<Test as frame_system::Config>::AccountId::decode(&mut &input_bytes[..]).unwrap();
		assert_eq!(
			<Test as darwinia_evm::Config>::KtonAccountBasic::account_balance(&to_id),
			U256::from(0)
		);
		assert_eq!(
			get_wkton_balance(alice, 4),
			U256::from(30_000_000_000_000_000_000u128)
		);
	});
}

#[test]
fn mutate_account_works_well() {
	let (_, mut ext) = new_test_ext(1);
	ext.execute_with(|| {
		let test_addr = H160::from_str("1000000000000000000000000000000000000001").unwrap();
		let origin_balance = U256::from(123_456_789_000_000_090u128);
		<Test as darwinia_evm::Config>::RingAccountBasic::mutate_account_basic_balance(
			&test_addr,
			origin_balance,
		);

		assert_balance!(test_addr, origin_balance, 123456789, 90);
	});
}

#[test]
fn mutate_account_inc_balance_by_10() {
	let (_, mut ext) = new_test_ext(1);
	ext.execute_with(|| {
		let test_addr = H160::from_str("1000000000000000000000000000000000000001").unwrap();
		// origin
		let origin_balance = U256::from(600_000_000_090u128);
		<Test as darwinia_evm::Config>::RingAccountBasic::mutate_account_basic_balance(
			&test_addr,
			origin_balance,
		);

		let new_balance = origin_balance.saturating_add(U256::from(10));
		<Test as darwinia_evm::Config>::RingAccountBasic::mutate_account_basic_balance(
			&test_addr,
			new_balance,
		);
		assert_balance!(test_addr, new_balance, 600, 100);
	});
}

#[test]
fn mutate_account_inc_balance_by_999_999_910() {
	let (_, mut ext) = new_test_ext(1);
	ext.execute_with(|| {
		let test_addr = H160::from_str("1000000000000000000000000000000000000001").unwrap();
		// origin
		let origin_balance = U256::from(600_000_000_090u128);
		<Test as darwinia_evm::Config>::RingAccountBasic::mutate_account_basic_balance(
			&test_addr,
			origin_balance,
		);

		let new_balance = origin_balance.saturating_add(U256::from(999999910u128));
		<Test as darwinia_evm::Config>::RingAccountBasic::mutate_account_basic_balance(
			&test_addr,
			new_balance,
		);
		assert_balance!(test_addr, new_balance, 601, 0);
	});
}

#[test]
fn mutate_account_inc_by_1000_000_000() {
	let (_, mut ext) = new_test_ext(1);
	ext.execute_with(|| {
		let test_addr = H160::from_str("1000000000000000000000000000000000000001").unwrap();
		// origin
		let origin_balance = U256::from(600_000_000_090u128);
		<Test as darwinia_evm::Config>::RingAccountBasic::mutate_account_basic_balance(
			&test_addr,
			origin_balance,
		);

		let new_balance = origin_balance.saturating_add(U256::from(1000_000_000u128));
		<Test as darwinia_evm::Config>::RingAccountBasic::mutate_account_basic_balance(
			&test_addr,
			new_balance,
		);
		assert_balance!(test_addr, new_balance, 601, 90);
	});
}

#[test]
fn mutate_account_dec_balance_by_90() {
	let (_, mut ext) = new_test_ext(1);
	ext.execute_with(|| {
		let test_addr = H160::from_str("1000000000000000000000000000000000000001").unwrap();
		// origin
		let origin_balance = U256::from(600_000_000_090u128);
		<Test as darwinia_evm::Config>::RingAccountBasic::mutate_account_basic_balance(
			&test_addr,
			origin_balance,
		);

		let new_balance = origin_balance.saturating_sub(U256::from(90));
		<Test as darwinia_evm::Config>::RingAccountBasic::mutate_account_basic_balance(
			&test_addr,
			new_balance,
		);
		assert_balance!(test_addr, new_balance, 600, 0);
	});
}
#[test]
fn mutate_account_dec_balance_by_990() {
	let (_, mut ext) = new_test_ext(1);
	ext.execute_with(|| {
		let test_addr = H160::from_str("1000000000000000000000000000000000000001").unwrap();
		// origin
		let origin_balance = U256::from(600_000_000_090u128);
		<Test as darwinia_evm::Config>::RingAccountBasic::mutate_account_basic_balance(
			&test_addr,
			origin_balance,
		);

		let new_balance = origin_balance.saturating_sub(U256::from(990));
		<Test as darwinia_evm::Config>::RingAccountBasic::mutate_account_basic_balance(
			&test_addr,
			new_balance,
		);
		assert_balance!(test_addr, new_balance, 599, 1_000_000_090 - 990);
	});
}
#[test]
fn mutate_account_dec_balance_existential_by_90() {
	let (_, mut ext) = new_test_ext(1);
	ext.execute_with(|| {
		let test_addr = H160::from_str("1000000000000000000000000000000000000001").unwrap();
		// origin
		let origin_balance = U256::from(500_000_000_090u128);
		<Test as darwinia_evm::Config>::RingAccountBasic::mutate_account_basic_balance(
			&test_addr,
			origin_balance,
		);

		let new_balance = origin_balance.saturating_sub(U256::from(90));
		<Test as darwinia_evm::Config>::RingAccountBasic::mutate_account_basic_balance(
			&test_addr,
			new_balance,
		);
		assert_balance!(test_addr, new_balance, 500, 0);
	});
}
#[test]
fn mutate_account_dec_balance_existential_by_990() {
	let (_, mut ext) = new_test_ext(1);
	ext.execute_with(|| {
		let test_addr = H160::from_str("1000000000000000000000000000000000000001").unwrap();
		// origin
		let origin_balance = U256::from(500_000_000_090u128);
		<Test as darwinia_evm::Config>::RingAccountBasic::mutate_account_basic_balance(
			&test_addr,
			origin_balance,
		);

		let new_balance = origin_balance.saturating_sub(U256::from(990));
		<Test as darwinia_evm::Config>::RingAccountBasic::mutate_account_basic_balance(
			&test_addr,
			new_balance,
		);

		assert_balance!(test_addr, U256::zero(), 0, 0);
	});
}
