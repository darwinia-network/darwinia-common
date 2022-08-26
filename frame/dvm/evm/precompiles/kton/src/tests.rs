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
use codec::Encode;
use sha3::{Digest, Keccak256};
use std::str::FromStr;
// --- paritytech ---
use frame_support::{Blake2_128Concat, StorageHasher, Twox128};
// --- darwinia-network ---
use crate::{mock::*, *};
use darwinia_support::evm::decimal_convert;

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

	assert_eq!(
		crate::SELECTOR_LOG_TRANSFER,
		&Keccak256::digest(b"Transfer(address,address,uint256)")[..]
	);

	assert_eq!(
		crate::SELECTOR_LOG_APPROVAL,
		&Keccak256::digest(b"Approval(address,address,uint256)")[..]
	);
}

#[test]
fn test_total_supply() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let nonce = 0;
		construct_tx_asserter(
			nonce,
			EvmDataWriter::new_with_selector(Action::TotalSupply).build(),
			alice,
		)
		.execute()
		.assert_executed_value(
			&EvmDataWriter::new().write(decimal_convert(INITIAL_BALANCE, None)).build(),
		);
	});
}

#[test]
fn test_token_name() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let nonce = 0;
		construct_tx_asserter(nonce, EvmDataWriter::new_with_selector(Action::Name).build(), alice)
			.execute()
			.assert_executed_value(&EvmDataWriter::new().write::<Bytes>(TOKEN_NAME.into()).build());
	});
}

#[test]
fn test_token_symbol() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let nonce = 0;
		construct_tx_asserter(
			nonce,
			EvmDataWriter::new_with_selector(Action::Symbol).build(),
			alice,
		)
		.execute()
		.assert_executed_value(&EvmDataWriter::new().write::<Bytes>(TOKEN_SYMBOL.into()).build());
	});
}

#[test]
fn test_token_decimals() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let nonce = 0;
		construct_tx_asserter(
			nonce,
			EvmDataWriter::new_with_selector(Action::Decimals).build(),
			alice,
		)
		.execute()
		.assert_executed_value(&EvmDataWriter::new().write(TOKEN_DECIMAL).build());
	});
}

#[test]
fn test_balance_of_known_user() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let nonce = 0;
		construct_tx_asserter(
			nonce,
			EvmDataWriter::new_with_selector(Action::BalanceOf)
				.write::<Address>(alice.address.into())
				.build(),
			alice,
		)
		.execute()
		.assert_executed_value(
			&EvmDataWriter::new().write(decimal_convert(INITIAL_BALANCE, None)).build(),
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
		construct_tx_asserter(
			nonce,
			EvmDataWriter::new_with_selector(Action::BalanceOf)
				.write::<Address>(mock_address.into())
				.build(),
			alice,
		)
		.execute()
		.assert_executed_value(&EvmDataWriter::new().write(decimal_convert(0, None)).build());
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
		construct_tx_asserter(
			nonce,
			EvmDataWriter::new_with_selector(Action::Approve)
				.write::<Address>(mock_address.into())
				.write::<U256>(approve_value)
				.build(),
			alice,
		)
		.execute()
		.assert_executed_value(&EvmDataWriter::new().write(true).build())
		.assert_has_log(&log3(
			H160::from_str(PRECOMPILE_ADDR).unwrap(),
			SELECTOR_LOG_APPROVAL,
			alice.address,
			mock_address,
			EvmDataWriter::new().write(approve_value).build(),
		));
	});
}

#[test]
fn test_approve_storage() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let mock_address = H160::from_low_u64_be(100);
		let approve_value = decimal_convert(500, None);

		let nonce = 0;
		construct_tx_asserter(
			nonce,
			EvmDataWriter::new_with_selector(Action::Approve)
				.write::<Address>(mock_address.into())
				.write::<U256>(approve_value)
				.build(),
			alice,
		)
		.execute()
		.assert_executed_value(&EvmDataWriter::new().write(true).build());

		let mut key = Vec::new();
		key.extend_from_slice(&Twox128::hash(b"KtonERC20"));
		key.extend_from_slice(&Twox128::hash(b"Approves"));
		key.extend_from_slice(&Blake2_128Concat::hash(&Encode::encode(&alice.address)));
		key.extend_from_slice(&Blake2_128Concat::hash(&Encode::encode(&mock_address)));

		let storage = frame_support::storage::unhashed::get_raw(&key).unwrap();
		assert_eq!(approve_value, U256::from_little_endian(&storage));
	});
}

#[test]
fn test_allowance_exist() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let mock_address = H160::from_low_u64_be(100);
		let approve_value = decimal_convert(500, None);

		// Approve
		let mut nonce = 0;
		construct_tx_asserter(
			nonce,
			EvmDataWriter::new_with_selector(Action::Approve)
				.write::<Address>(mock_address.into())
				.write::<U256>(approve_value)
				.build(),
			alice,
		)
		.execute()
		.assert_executed_value(&EvmDataWriter::new().write(true).build());

		// Allowance
		nonce += 1;
		construct_tx_asserter(
			nonce,
			EvmDataWriter::new_with_selector(Action::Allowance)
				.write::<Address>(alice.address.into())
				.write::<Address>(mock_address.into())
				.build(),
			alice,
		)
		.execute()
		.assert_executed_value(&EvmDataWriter::new().write(approve_value).build());
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
		construct_tx_asserter(
			nonce,
			EvmDataWriter::new_with_selector(Action::Allowance)
				.write::<Address>(alice.address.into())
				.write::<Address>(mock_address.into())
				.build(),
			alice,
		)
		.execute()
		.assert_executed_value(&EvmDataWriter::new().write(0u8).build());
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
		// Transfer
		construct_tx_asserter(
			nonce,
			EvmDataWriter::new_with_selector(Action::Transfer)
				.write::<Address>(mock_address.into())
				.write::<U256>(transfer_value)
				.build(),
			alice,
		)
		.execute()
		.assert_executed_value(&EvmDataWriter::new().write(true).build());

		// Check source account balance
		nonce += 1;
		construct_tx_asserter(
			nonce,
			EvmDataWriter::new_with_selector(Action::BalanceOf)
				.write::<Address>(alice.address.into())
				.build(),
			alice,
		)
		.execute()
		.assert_executed_value(
			&EvmDataWriter::new().write(decimal_convert(INITIAL_BALANCE - 500, None)).build(),
		);

		// Check target account balance
		nonce += 1;
		construct_tx_asserter(
			nonce,
			EvmDataWriter::new_with_selector(Action::BalanceOf)
				.write::<Address>(mock_address.into())
				.build(),
			alice,
		)
		.execute()
		.assert_executed_value(&EvmDataWriter::new().write(transfer_value).build());
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
		construct_tx_asserter(
			nonce,
			EvmDataWriter::new_with_selector(Action::Transfer)
				.write::<Address>(mock_address.into())
				.write::<U256>(transfer_value)
				.build(),
			alice,
		)
		.execute()
		.assert_revert(
			&EvmDataWriter::new().write(Bytes("Transfer failed".as_bytes().to_vec())).build(),
		);

		// Check source account balance
		nonce += 1;
		construct_tx_asserter(
			nonce,
			EvmDataWriter::new_with_selector(Action::BalanceOf)
				.write::<Address>(alice.address.into())
				.build(),
			alice,
		)
		.execute()
		.assert_executed_value(
			&EvmDataWriter::new().write(decimal_convert(INITIAL_BALANCE, None)).build(),
		);

		// Check target account balance
		nonce += 1;
		construct_tx_asserter(
			nonce,
			EvmDataWriter::new_with_selector(Action::BalanceOf)
				.write::<Address>(mock_address.into())
				.build(),
			alice,
		)
		.execute()
		.assert_executed_value(&EvmDataWriter::new().write(decimal_convert(0, None)).build());
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

		// Approve
		let mut alice_nonce = 0;
		let bob_nonce = 0;
		construct_tx_asserter(
			alice_nonce,
			EvmDataWriter::new_with_selector(Action::Approve)
				.write::<Address>(bob.address.into())
				.write::<U256>(approve_value)
				.build(),
			alice,
		)
		.execute()
		.assert_executed_value(&EvmDataWriter::new().write(true).build());

		// Transfer from
		construct_tx_asserter(
			bob_nonce,
			EvmDataWriter::new_with_selector(Action::TransferFrom)
				.write::<Address>(alice.address.into())
				.write::<Address>(mock_address.into())
				.write::<U256>(transfer_value)
				.build(),
			bob,
		)
		.execute()
		.assert_executed_value(&EvmDataWriter::new().write(true).build());

		// Check source account balance
		alice_nonce += 1;
		construct_tx_asserter(
			alice_nonce,
			EvmDataWriter::new_with_selector(Action::BalanceOf)
				.write::<Address>(alice.address.into())
				.build(),
			alice,
		)
		.execute()
		.assert_executed_value(
			&EvmDataWriter::new().write(decimal_convert(INITIAL_BALANCE - 400, None)).build(),
		);

		// Check target account balance
		alice_nonce += 1;
		construct_tx_asserter(
			alice_nonce,
			EvmDataWriter::new_with_selector(Action::BalanceOf)
				.write::<Address>(mock_address.into())
				.build(),
			alice,
		)
		.execute()
		.assert_executed_value(&EvmDataWriter::new().write(decimal_convert(400, None)).build());

		// Check Allowance
		alice_nonce += 1;
		construct_tx_asserter(
			alice_nonce,
			EvmDataWriter::new_with_selector(Action::Allowance)
				.write::<Address>(alice.address.into())
				.write::<Address>(bob.address.into())
				.build(),
			alice,
		)
		.execute()
		.assert_executed_value(&EvmDataWriter::new().write(decimal_convert(100, None)).build());
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
		let bob_nonce = 0;
		// Approve
		construct_tx_asserter(
			alice_nonce,
			EvmDataWriter::new_with_selector(Action::Approve)
				.write::<Address>(bob.address.into())
				.write::<U256>(approve_value)
				.build(),
			alice,
		)
		.execute()
		.assert_executed_value(&EvmDataWriter::new().write(true).build());

		// Transfer from
		construct_tx_asserter(
			bob_nonce,
			EvmDataWriter::new_with_selector(Action::TransferFrom)
				.write::<Address>(alice.address.into())
				.write::<Address>(mock_address.into())
				.write::<U256>(transfer_value)
				.build(),
			bob,
		)
		.execute()
		.assert_revert(
			&EvmDataWriter::new()
				.write(Bytes("trying to spend more than allowed".as_bytes().to_vec()))
				.build(),
		);

		// Check source account balance
		alice_nonce += 1;
		construct_tx_asserter(
			alice_nonce,
			EvmDataWriter::new_with_selector(Action::BalanceOf)
				.write::<Address>(alice.address.into())
				.build(),
			alice,
		)
		.execute()
		.assert_executed_value(
			&EvmDataWriter::new().write(decimal_convert(INITIAL_BALANCE, None)).build(),
		);

		// Check target account balance
		alice_nonce += 1;
		construct_tx_asserter(
			alice_nonce,
			EvmDataWriter::new_with_selector(Action::BalanceOf)
				.write::<Address>(mock_address.into())
				.build(),
			alice,
		)
		.execute()
		.assert_executed_value(&EvmDataWriter::new().write(decimal_convert(0, None)).build());

		// Check Allowance
		alice_nonce += 1;
		construct_tx_asserter(
			alice_nonce,
			EvmDataWriter::new_with_selector(Action::Allowance)
				.write::<Address>(alice.address.into())
				.write::<Address>(bob.address.into())
				.build(),
			alice,
		)
		.execute()
		.assert_executed_value(&EvmDataWriter::new().write(approve_value).build());
	});
}

#[test]
fn test_transfer_from_self() {
	let (pairs, mut ext) = new_test_ext(2);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let mock_address = H160::from_low_u64_be(100);
		let transfer_value = decimal_convert(400, None);

		// Transfer from
		let mut nonce = 0;
		construct_tx_asserter(
			nonce,
			EvmDataWriter::new_with_selector(Action::TransferFrom)
				.write::<Address>(alice.address.into())
				.write::<Address>(mock_address.into())
				.write::<U256>(transfer_value)
				.build(),
			alice,
		)
		.execute()
		.assert_executed_value(&EvmDataWriter::new().write(true).build());

		// Check source account balance
		nonce += 1;
		construct_tx_asserter(
			nonce,
			EvmDataWriter::new_with_selector(Action::BalanceOf)
				.write::<Address>(alice.address.into())
				.build(),
			alice,
		)
		.execute()
		.assert_executed_value(
			&EvmDataWriter::new().write(decimal_convert(INITIAL_BALANCE - 400, None)).build(),
		);

		// Check target account balance
		nonce += 1;
		construct_tx_asserter(
			nonce,
			EvmDataWriter::new_with_selector(Action::BalanceOf)
				.write::<Address>(mock_address.into())
				.build(),
			alice,
		)
		.execute()
		.assert_executed_value(&EvmDataWriter::new().write(decimal_convert(400, None)).build());
	});
}
