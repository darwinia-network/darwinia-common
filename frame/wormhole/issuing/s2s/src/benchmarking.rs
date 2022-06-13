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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

#![cfg(feature = "runtime-benchmarks")]

// --- crates.io ---
use array_bytes::{hex2bytes_unchecked, hex_into_unchecked};
// darwinia-network
use super::*;
use crate::Pallet as S2sIssuing;
use darwinia_evm::Runner;
use dp_asset::{TokenMetadata, NATIVE_TOKEN_TYPE};
// --- paritytech ---
use frame_benchmarking::benchmarks;
use frame_support::assert_ok;
use frame_system::RawOrigin;

// S2S mapping token factory contract
// https://github.com/darwinia-network/darwinia-bridges-sol/blob/master/contracts/wormhole/contracts/mapping-token/darwinia/Sub2SubMappingTokenFactory.sol
pub const MAPPING_TOKEN_FACTORY_CONTRACT_BYTECODE: &str =
	include_str!("./res/mapping_token_factory_bytecode.txt");
// https://github.com/darwinia-network/darwinia-bridges-sol/blob/master/contracts/wormhole/contracts/mapping-token/darwinia/MappingERC20.sol
pub const MAPPING_TOKEN_LOGIC_CONTRACT_BYTECODE: &str =
	include_str!("./res/mapping_erc20_bytecode.txt");

fn deploy_mapping_token_factory<T: Config>() -> H160 {
	let contract_bytecode = hex2bytes_unchecked(MAPPING_TOKEN_FACTORY_CONTRACT_BYTECODE);
	let creator = H160::default();

	let nonce: U256 = U256::zero();

	let value = U256::default();
	let gas_limit_create: u64 = 1_250_000 * 1_000_000_000;
	let create_runner_results = <T as darwinia_evm::Config>::Runner::create(
		creator,
		contract_bytecode,
		value,
		gas_limit_create,
		None,
		None,
		Some(nonce),
		Vec::new(),
		T::config(),
	);
	assert_eq!(create_runner_results.is_ok(), true, "create() failed");
	hex_into_unchecked("bd770416a3345f91e4b34576cb804a576fa48eb1")
}

fn deploy_mapping_token_logic<T: Config>() -> H160 {
	let contract_bytecode = hex2bytes_unchecked(MAPPING_TOKEN_LOGIC_CONTRACT_BYTECODE);
	let creator = H160::default();

	let nonce: U256 = U256::one();

	let value = U256::default();
	let gas_limit_create: u64 = 1_250_000 * 1_000_000_000;
	let create_runner_results = <T as darwinia_evm::Config>::Runner::create(
		creator,
		contract_bytecode,
		value,
		gas_limit_create,
		None,
		None,
		Some(nonce),
		Vec::new(),
		T::config(),
	);
	assert_eq!(create_runner_results.is_ok(), true, "create() failed");
	hex_into_unchecked("bd770416a3345f91e4b34576cb804a576fa48eb1")
}

fn configure_mapping_token_factory<T: Config>() {
	let mapping_token_factory: H160 =
		hex_into_unchecked("bd770416a3345f91e4b34576cb804a576fa48eb1");
	// initialize, then the owner is system account
	let initialize: Vec<u8> = hex2bytes_unchecked("0x8129fc1c").to_vec();
	assert_ok!(T::InternalTransactHandler::internal_transact(mapping_token_factory, initialize));
	// setTokenContractLogic
	let set_token_contract_logic0 = hex2bytes_unchecked("0x3c547e1600000000000000000000000000000000000000000000000000000000000000000000000000000000000000005a443704dd4b594b382c22a083e2bd3090a6fef3");
	let set_token_contract_logic1 = hex2bytes_unchecked("0x3c547e1600000000000000000000000000000000000000000000000000000000000000010000000000000000000000005a443704dd4b594b382c22a083e2bd3090a6fef3");
	assert_ok!(T::InternalTransactHandler::internal_transact(
		mapping_token_factory,
		set_token_contract_logic0
	));
	assert_ok!(T::InternalTransactHandler::internal_transact(
		mapping_token_factory,
		set_token_contract_logic1
	));
}

benchmarks! {
	register_from_remote {
		let addr_bytes = hex2bytes_unchecked("0xef5618270c59c8cae389b45f5528012a62fde85b9a14f91e668a746a30ff5018");
		let caller: T::AccountId = T::AccountId::decode(&mut &addr_bytes[..]).unwrap_or_default();

		let register_token_address = hex_into_unchecked("0000000000000000000000000000000000000002");
		let token_metadata = TokenMetadata::new(
			NATIVE_TOKEN_TYPE,
			register_token_address,
			[10; 32].to_vec(),
			[20; 32].to_vec(),
			18,
		);

		let contract_address = deploy_mapping_token_factory::<T>();
		let mapping_token_logic = deploy_mapping_token_logic::<T>();
		configure_mapping_token_factory::<T>();
		assert_ok!(<S2sIssuing<T>>::set_mapping_factory_address(
			RawOrigin::Root.into(),
			contract_address
		));
	}: _(RawOrigin::Signed(caller), token_metadata)

	issue_from_remote {
		let addr_bytes = hex2bytes_unchecked("0xef5618270c59c8cae389b45f5528012a62fde85b9a14f91e668a746a30ff5018");
		let caller: T::AccountId = T::AccountId::decode(&mut &addr_bytes[..]).unwrap_or_default();
		let issue_token_address = hex_into_unchecked("0000000000000000000000000000000000000002");
		let token_metadata = TokenMetadata::new(
			NATIVE_TOKEN_TYPE,
			issue_token_address,
			[10; 32].to_vec(),
			[20; 32].to_vec(),
			18,
		);
		let recipient = hex_into_unchecked("0000000000000000000000000000000000000001");

		let contract_address = deploy_mapping_token_factory::<T>();
		let mapping_token_logic = deploy_mapping_token_logic::<T>();
		configure_mapping_token_factory::<T>();
		assert_ok!(<S2sIssuing<T>>::set_mapping_factory_address(
			RawOrigin::Root.into(),
			contract_address
		));
		assert_ok!(<S2sIssuing<T>>::register_from_remote(
			RawOrigin::Signed(caller.clone()).into(),
			token_metadata
		));
		//setDailyLimit
		let set_dailylimit = hex2bytes_unchecked("0x2803212f000000000000000000000000121a7342c6bc11d73bc952bdff26ec18c5f257f7000000000000000000000000000000000000000000000000002386f26fc10000");
		assert_ok!(T::InternalTransactHandler::internal_transact(
				contract_address,
				set_dailylimit
		));
	}: _(RawOrigin::Signed(caller), issue_token_address, U256::from(10_000_000_000u128), recipient)

	set_mapping_factory_address {
		let address = hex_into_unchecked("0000000000000000000000000000000000000001");
	}: _(RawOrigin::Root, address)
	set_remote_backing_account {
		let addr_bytes = hex2bytes_unchecked("0x0000000000000000000000000000000000000000000000000000000000000000");
		let address: T::AccountId = T::AccountId::decode(&mut &addr_bytes[..]).unwrap_or_default();
	}: _(RawOrigin::Root, address)
}
