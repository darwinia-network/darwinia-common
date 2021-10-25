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

// --- crates.io ---
use std::str::FromStr;
// --- darwinia-network ---
use crate::{
	*, {self as s2s_issuing},
};
use darwinia_support::evm::IntoAccountId;
use dp_asset::token::TokenInfo;

use array_bytes::hex2bytes_unchecked;
use frame_support::assert_ok;

use mock::*;

#[test]
fn burn_and_remote_unlock_success() {
	let (_, mut ext) = new_test_ext(1);
	ext.execute_with(|| {
		let original_token = H160::from_str("1000000000000000000000000000000000000001").unwrap();
		let token: Token = (1, TokenInfo::new(original_token, Some(U256::from(1)), None)).into();
		let burn_info = S2sRemoteUnlockInfo {
			spec_version: 0,
			weight: 100,
			token,
			recipient: [1; 32].to_vec(),
		};
		let submitter = HashedConverter::into_account_id(
			H160::from_str("1000000000000000000000000000000000000002").unwrap(),
		);
		<Test as s2s_issuing::Config>::CallEncoder::encode_remote_unlock(submitter, burn_info)
			.unwrap();
	});
}

#[test]
fn register_from_remote_success() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];
	ext.execute_with(|| {
		let t = UnsignedTransaction {
			nonce: U256::zero(),
			gas_price: U256::from(1),
			gas_limit: U256::from(0x100000),
			action: ethereum::TransactionAction::Create,
			value: U256::zero(),
			input: hex2bytes_unchecked(TEST_CONTRACT_BYTECODE),
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
		let mapping_token_factory_address: H160 =
			array_bytes::hex_into_unchecked("32dcab0ef3fb2de2fce1d2e0799d36239671f04a");
		assert_ok!(S2sIssuing::set_mapping_factory_address(
			Origin::root(),
			mapping_token_factory_address,
		));
	});
}
