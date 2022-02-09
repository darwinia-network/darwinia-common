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
use array_bytes::{hex2bytes_unchecked, hex_into_unchecked};
use std::str::FromStr;
// --- darwinia-network ---
use crate::{
	*, {self as s2s_issuing},
};
use darwinia_support::evm::IntoAccountId;
use dp_asset::{TokenMetadata, NATIVE_TOKEN_TYPE};
use dp_contract::mapping_token_factory::s2s::S2sRemoteUnlockInfo;
use dp_s2s::CallParams;
use mock::*;

// --- paritytech ---
use bp_message_dispatch::CallOrigin;
use bp_runtime::messages::DispatchFeePayment;
use frame_support::assert_ok;
use frame_system::RawOrigin;
use sp_runtime::AccountId32;

#[test]
fn burn_and_remote_unlock_success() {
	let (_, mut ext) = new_test_ext(1);
	ext.execute_with(|| {
		let original_token = H160::from_str("1000000000000000000000000000000000000001").unwrap();
		let burn_info = S2sRemoteUnlockInfo {
			spec_version: 0,
			weight: 100,
			token_type: 0,
			original_token,
			amount: U256::from(1),
			recipient: [1; 32].to_vec(),
		};
		let submitter = HashedConverter::into_account_id(
			H160::from_str("1000000000000000000000000000000000000002").unwrap(),
		);
		<<Test as s2s_issuing::Config>::OutboundPayloadCreator as CreatePayload<
			_,
			MultiSigner,
			MultiSignature,
		>>::create(
			CallOrigin::SourceAccount(submitter),
			burn_info.spec_version,
			burn_info.weight,
			CallParams::S2sBackingPalletUnlockFromRemote(
				original_token,
				U256::from(1),
				[1; 32].to_vec(),
			),
			DispatchFeePayment::AtSourceChain,
		)
		.unwrap();
	});
}

#[test]
fn register_and_issue_from_remote_success() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];
	ext.execute_with(|| {
		let t = LegacyUnsignedTransaction {
			nonce: U256::zero(),
			gas_price: U256::from(1),
			gas_limit: U256::from(0x100000),
			action: ethereum::TransactionAction::Create,
			value: U256::zero(),
			input: hex2bytes_unchecked(TEST_CONTRACT_BYTECODE),
		}
		.sign(&alice.private_key);
		// assert_ok!(Ethereum::execute(
		// 	alice.address,
		// 	t.input,
		// 	t.value,
		// 	t.gas_limit,
		// 	Some(t.gas_price),
		// 	Some(t.nonce),
		// 	t.action,
		// 	None,
		// ));
		assert_ok!(Ethereum::execute(alice.address, &t.into(), None,));
		let mapping_token_factory_address: H160 =
			array_bytes::hex_into_unchecked("32dcab0ef3fb2de2fce1d2e0799d36239671f04a");
		assert_ok!(S2sIssuing::set_mapping_factory_address(
			Origin::root(),
			mapping_token_factory_address,
		));
		let remote_root_address = hex2bytes_unchecked(
			"0xaaa5b780fa60c639ad17212d92e8e6257cb468baa88e1f826e6fe8ae6b7b700c",
		);
		let remote_backing_account: AccountId32 =
			AccountId32::decode(&mut &remote_root_address[..]).unwrap_or_default();
		let original_token_address = hex_into_unchecked("0000000000000000000000000000000000000002");
		let token = TokenMetadata::new(
			NATIVE_TOKEN_TYPE,
			original_token_address,
			[10u8; 32].to_vec(),
			[20u8; 32].to_vec(),
			18u8,
		);
		let drived_remote_backing_account: AccountId32 =
			hex_into_unchecked("77c1308128b230173f735cb97d6c62e5d8eeb86b148ff8461835c836945b1d84");
		let backing_address = <Test as s2s_issuing::Config>::ToEthAddressT::into_ethereum_id(
			&drived_remote_backing_account,
		);

		assert_ok!(S2sIssuing::set_remote_backing_account(
			RawOrigin::Root.into(),
			remote_backing_account.clone()
		));

		// before register, the mapping token address is Zero
		assert_eq!(
			S2sIssuing::mapped_token_address(backing_address, original_token_address).unwrap(),
			H160::from_str("0000000000000000000000000000000000000000").unwrap()
		);
		assert_ok!(S2sIssuing::register_from_remote(
			Origin::signed(drived_remote_backing_account.clone()),
			token
		));
		let mapping_token =
			S2sIssuing::mapped_token_address(backing_address, original_token_address).unwrap();
		// after register, the mapping token address is 0x0000000000000000000000000000000000000001
		assert_eq!(
			mapping_token,
			H160::from_str("0000000000000000000000000000000000000001").unwrap()
		);
		let recipient = H160::from_str("1000000000000000000000000000000000000000").unwrap();
		assert_ok!(S2sIssuing::issue_from_remote(
			Origin::signed(drived_remote_backing_account.clone()),
			original_token_address,
			U256::from(10_000_000_000u128),
			recipient
		));
	});
}
