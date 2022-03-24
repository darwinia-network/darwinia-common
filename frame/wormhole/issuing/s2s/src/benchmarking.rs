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

use super::*;
use crate::Pallet as S2sIssuing;

use array_bytes::{hex2bytes_unchecked, hex_into_unchecked};
use darwinia_evm::Runner;
use dp_asset::{TokenMetadata, NATIVE_TOKEN_TYPE};
use frame_benchmarking::benchmarks;
use frame_support::assert_ok;
use frame_system::RawOrigin;
/*
 * Test Contract
pragma solidity ^0.6.0;

contract MockS2SMappingTokenFactory {

	address public constant SYSTEM_ACCOUNT = 0x6D6F646C6461722f64766D700000000000000000;
	address public constant MOCKED_ADDRESS = 0x0000000000000000000000000000000000000001;

	mapping(bytes32 => address) public salt2MappingToken;

	event IssuingERC20Created(address backing_address, address original_token, address mapping_token);
	event MappingTokenIssued(address mapping_token, address recipient, uint256 amount);

	receive() external payable {
	}

	/**
	 * @dev Throws if called by any account other than the system account defined by SYSTEM_ACCOUNT address.
	 */
	modifier onlySystem() {
		require(SYSTEM_ACCOUNT == msg.sender, "System: caller is not the system account");
		_;
	}

	function mappingToken(address backing_address, address original_token) public view returns (address) {
		bytes32 salt = keccak256(abi.encodePacked(backing_address, original_token));
		return salt2MappingToken[salt];
	}

	function newErc20Contract(
		uint32,
		string memory,
		string memory,
		uint8,
		address backing_address,
		address original_token
	) public virtual onlySystem returns (address mapping_token) {
		bytes32 salt = keccak256(abi.encodePacked(backing_address, original_token));
		salt2MappingToken[salt] = MOCKED_ADDRESS;
		emit IssuingERC20Created(backing_address, original_token, MOCKED_ADDRESS);
		return MOCKED_ADDRESS;
	}

	function issueMappingToken(address mapping_token, address recipient, uint256 amount) public virtual onlySystem {
		require(mapping_token == MOCKED_ADDRESS, "invalid mapping token address");
		emit MappingTokenIssued(mapping_token, recipient, amount);
	}
}
*/
pub const TEST_CONTRACT_BYTECODE: &str = "608060405234801561001057600080fd5b506105ad806100206000396000f3fe6080604052600436106100595760003560e01c8063148a79fd14610065578063739d40d9146100ab578063b28bf620146100c0578063c8ff0854146100d5578063ecd22a191461011a578063ef13ef4d1461015557610060565b3661006057005b600080fd5b34801561007157600080fd5b5061008f6004803603602081101561008857600080fd5b50356102b6565b604080516001600160a01b039092168252519081900360200190f35b3480156100b757600080fd5b5061008f6102d1565b3480156100cc57600080fd5b5061008f6102e4565b3480156100e157600080fd5b50610118600480360360608110156100f857600080fd5b506001600160a01b038135811691602081013590911690604001356102e9565b005b34801561012657600080fd5b5061008f6004803603604081101561013d57600080fd5b506001600160a01b03813581169160200135166103e3565b34801561016157600080fd5b5061008f600480360360c081101561017857600080fd5b63ffffffff82351691908101906040810160208201356401000000008111156101a057600080fd5b8201836020820111156101b257600080fd5b803590602001918460018302840111640100000000831117156101d457600080fd5b91908080601f016020809104026020016040519081016040528093929190818152602001838380828437600092019190915250929594936020810193503591505064010000000081111561022757600080fd5b82018360208201111561023957600080fd5b8035906020019184600183028401116401000000008311171561025b57600080fd5b91908080601f0160208091040260200160405190810160405280939291908181526020018383808284376000920191909152509295505060ff8335169350506001600160a01b03602083013581169260400135169050610442565b6000602081905290815260409020546001600160a01b031681565b6b06d6f646c6461722f64766d760441b81565b600181565b6b06d6f646c6461722f64766d760441b33146103365760405162461bcd60e51b81526004018080602001828103825260288152602001806105506028913960400191505060405180910390fd5b6001600160a01b038316600114610394576040805162461bcd60e51b815260206004820152601d60248201527f696e76616c6964206d617070696e6720746f6b656e2061646472657373000000604482015290519081900360640190fd5b604080516001600160a01b0380861682528416602082015280820183905290517f4c965b0027d1a0b20e874218493f3717f065d312001e29e75c42d135c7ab96259181900360600190a1505050565b604080516bffffffffffffffffffffffff19606094851b81166020808401919091529390941b9093166034840152805160288185030181526048909301815282519282019290922060009081529081905220546001600160a01b031690565b60006b06d6f646c6461722f64766d760441b33146104915760405162461bcd60e51b81526004018080602001828103825260288152602001806105506028913960400191505060405180910390fd5b604080516bffffffffffffffffffffffff19606086811b82166020808501919091529086901b909116603483015282516028818403018152604883018085528151918301919091206000818152928390529184902080546001600160a01b03191660019081179091556001600160a01b038089169092529086166068840152608883015291517fc9c337e478378d4317643765b21b7d2da0d66f86675b2e3b6e1aff67ce572daf9181900360a80190a150600197965050505050505056fe53797374656d3a2063616c6c6572206973206e6f74207468652073797374656d206163636f756e74a26469706673582212202576f15b3a6363c8f6949d2605f736ff2b3b65e179f1788b5db7d244efebbc0d64736f6c63430006090033";

fn deploy_mapping_token_factory<T: Config>() -> H160 {
	let contract_bytecode = array_bytes::hex2bytes(TEST_CONTRACT_BYTECODE).unwrap();
	let creator = H160::default();

	let nonce: u64 = 0;
	let nonce_as_u256: U256 = nonce.into();

	let value = U256::default();
	let gas_limit_create: u64 = 1_250_000 * 1_000_000_000;
	let create_runner_results = <T as darwinia_evm::Config>::Runner::create(
		creator,
		contract_bytecode,
		value,
		gas_limit_create,
		None,
		None,
		Some(nonce_as_u256),
		Vec::new(),
		T::config(),
	);
	assert_eq!(create_runner_results.is_ok(), true, "create() failed");
	hex_into_unchecked("bd770416a3345f91e4b34576cb804a576fa48eb1")
}

benchmarks! {
	register_from_remote {
		let addr_bytes = hex2bytes_unchecked("0xaaa5b780fa60c639ad17212d92e8e6257cb468baa88e1f826e6fe8ae6b7b700c");
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
		assert_ok!(<S2sIssuing<T>>::set_mapping_factory_address(
			RawOrigin::Root.into(),
			contract_address
		));
	}: _(RawOrigin::Signed(caller), token_metadata)

	issue_from_remote {
		let addr_bytes = hex2bytes_unchecked("0xaaa5b780fa60c639ad17212d92e8e6257cb468baa88e1f826e6fe8ae6b7b700c");
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
		assert_ok!(<S2sIssuing<T>>::set_mapping_factory_address(
			RawOrigin::Root.into(),
			contract_address
		));
		assert_ok!(<S2sIssuing<T>>::register_from_remote(
			RawOrigin::Signed(caller.clone()).into(),
			token_metadata
		));
	}: _(RawOrigin::Signed(caller), issue_token_address, U256::from(10_000_000_000u128), recipient)

	set_mapping_factory_address {
		let address = hex_into_unchecked("0000000000000000000000000000000000000001");
	}: _(RawOrigin::Root, address)
}
