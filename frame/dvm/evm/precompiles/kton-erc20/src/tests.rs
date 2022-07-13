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

use crate::{mock::*, *};
use darwinia_evm_precompile_utils::test_helper::LegacyUnsignedTransaction;
use darwinia_support::evm::decimal_convert;
use fp_evm::CallOrCreateInfo;
use sha3::{Digest, Keccak256};
use std::str::FromStr;

#[test]
fn selector() {
	assert_eq!(Action::BalanceOf as u32, 0x70a08231);
	assert_eq!(Action::TotalSupply as u32, 0x18160ddd);
	assert_eq!(Action::Approve as u32, 0x095ea7b3);
	assert_eq!(Action::Allowance as u32, 0xdd62ed3e);
	assert_eq!(Action::Transfer as u32, 0xa9059cbb);
	assert_eq!(Action::TransferFrom as u32, 0x23b872dd);
	assert_eq!(Action::Name as u32, 0x06fdde03);
	assert_eq!(Action::Symbol as u32, 0x95d89b41);
	assert_eq!(Action::Deposit as u32, 0xd0e30db0);
	assert_eq!(Action::Withdraw as u32, 0x40cf020);

	assert_eq!(
		crate::SELECTOR_LOG_TRANSFER,
		&Keccak256::digest(b"Transfer(address,address,uint256)")[..]
	);

	assert_eq!(
		crate::SELECTOR_LOG_APPROVAL,
		&Keccak256::digest(b"Approval(address,address,uint256)")[..]
	);

	assert_eq!(crate::SELECTOR_LOG_DEPOSIT, &Keccak256::digest(b"Deposit(address,uint256)")[..]);

	assert_eq!(
		crate::SELECTOR_LOG_WITHDRAWAL,
		&Keccak256::digest(b"Withdrawal(address,uint256)")[..]
	);
}

#[test]
fn test_total_supply() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let unsign_tx = LegacyUnsignedTransaction::new(
			0,
			1,
			1000000,
			ethereum::TransactionAction::Call(
				H160::from_str("0x000000000000000000000000000000000000000a").unwrap(),
			),
			0,
			EvmDataWriter::new_with_selector(Action::TotalSupply).build(),
		);
		let tx = unsign_tx.sign_with_chain_id(&alice.private_key, 42);
		let executed_info =
			Ethereum::execute(alice.address, &tx.into(), None).map(|(_, _, res)| match res {
				CallOrCreateInfo::Call(info) => info,
				CallOrCreateInfo::Create(_) => todo!(),
			});

		assert_eq!(
			executed_info.unwrap().value,
			EvmDataWriter::new().write(decimal_convert(100000000000, None)).build(),
		);
	});
}

#[test]
fn test_token_name() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let unsign_tx = LegacyUnsignedTransaction::new(
			0,
			1,
			1000000,
			ethereum::TransactionAction::Call(
				H160::from_str("0x000000000000000000000000000000000000000a").unwrap(),
			),
			0,
			EvmDataWriter::new_with_selector(Action::Name).build(),
		);
		let tx = unsign_tx.sign_with_chain_id(&alice.private_key, 42);
		let executed_info =
			Ethereum::execute(alice.address, &tx.into(), None).map(|(_, _, res)| match res {
				CallOrCreateInfo::Call(info) => info,
				CallOrCreateInfo::Create(_) => todo!(),
			});

		assert_eq!(
			executed_info.unwrap().value,
			EvmDataWriter::new().write::<Bytes>(TOKEN_NAME.into()).build(),
		);
	});
}

#[test]
fn test_token_symbol() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let unsign_tx = LegacyUnsignedTransaction::new(
			0,
			1,
			1000000,
			ethereum::TransactionAction::Call(
				H160::from_str("0x000000000000000000000000000000000000000a").unwrap(),
			),
			0,
			EvmDataWriter::new_with_selector(Action::Symbol).build(),
		);
		let tx = unsign_tx.sign_with_chain_id(&alice.private_key, 42);
		let executed_info =
			Ethereum::execute(alice.address, &tx.into(), None).map(|(_, _, res)| match res {
				CallOrCreateInfo::Call(info) => info,
				CallOrCreateInfo::Create(_) => todo!(),
			});

		assert_eq!(
			executed_info.unwrap().value,
			EvmDataWriter::new().write::<Bytes>(TOKEN_SYMBOL.into()).build(),
		);
	});
}

#[test]
fn test_token_decimals() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let unsign_tx = LegacyUnsignedTransaction::new(
			0,
			1,
			1000000,
			ethereum::TransactionAction::Call(
				H160::from_str("0x000000000000000000000000000000000000000a").unwrap(),
			),
			0,
			EvmDataWriter::new_with_selector(Action::Decimals).build(),
		);
		let tx = unsign_tx.sign_with_chain_id(&alice.private_key, 42);
		let executed_info =
			Ethereum::execute(alice.address, &tx.into(), None).map(|(_, _, res)| match res {
				CallOrCreateInfo::Call(info) => info,
				CallOrCreateInfo::Create(_) => todo!(),
			});

		assert_eq!(executed_info.unwrap().value, EvmDataWriter::new().write(TOKEN_DECIMAL).build(),);
	});
}
