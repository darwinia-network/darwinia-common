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
use rustc_hex::{FromHex, ToHex};
use std::str::FromStr;
// --- paritytech ---
use frame_support::{assert_err, assert_noop, assert_ok, unsigned::TransactionValidityError};
use sp_runtime::{
	traits::Applyable,
	transaction_validity::{InvalidTransaction, ValidTransactionBuilder},
};
// --- darwinia-network ---
use crate::{
	mock::*, CallOrCreateInfo, Error, RawOrigin, Transaction, TransactionAction, H160, H256, U256,
};

mod account_basic;
mod eip1559;
mod eip2930;
mod internal;
mod legacy;
mod transfer;

// This ERC-20 contract mints the maximum amount of tokens to the contract creator.
// pragma solidity ^0.5.0;`
// import "https://github.com/OpenZeppelin/openzeppelin-contracts/blob/v2.5.1/contracts/token/ERC20/ERC20.sol";
// contract MyToken is ERC20 {
//	 constructor() public { _mint(msg.sender, 2**256 - 1); }
// }
pub const ERC20_CONTRACT_BYTECODE: &str = include_str!("./res/erc20_contract_bytecode.txt");
// pragma solidity ^0.6.6;
// contract Test {
// 	uint256 public number;

// 	function add(uint256 _value) public {
// 		number = number + _value;
// 	}

// 	function foo() external pure returns (bool) {
// 		return true;
// 	}

// 	function bar() external pure {
// 		require(false, "error_msg");
// 	}
// }
const TEST_CONTRACT_BYTECODE: &str = "608060405234801561001057600080fd5b50610190806100206000396000f3fe608060405234801561001057600080fd5b506004361061004c5760003560e01c80631003e2d2146100515780638381f58a1461007f578063c29855781461009d578063febb0f7e146100bd575b600080fd5b61007d6004803603602081101561006757600080fd5b81019080803590602001909291905050506100c7565b005b6100876100d5565b6040518082815260200191505060405180910390f35b6100a56100db565b60405180821515815260200191505060405180910390f35b6100c56100e4565b005b806000540160008190555050565b60005481565b60006001905090565b6000610158576040517f08c379a00000000000000000000000000000000000000000000000000000000081526004018080602001828103825260098152602001807f6572726f725f6d7367000000000000000000000000000000000000000000000081525060200191505060405180910390fd5b56fea26469706673582212200b5e8ce3d7eb2718a9918bc212cc7cbb53c28cacf08c834278d58f008b336c3064736f6c634300060c0033";

pub type RingAccount = <Test as darwinia_evm::Config>::RingAccountBasic;
pub type KtonAccount = <Test as darwinia_evm::Config>::KtonAccountBasic;
