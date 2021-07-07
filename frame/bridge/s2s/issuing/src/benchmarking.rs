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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

#![cfg(feature = "runtime-benchmarks")]

use super::*;

use array_bytes::{hex2bytes_unchecked, hex_into_unchecked};
use dp_asset::token::{Token, TokenOption};
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_system::RawOrigin;
use sp_runtime::traits::Bounded;
use sp_runtime::traits::UniqueSaturatedInto;
use sp_std::str::FromStr;
use sp_std::vec;

use crate::Pallet as Issuing;

const SPEC_VERSION: u32 = 123;
const FACTORY_ADDR: &str = "0xE1586e744b99bF8e4C981DfE4dD4369d6f8Ed88A";

benchmarks! {
	asset_burn_event_handle {
		let caller = <T as darwinia_evm::Config>::AddressMapping::into_account_id(
			hex_into_unchecked(FACTORY_ADDR)
		);
		<T as Config>::RingCurrency::deposit_creating(&caller, U256::from(500).low_u128().unique_saturated_into());
		log::debug!("bear: --- benchmark: the caller is {:?}", caller);

		let mut input = vec![0; 4];
		let mut burn_action = &sha3::Keccak256::digest(&BURN_ACTION)[0..4];
		input.extend_from_slice(&mut burn_action);
		let token_info = TokenBurnInfo::encode(
			SPEC_VERSION,
			10000,
			0,
			hex_into_unchecked("0000000000000000000000000000000000000001"),
			hex_into_unchecked("0000000000000000000000000000000000000002"),
			hex_into_unchecked("0000000000000000000000000000000000000003"),
			vec![1; 32],
			U256::from(250),
			U256::from(10_000_000_000u128),
		);
		input.extend_from_slice(&token_info);
	}:asset_burn_event_handle(RawOrigin::Signed(caller), input)

	register_from_remote {
		let addr_bytes = hex2bytes_unchecked("0x8e13b96a9c9e3b1832f07935be76c2b331251e26445f520ad1c56b24477ed8dd");
		let caller: T::AccountId = T::AccountId::decode(&mut &addr_bytes[..]).unwrap_or_default();

		let register_token_address = H160::from_str("0000000000000000000000000000000000000002").unwrap();
		let token_option = TokenOption {
			name: [10; 32],
			symbol: [20; 32],
			decimal: 18,
		};
		let token = Token::Native(TokenInfo::new(register_token_address, None, Some(token_option)));
	}: register_from_remote(RawOrigin::Signed(caller), token)

	// // remote_issue
	// remote_issue {
	// 	let caller: T::AccountId = whitelisted_caller();
	// 	let token = Token::Native(TokenInfo::default());
	// }: _(RawOrigin::Signed(caller), token)
}

impl_benchmark_test_suite!(Issuing, crate::tests::new_test_ext(), crate::tests::Test,);
