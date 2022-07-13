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
use codec::Encode;
use darwinia_evm_precompile_utils::{data::Bytes, test_helper::LegacyUnsignedTransaction};
use darwinia_support::evm::decimal_convert;
use ethabi::{ParamType, Token};
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
	assert_eq!(Action::Withdraw as u32, 0x40cf020);

	assert_eq!(
		crate::SELECTOR_LOG_TRANSFER,
		&Keccak256::digest(b"Transfer(address,address,uint256)")[..]
	);

	assert_eq!(
		crate::SELECTOR_LOG_APPROVAL,
		&Keccak256::digest(b"Approval(address,address,uint256)")[..]
	);

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
		let nonce = 0;
		let tx = prepare_transaction(
			nonce,
			EvmDataWriter::new_with_selector(Action::TotalSupply).build(),
			&alice.private_key,
		);

		let executed_info =
			Ethereum::execute(alice.address, &tx.into(), None).map(|(_, _, res)| match res {
				CallOrCreateInfo::Call(info) => info,
				CallOrCreateInfo::Create(_) => todo!(),
			});

		assert_eq!(
			executed_info.unwrap().value,
			EvmDataWriter::new().write(decimal_convert(INITIAL_BALANCE, None)).build(),
		);
	});
}

#[test]
fn test_token_name() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let nonce = 0;
		let tx = prepare_transaction(
			nonce,
			EvmDataWriter::new_with_selector(Action::Name).build(),
			&alice.private_key,
		);

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
		let nonce = 0;
		let tx = prepare_transaction(
			nonce,
			EvmDataWriter::new_with_selector(Action::Symbol).build(),
			&alice.private_key,
		);

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
		let nonce = 0;
		let tx = prepare_transaction(
			nonce,
			EvmDataWriter::new_with_selector(Action::Decimals).build(),
			&alice.private_key,
		);

		let executed_info =
			Ethereum::execute(alice.address, &tx.into(), None).map(|(_, _, res)| match res {
				CallOrCreateInfo::Call(info) => info,
				CallOrCreateInfo::Create(_) => todo!(),
			});

		assert_eq!(executed_info.unwrap().value, EvmDataWriter::new().write(TOKEN_DECIMAL).build(),);
	});
}

#[test]
fn test_balance_of_known_user() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let nonce = 0;
		let tx = prepare_transaction(
			nonce,
			EvmDataWriter::new_with_selector(Action::BalanceOf)
				.write::<Address>(alice.address.into())
				.build(),
			&alice.private_key,
		);

		let executed_info =
			Ethereum::execute(alice.address, &tx.into(), None).map(|(_, _, res)| match res {
				CallOrCreateInfo::Call(info) => info,
				CallOrCreateInfo::Create(_) => todo!(),
			});

		assert_eq!(
			executed_info.unwrap().value,
			EvmDataWriter::new().write(decimal_convert(INITIAL_BALANCE, None)).build(),
		);
	});
}

#[test]
fn test_balance_of_unknown_user() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let mock_address = H160::from_low_u64_be(100);

		let nonce = 0;
		let tx = prepare_transaction(
			nonce,
			EvmDataWriter::new_with_selector(Action::BalanceOf)
				.write::<Address>(mock_address.into())
				.build(),
			&alice.private_key,
		);

		let executed_info =
			Ethereum::execute(alice.address, &tx.into(), None).map(|(_, _, res)| match res {
				CallOrCreateInfo::Call(info) => info,
				CallOrCreateInfo::Create(_) => todo!(),
			});

		assert_eq!(
			executed_info.unwrap().value,
			EvmDataWriter::new().write(decimal_convert(0, None)).build(),
		);
	});
}

#[test]
fn test_approve() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let mock_address = H160::from_low_u64_be(100);
		let approve_value = decimal_convert(500, None);

		let nonce = 0;
		let tx = prepare_transaction(
			nonce,
			EvmDataWriter::new_with_selector(Action::Approve)
				.write::<Address>(mock_address.into())
				.write::<U256>(approve_value.into())
				.build(),
			&alice.private_key,
		);

		let executed_info =
			Ethereum::execute(alice.address, &tx.into(), None).map(|(_, _, res)| match res {
				CallOrCreateInfo::Call(info) => info,
				CallOrCreateInfo::Create(_) => todo!(),
			});

		assert_eq!(executed_info.clone().unwrap().value, EvmDataWriter::new().write(true).build());
		assert_eq!(
			executed_info.unwrap().logs,
			vec![log3(
				H160::from_str(PRECOMPILE_ADDR).unwrap(),
				SELECTOR_LOG_APPROVAL,
				alice.address,
				mock_address,
				EvmDataWriter::new().write(approve_value).build()
			)]
		);
	});
}

#[test]
fn test_allowance_exist() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let mock_address = H160::from_low_u64_be(100);
		let approve_value = decimal_convert(500, None);
		let precompile_address = H160::from_str(PRECOMPILE_ADDR).unwrap();

		// Approve
		let mut nonce = 0;
		let tx = prepare_transaction(
			nonce,
			EvmDataWriter::new_with_selector(Action::Approve)
				.write::<Address>(mock_address.into())
				.write::<U256>(approve_value.into())
				.build(),
			&alice.private_key,
		);
		let _ = Ethereum::execute(alice.address, &tx.into(), None).map(|(_, _, res)| match res {
			CallOrCreateInfo::Call(info) => info,
			CallOrCreateInfo::Create(_) => todo!(),
		});

		// Allowance
		nonce += 1;
		let tx = prepare_transaction(
			nonce,
			EvmDataWriter::new_with_selector(Action::Allowance)
				.write::<Address>(alice.address.into())
				.write::<Address>(mock_address.into())
				.build(),
			&alice.private_key,
		);

		let executed_info =
			Ethereum::execute(alice.address, &tx.into(), None).map(|(_, _, res)| match res {
				CallOrCreateInfo::Call(info) => info,
				CallOrCreateInfo::Create(_) => todo!(),
			});
		assert_eq!(
			executed_info.clone().unwrap().value,
			EvmDataWriter::new().write(approve_value).build()
		);
	});
}

#[test]
fn test_allowance_not_exist() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let mock_address = H160::from_low_u64_be(100);

		// Allowance
		let nonce = 0;
		let tx = prepare_transaction(
			nonce,
			EvmDataWriter::new_with_selector(Action::Allowance)
				.write::<Address>(alice.address.into())
				.write::<Address>(mock_address.into())
				.build(),
			&alice.private_key,
		);

		let executed_info =
			Ethereum::execute(alice.address, &tx.into(), None).map(|(_, _, res)| match res {
				CallOrCreateInfo::Call(info) => info,
				CallOrCreateInfo::Create(_) => todo!(),
			});
		assert_eq!(executed_info.clone().unwrap().value, EvmDataWriter::new().write(0u8).build());
	});
}

#[test]
fn test_transfer() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let mock_address = H160::from_low_u64_be(100);
		let transfer_value = decimal_convert(500, None);

		let mut nonce = 0;
		let tx = prepare_transaction(
			nonce,
			EvmDataWriter::new_with_selector(Action::Transfer)
				.write::<Address>(mock_address.into())
				.write::<U256>(transfer_value.into())
				.build(),
			&alice.private_key,
		);

		// Transfer
		let executed_info =
			Ethereum::execute(alice.address, &tx.into(), None).map(|(_, _, res)| match res {
				CallOrCreateInfo::Call(info) => info,
				CallOrCreateInfo::Create(_) => todo!(),
			});
		assert_eq!(executed_info.clone().unwrap().value, EvmDataWriter::new().write(true).build());

		// Check source account balance
		nonce += 1;
		let tx = prepare_transaction(
			nonce,
			EvmDataWriter::new_with_selector(Action::BalanceOf)
				.write::<Address>(alice.address.into())
				.build(),
			&alice.private_key,
		);

		let executed_info =
			Ethereum::execute(alice.address, &tx.into(), None).map(|(_, _, res)| match res {
				CallOrCreateInfo::Call(info) => info,
				CallOrCreateInfo::Create(_) => todo!(),
			});

		assert_eq!(
			executed_info.unwrap().value,
			EvmDataWriter::new().write(decimal_convert(INITIAL_BALANCE - 500, None)).build(),
		);

		// Check target account balance
		nonce += 1;
		let tx = prepare_transaction(
			nonce,
			EvmDataWriter::new_with_selector(Action::BalanceOf)
				.write::<Address>(mock_address.into())
				.build(),
			&alice.private_key,
		);

		let executed_info =
			Ethereum::execute(alice.address, &tx.into(), None).map(|(_, _, res)| match res {
				CallOrCreateInfo::Call(info) => info,
				CallOrCreateInfo::Create(_) => todo!(),
			});

		assert_eq!(
			executed_info.unwrap().value,
			EvmDataWriter::new().write(transfer_value).build(),
		);
	});
}

#[test]
fn test_transfer_not_enough_fund() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let mock_address = H160::from_low_u64_be(100);
		let transfer_value = decimal_convert(INITIAL_BALANCE + 100, None);

		// Transfer
		let mut nonce = 0;
		let tx = prepare_transaction(
			nonce,
			EvmDataWriter::new_with_selector(Action::Transfer)
				.write::<Address>(mock_address.into())
				.write::<U256>(transfer_value.into())
				.build(),
			&alice.private_key,
		);

		let executed_info =
			Ethereum::execute(alice.address, &tx.into(), None).map(|(_, _, res)| match res {
				CallOrCreateInfo::Call(info) => info,
				CallOrCreateInfo::Create(_) => todo!(),
			});
		assert_eq!(
			ethabi::decode(&[ParamType::String], &executed_info.unwrap().value[4..]).unwrap()[0],
			Token::String("Transfer failed".to_string())
		);

		// Check source account balance
		nonce += 1;
		let tx = prepare_transaction(
			nonce,
			EvmDataWriter::new_with_selector(Action::BalanceOf)
				.write::<Address>(alice.address.into())
				.build(),
			&alice.private_key,
		);

		let executed_info =
			Ethereum::execute(alice.address, &tx.into(), None).map(|(_, _, res)| match res {
				CallOrCreateInfo::Call(info) => info,
				CallOrCreateInfo::Create(_) => todo!(),
			});

		assert_eq!(
			executed_info.unwrap().value,
			EvmDataWriter::new().write(decimal_convert(INITIAL_BALANCE, None)).build(),
		);

		// Check target account balance
		nonce += 1;
		let tx = prepare_transaction(
			nonce,
			EvmDataWriter::new_with_selector(Action::BalanceOf)
				.write::<Address>(mock_address.into())
				.build(),
			&alice.private_key,
		);
		let executed_info =
			Ethereum::execute(alice.address, &tx.into(), None).map(|(_, _, res)| match res {
				CallOrCreateInfo::Call(info) => info,
				CallOrCreateInfo::Create(_) => todo!(),
			});

		assert_eq!(
			executed_info.unwrap().value,
			EvmDataWriter::new().write(decimal_convert(0, None)).build(),
		);
	});
}

#[test]
fn test_transfer_from() {
	let (pairs, mut ext) = new_test_ext(2);
	let alice = &pairs[0];
	let bob = &pairs[1];

	ext.execute_with(|| {
		let mock_address = H160::from_low_u64_be(100);
		let approve_value = decimal_convert(500, None);
		let transfer_value = decimal_convert(400, None);
		let precompile_address = H160::from_str(PRECOMPILE_ADDR).unwrap();

		// Approve
		let mut alice_nonce = 0;
		let mut bob_nonce = 0;
		let tx = prepare_transaction(
			alice_nonce,
			EvmDataWriter::new_with_selector(Action::Approve)
				.write::<Address>(bob.address.into())
				.write::<U256>(approve_value.into())
				.build(),
			&alice.private_key,
		);
		let executed_info =
			Ethereum::execute(alice.address, &tx.into(), None).map(|(_, _, res)| match res {
				CallOrCreateInfo::Call(info) => info,
				CallOrCreateInfo::Create(_) => todo!(),
			});
		assert_eq!(executed_info.clone().unwrap().value, EvmDataWriter::new().write(true).build());

		// Transfer from
		let tx = prepare_transaction(
			bob_nonce,
			EvmDataWriter::new_with_selector(Action::TransferFrom)
				.write::<Address>(alice.address.into())
				.write::<Address>(mock_address.into())
				.write::<U256>(transfer_value.into())
				.build(),
			&bob.private_key,
		);
		let executed_info =
			Ethereum::execute(bob.address, &tx.into(), None).map(|(_, _, res)| match res {
				CallOrCreateInfo::Call(info) => info,
				CallOrCreateInfo::Create(_) => todo!(),
			});
		assert_eq!(executed_info.unwrap().value, EvmDataWriter::new().write(true).build());

		// Check source account balance
		alice_nonce += 1;
		let tx = prepare_transaction(
			alice_nonce,
			EvmDataWriter::new_with_selector(Action::BalanceOf)
				.write::<Address>(alice.address.into())
				.build(),
			&alice.private_key,
		);
		let executed_info =
			Ethereum::execute(alice.address, &tx.into(), None).map(|(_, _, res)| match res {
				CallOrCreateInfo::Call(info) => info,
				CallOrCreateInfo::Create(_) => todo!(),
			});
		assert_eq!(
			executed_info.unwrap().value,
			EvmDataWriter::new().write(decimal_convert(INITIAL_BALANCE - 400, None)).build(),
		);

		// Check target account balance
		alice_nonce += 1;
		let tx = prepare_transaction(
			alice_nonce,
			EvmDataWriter::new_with_selector(Action::BalanceOf)
				.write::<Address>(mock_address.into())
				.build(),
			&alice.private_key,
		);
		let executed_info =
			Ethereum::execute(alice.address, &tx.into(), None).map(|(_, _, res)| match res {
				CallOrCreateInfo::Call(info) => info,
				CallOrCreateInfo::Create(_) => todo!(),
			});

		assert_eq!(
			executed_info.unwrap().value,
			EvmDataWriter::new().write(decimal_convert(400, None)).build(),
		);

		// Check Allowance
		alice_nonce += 1;
		let tx = prepare_transaction(
			alice_nonce,
			EvmDataWriter::new_with_selector(Action::Allowance)
				.write::<Address>(alice.address.into())
				.write::<Address>(bob.address.into())
				.build(),
			&alice.private_key,
		);
		let executed_info =
			Ethereum::execute(alice.address, &tx.into(), None).map(|(_, _, res)| match res {
				CallOrCreateInfo::Call(info) => info,
				CallOrCreateInfo::Create(_) => todo!(),
			});
		assert_eq!(
			executed_info.clone().unwrap().value,
			EvmDataWriter::new().write(decimal_convert(100, None)).build()
		);
	});
}

#[test]
fn test_transfer_from_above_allowance() {
	let (pairs, mut ext) = new_test_ext(2);
	let alice = &pairs[0];
	let bob = &pairs[1];

	ext.execute_with(|| {
		let mock_address = H160::from_low_u64_be(100);
		let approve_value = decimal_convert(500, None);
		let transfer_value = decimal_convert(700, None);

		let mut alice_nonce = 0;
		let mut bob_nonce = 0;
		// Approve
		let tx = prepare_transaction(
			alice_nonce,
			EvmDataWriter::new_with_selector(Action::Approve)
				.write::<Address>(bob.address.into())
				.write::<U256>(approve_value.into())
				.build(),
			&alice.private_key,
		);

		let executed_info =
			Ethereum::execute(alice.address, &tx.into(), None).map(|(_, _, res)| match res {
				CallOrCreateInfo::Call(info) => info,
				CallOrCreateInfo::Create(_) => todo!(),
			});
		assert_eq!(executed_info.clone().unwrap().value, EvmDataWriter::new().write(true).build());

		// Transfer from
		let tx = prepare_transaction(
			bob_nonce,
			EvmDataWriter::new_with_selector(Action::TransferFrom)
				.write::<Address>(alice.address.into())
				.write::<Address>(mock_address.into())
				.write::<U256>(transfer_value.into())
				.build(),
			&bob.private_key,
		);
		let executed_info =
			Ethereum::execute(bob.address, &tx.into(), None).map(|(_, _, res)| match res {
				CallOrCreateInfo::Call(info) => info,
				CallOrCreateInfo::Create(_) => todo!(),
			});
		assert_eq!(
			ethabi::decode(&[ParamType::String], &executed_info.unwrap().value[4..]).unwrap()[0],
			Token::String("trying to spend more than allowed".to_string())
		);

		// Check source account balance
		alice_nonce += 1;
		let tx = prepare_transaction(
			alice_nonce,
			EvmDataWriter::new_with_selector(Action::BalanceOf)
				.write::<Address>(alice.address.into())
				.build(),
			&alice.private_key,
		);
		let executed_info =
			Ethereum::execute(alice.address, &tx.into(), None).map(|(_, _, res)| match res {
				CallOrCreateInfo::Call(info) => info,
				CallOrCreateInfo::Create(_) => todo!(),
			});
		assert_eq!(
			executed_info.unwrap().value,
			EvmDataWriter::new().write(decimal_convert(INITIAL_BALANCE, None)).build(),
		);

		// Check target account balance
		alice_nonce += 1;
		let tx = prepare_transaction(
			alice_nonce,
			EvmDataWriter::new_with_selector(Action::BalanceOf)
				.write::<Address>(mock_address.into())
				.build(),
			&alice.private_key,
		);

		let executed_info =
			Ethereum::execute(alice.address, &tx.into(), None).map(|(_, _, res)| match res {
				CallOrCreateInfo::Call(info) => info,
				CallOrCreateInfo::Create(_) => todo!(),
			});

		assert_eq!(
			executed_info.unwrap().value,
			EvmDataWriter::new().write(decimal_convert(0, None)).build(),
		);

		// Check Allowance
		alice_nonce += 1;
		let tx = prepare_transaction(
			alice_nonce,
			EvmDataWriter::new_with_selector(Action::Allowance)
				.write::<Address>(alice.address.into())
				.write::<Address>(bob.address.into())
				.build(),
			&alice.private_key,
		);
		let executed_info =
			Ethereum::execute(alice.address, &tx.into(), None).map(|(_, _, res)| match res {
				CallOrCreateInfo::Call(info) => info,
				CallOrCreateInfo::Create(_) => todo!(),
			});
		assert_eq!(
			executed_info.clone().unwrap().value,
			EvmDataWriter::new().write(approve_value).build()
		);
	});
}

#[test]
fn test_transfer_from_self() {
	let (pairs, mut ext) = new_test_ext(2);
	let alice = &pairs[0];
	let bob = &pairs[1];

	ext.execute_with(|| {
		let mock_address = H160::from_low_u64_be(100);
		let approve_value = decimal_convert(500, None);
		let transfer_value = decimal_convert(400, None);
		let precompile_address = H160::from_str(PRECOMPILE_ADDR).unwrap();

		// Transfer from
		let mut nonce = 0;
		let tx = prepare_transaction(
			nonce,
			EvmDataWriter::new_with_selector(Action::TransferFrom)
				.write::<Address>(alice.address.into())
				.write::<Address>(mock_address.into())
				.write::<U256>(transfer_value.into())
				.build(),
			&alice.private_key,
		);

		let executed_info =
			Ethereum::execute(alice.address, &tx.into(), None).map(|(_, _, res)| match res {
				CallOrCreateInfo::Call(info) => info,
				CallOrCreateInfo::Create(_) => todo!(),
			});
		assert_eq!(executed_info.unwrap().value, EvmDataWriter::new().write(true).build());

		// Check source account balance
		nonce += 1;
		let tx = prepare_transaction(
			nonce,
			EvmDataWriter::new_with_selector(Action::BalanceOf)
				.write::<Address>(alice.address.into())
				.build(),
			&alice.private_key,
		);
		let executed_info =
			Ethereum::execute(alice.address, &tx.into(), None).map(|(_, _, res)| match res {
				CallOrCreateInfo::Call(info) => info,
				CallOrCreateInfo::Create(_) => todo!(),
			});
		assert_eq!(
			executed_info.unwrap().value,
			EvmDataWriter::new().write(decimal_convert(INITIAL_BALANCE - 400, None)).build(),
		);

		// Check target account balance
		nonce += 1;
		let tx = prepare_transaction(
			nonce,
			EvmDataWriter::new_with_selector(Action::BalanceOf)
				.write::<Address>(mock_address.into())
				.build(),
			&alice.private_key,
		);
		let executed_info =
			Ethereum::execute(alice.address, &tx.into(), None).map(|(_, _, res)| match res {
				CallOrCreateInfo::Call(info) => info,
				CallOrCreateInfo::Create(_) => todo!(),
			});

		assert_eq!(
			executed_info.unwrap().value,
			EvmDataWriter::new().write(decimal_convert(400, None)).build(),
		);
	});
}

#[test]
fn test_withdraw() {
	let (pairs, mut ext) = new_test_ext(2);
	let alice = &pairs[0];
	let bob = &pairs[1];

	ext.execute_with(|| {
		let mock_address = H160::from_low_u64_be(100);
		let mock_account_id =
			<Test as darwinia_evm::Config>::IntoAccountId::derive_substrate_address(mock_address);
		let withdraw_value = decimal_convert(500, None);
		let precompile_address = H160::from_str(PRECOMPILE_ADDR).unwrap();

		// Withdraw
		let mut nonce = 0;
		let tx = prepare_transaction(
			nonce,
			EvmDataWriter::new_with_selector(Action::Withdraw)
				.write::<Bytes>(Bytes(mock_account_id.encode()))
				.write::<U256>(withdraw_value.into())
				.build(),
			&alice.private_key,
		);
		let executed_info =
			Ethereum::execute(alice.address, &tx.into(), None).map(|(_, _, res)| match res {
				CallOrCreateInfo::Call(info) => info,
				CallOrCreateInfo::Create(_) => todo!(),
			});
		assert_eq!(executed_info.unwrap().value, EvmDataWriter::new().write(true).build());

		// Check source account balance
		nonce += 1;
		let tx = prepare_transaction(
			nonce,
			EvmDataWriter::new_with_selector(Action::BalanceOf)
				.write::<Address>(alice.address.into())
				.build(),
			&alice.private_key,
		);

		let executed_info =
			Ethereum::execute(alice.address, &tx.into(), None).map(|(_, _, res)| match res {
				CallOrCreateInfo::Call(info) => info,
				CallOrCreateInfo::Create(_) => todo!(),
			});
		assert_eq!(
			executed_info.unwrap().value,
			EvmDataWriter::new().write(decimal_convert(INITIAL_BALANCE - 500, None)).build(),
		);

		// Check target account balance
		nonce += 1;
		let tx = prepare_transaction(
			nonce,
			EvmDataWriter::new_with_selector(Action::BalanceOf)
				.write::<Address>(mock_address.into())
				.build(),
			&alice.private_key,
		);
		let executed_info =
			Ethereum::execute(alice.address, &tx.into(), None).map(|(_, _, res)| match res {
				CallOrCreateInfo::Call(info) => info,
				CallOrCreateInfo::Create(_) => todo!(),
			});

		assert_eq!(
			executed_info.unwrap().value,
			EvmDataWriter::new().write(decimal_convert(500, None)).build(),
		);
	});
}

#[test]
fn test_withdraw_not_enough() {
	let (pairs, mut ext) = new_test_ext(2);
	let alice = &pairs[0];
	let bob = &pairs[1];

	ext.execute_with(|| {
		let mock_address = H160::from_low_u64_be(100);
		let mock_account_id =
			<Test as darwinia_evm::Config>::IntoAccountId::derive_substrate_address(mock_address);
		let withdraw_value = decimal_convert(INITIAL_BALANCE + 500, None);
		let precompile_address = H160::from_str(PRECOMPILE_ADDR).unwrap();

		// Withdraw
		let mut nonce = 0;
		let tx = prepare_transaction(
			nonce,
			EvmDataWriter::new_with_selector(Action::Withdraw)
				.write::<Bytes>(Bytes(mock_account_id.encode()))
				.write::<U256>(withdraw_value.into())
				.build(),
			&alice.private_key,
		);

		let executed_info =
			Ethereum::execute(alice.address, &tx.into(), None).map(|(_, _, res)| match res {
				CallOrCreateInfo::Call(info) => info,
				CallOrCreateInfo::Create(_) => todo!(),
			});
		assert_eq!(
			ethabi::decode(&[ParamType::String], &executed_info.unwrap().value[4..]).unwrap()[0],
			Token::String("Transfer failed".to_string())
		);

		// Check source account balance
		nonce += 1;
		let tx = prepare_transaction(
			nonce,
			EvmDataWriter::new_with_selector(Action::BalanceOf)
				.write::<Address>(alice.address.into())
				.build(),
			&alice.private_key,
		);
		let executed_info =
			Ethereum::execute(alice.address, &tx.into(), None).map(|(_, _, res)| match res {
				CallOrCreateInfo::Call(info) => info,
				CallOrCreateInfo::Create(_) => todo!(),
			});
		assert_eq!(
			executed_info.unwrap().value,
			EvmDataWriter::new().write(decimal_convert(INITIAL_BALANCE, None)).build(),
		);

		// Check target account balance
		nonce += 1;
		let tx = prepare_transaction(
			nonce,
			EvmDataWriter::new_with_selector(Action::BalanceOf)
				.write::<Address>(mock_address.into())
				.build(),
			&alice.private_key,
		);
		let executed_info =
			Ethereum::execute(alice.address, &tx.into(), None).map(|(_, _, res)| match res {
				CallOrCreateInfo::Call(info) => info,
				CallOrCreateInfo::Create(_) => todo!(),
			});

		assert_eq!(
			executed_info.unwrap().value,
			EvmDataWriter::new().write(decimal_convert(0, None)).build(),
		);
	});
}
