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

#![cfg(feature = "runtime-benchmarks")]

//! Benchmarking
use frame_benchmarking::benchmarks;
use rlp::RlpStream;
use sha3::{Digest, Keccak256};
use sp_core::{H160, U256};
use sp_std::prelude::*;

use crate::{runner::Runner, Config, Pallet};
#[cfg(test)]
fn new_test_ext() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::default().build_storage::<crate::mock::Test>().unwrap();
	sp_io::TestExternalities::new(t)
}

benchmarks! {

	// This benchmark tests the relationship between gas and weight. It deploys a contract which
	// has an infinite loop in a public function. We then call this function with varying amounts
	// of gas, expecting it to OOG. The benchmarking framework measures the amount of time (aka
	// weight) it takes before OOGing and relates that to the amount of gas provided, leaving us
	// with an estimate for gas-to-weight mapping.
	runner_execute {

		let x in 1..10000000;

		// contract bytecode below is for:
		//
		// pragma solidity >=0.8.0;
		//
		// contract InfiniteContractVar {
		//     uint public count;

		//     constructor() public {
		//         count = 0;
		//     }

		//     function infinite() public {
		//         while (true) {
		//             count=count+1;
		//         }
		//     }
		// }

		let contract_bytecode = array_bytes::hex2bytes("0x608060405234801561001057600080fd5b506000808190555061017c806100276000396000f3fe608060405234801561001057600080fd5b50600436106100365760003560e01c806306661abd1461003b5780635bec9e6714610059575b600080fd5b610043610063565b604051610050919061009c565b60405180910390f35b610061610069565b005b60005481565b5b60011561008b57600160005461008091906100b7565b60008190555061006a565b565b6100968161010d565b82525050565b60006020820190506100b1600083018461008d565b92915050565b60006100c28261010d565b91506100cd8361010d565b9250827fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff0382111561010257610101610117565b5b828201905092915050565b6000819050919050565b7f4e487b7100000000000000000000000000000000000000000000000000000000600052601160045260246000fdfea26469706673582212207891ca3e64b75f98d843216c9d58cb2867b04996d7d4d731187616e330987fc764736f6c63430008040033").unwrap();
		let caller = H160::default();

		let mut nonce: u64 = 0;
		let nonce_as_u256: U256 = nonce.into();

		let value = U256::default();
		let gas_limit_create: u64 = 1_250_000 * 1_000_000_000;
		let is_transactional = true;
		let create_runner_results = T::Runner::create(
			caller,
			contract_bytecode,
			value,
			gas_limit_create,
			None,
			None,
			Some(nonce_as_u256),
			Vec::new(),
			is_transactional,
			T::config(),
		);
		assert_eq!(create_runner_results.is_ok(), true, "create() failed");

		// derive the resulting contract address from our create
		let mut rlp = RlpStream::new_list(2);
		rlp.append(&caller);
		rlp.append(&0u8);
		let contract_address = H160::from_slice(&Keccak256::digest(&rlp.out())[12..]);

		// derive encoded contract call -- in this case, just the function selector
		let mut encoded_call = vec![0u8; 4];
		encoded_call[0..4].copy_from_slice(&Keccak256::digest(b"infinite()")[0..4]);

		let gas_limit_call = x as u64;

	}: {

		nonce = nonce + 1;
		let nonce_as_u256: U256 = nonce.into();
		let is_transactional = true;

		let call_runner_results = T::Runner::call(
			caller,
			contract_address,
			encoded_call,
			value,
			gas_limit_call,
			None,
			None,
			Some(nonce_as_u256),
			Vec::new(),
			is_transactional,
			T::config(),
		);
		assert_eq!(call_runner_results.is_ok(), true, "call() failed");
	}
	impl_benchmark_test_suite!(Pallet, self::new_test_ext(), crate::mock::Test);
}
