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

// --- std ---
use std::str::FromStr;
// --- crates.io ---
use array_bytes::{hex2bytes_unchecked, hex_into_unchecked};
// --- darwinia-network ---
use crate::*;
use darwinia_evm_precompile_utils::test_helper::{AccountInfo, LegacyUnsignedTransaction};
use dp_asset::{TokenMetadata, NATIVE_TOKEN_TYPE};
use mock::*;
// --- paritytech ---
use frame_support::assert_ok;
use frame_system::RawOrigin;
use sp_runtime::AccountId32;

fn alice_create(alice: &AccountInfo, input: Vec<u8>, nonce: u32) {
	let gas_limit_create: u64 = 1_250_000 * 1_000_000_000;
	let t = LegacyUnsignedTransaction {
		nonce: U256::from(nonce),
		gas_price: U256::from(1),
		gas_limit: U256::from(gas_limit_create),
		action: ethereum::TransactionAction::Create,
		value: U256::zero(),
		input,
	}
	.sign_with_chain_id(&alice.private_key, 42);
	assert_ok!(Ethereum::execute(alice.address, &t.into(), None,));
}

fn alice_call(alice: &AccountInfo, input: Vec<u8>, nonce: u32, contract: H160, value: u128) {
	let gas_limit_call: u64 = 250_000 * 1_000_000_000;
	let t = LegacyUnsignedTransaction {
		nonce: U256::from(nonce),
		gas_price: U256::from(1),
		gas_limit: U256::from(gas_limit_call),
		action: ethereum::TransactionAction::Call(contract),
		value: U256::from(value),
		input,
	}
	.sign_with_chain_id(&alice.private_key, 42);
	assert_ok!(Ethereum::execute(alice.address, &t.into(), None,));
}

fn configure_mapping_token_factory(alice: &AccountInfo) {
	let mapping_token_factory_address: H160 =
		array_bytes::hex_into_unchecked("32dcab0ef3fb2de2fce1d2e0799d36239671f04a");
	// initialize, then the owner is system account
	let initialize: Vec<u8> = hex2bytes_unchecked("0x8129fc1c").to_vec();
	alice_call(&alice, initialize, 2, mapping_token_factory_address, 0);
	// setTokenContractLogic
	let set_token_contract_logic0 = hex2bytes_unchecked("0x3c547e160000000000000000000000000000000000000000000000000000000000000000000000000000000000000000248e85939e48ca12a20cdf80e60d9e3d380ca7f9");
	let set_token_contract_logic1 = hex2bytes_unchecked("0x3c547e160000000000000000000000000000000000000000000000000000000000000001000000000000000000000000248e85939e48ca12a20cdf80e60d9e3d380ca7f9");
	alice_call(&alice, set_token_contract_logic0, 3, mapping_token_factory_address, 0);
	alice_call(&alice, set_token_contract_logic1, 4, mapping_token_factory_address, 0);
}

#[test]
fn register_and_issue_from_remote_success() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];
	ext.execute_with(|| {
        alice_create(&alice, hex2bytes_unchecked(MAPPING_TOKEN_FACTORY_CONTRACT_BYTECODE.trim_end()), 0);
        alice_create(&alice, hex2bytes_unchecked(MAPPING_TOKEN_LOGIC_CONTRACT_BYTECODE.trim_end()), 1);
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
        let backing_address = drived_remote_backing_account.derive_ethereum_address();

        assert_ok!(S2sIssuing::set_remote_backing_account(
                RawOrigin::Root.into(),
                remote_backing_account.clone()
                ));

        // before register, the mapping token address is Zero
        assert_eq!(
            S2sIssuing::mapped_token_address(backing_address, original_token_address).unwrap(),
            H160::from_str("0000000000000000000000000000000000000000").unwrap()
            );
        configure_mapping_token_factory(&alice);
        assert_ok!(S2sIssuing::register_from_remote(
                Origin::signed(drived_remote_backing_account.clone()),
                token
                ));
        let mapping_token =
            S2sIssuing::mapped_token_address(backing_address, original_token_address).unwrap();
        // after register, the mapping token address is 0x0000000000000000000000000000000000000001
        assert_eq!(
            mapping_token,
            H160::from_str("deb21a862ebe470d8982423a03d525b50ea66c8c").unwrap()
            );

        //setDailyLimit
        let set_dailylimit = hex2bytes_unchecked("0x2803212f000000000000000000000000deb21a862ebe470d8982423a03d525b50ea66c8c000000000000000000000000000000000000000000000000002386f26fc10000");
        alice_call(&alice, set_dailylimit, 5, mapping_token_factory_address, 0);
        let alice_h160 = H160::from_str("1a642f0e3c3af545e7acbd38b07251b3990914f1").unwrap();
        assert_ok!(S2sIssuing::issue_from_remote(
                Origin::signed(drived_remote_backing_account.clone()),
                original_token_address,
                U256::from(10_000_000_000u128),
                alice_h160
                ));
        let balance_of_alice = hex2bytes_unchecked("0x70a082310000000000000000000000001a642f0e3c3af545e7acbd38b07251b3990914f1");

        let balance = Ethereum::read_only_call(mapping_token, balance_of_alice.clone()).unwrap();
        assert_eq!(
            U256::from_big_endian(balance.as_slice()),
            U256::from(10_000_000_000u128)
            );

        // approve(
        //     0x32dcab0ef3fb2de2fce1d2e0799d36239671f04a,
        //     0x0fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
        //  );
        let approve_mtf = hex2bytes_unchecked("0x095ea7b300000000000000000000000032dcab0ef3fb2de2fce1d2e0799d36239671f04a0fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
        alice_call(&alice, approve_mtf, 6, mapping_token, 0);

        // let balance_of_factory = hex2bytes_unchecked("0x70a0823100000000000000000000000032dcab0ef3fb2de2fce1d2e0799d36239671f04a");
        //
        // let balance_factory_before = Ethereum::read_only_call(mapping_token, balance_of_factory.clone()).unwrap();
        //
        // assert_eq!(
        //     U256::from_big_endian(balance_factory_before.as_slice()),
        //     U256::from(9_999_990_000u128)
        // );

        // burnAndRemoteUnlockWaitingConfirm(
        //     1,
        //     1,
        //     0xdeb21a862ebe470d8982423a03d525b50ea66c8c,
        //     0x10101010101010101010101010101010,
        //     10000
        //  );
        let burn_info = hex2bytes_unchecked("0x0c90193d00000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000001000000000000000000000000deb21a862ebe470d8982423a03d525b50ea66c8c00000000000000000000000000000000000000000000000000000000000000a0000000000000000000000000000000000000000000000000000000000000271000000000000000000000000000000000000000000000000000000000000000101010101010101010101010101010101000000000000000000000000000000000");
        alice_call(&alice, burn_info, 7, mapping_token_factory_address, 10000);

        let balance_after = Ethereum::read_only_call(mapping_token, balance_of_alice).unwrap();
        assert_eq!(
            U256::from_big_endian(balance_after.as_slice()),
            U256::from(9_999_990_000u128)
            );

        // let balance_factory_after = Ethereum::read_only_call(mapping_token, balance_of_factory).unwrap();
        //
        // assert_eq!(
        //     U256::from_big_endian(balance_factory_after.as_slice()),
        //     U256::from(9_999_990_000u128)
        // );
    });
}

#[test]
fn test_judge_self_message() {
    let (_, mut ext) = new_test_ext(1);
    ext.execute_with(|| {
        use crate::Pallet as S2sIssuing;
        assert_ok!(<S2sIssuing<Test>>::set_mapping_factory_address(RawOrigin::Root.into(), H160::from_str("32dcab0ef3fb2de2fce1d2e0799d36239671f04a").unwrap()));
        assert_ok!(<S2sIssuing<Test>>::judge_self_message(0));
    });
}
