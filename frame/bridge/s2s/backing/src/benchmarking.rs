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

const SPEC_VERSION: u32 = 123;

use array_bytes::{hex2bytes_unchecked, hex_into_unchecked};
use dp_asset::token::{Token, TokenOption};
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;
use sp_runtime::traits::UniqueSaturatedInto;
const FACTORY_ADDR: &str = "0xE1586e744b99bF8e4C981DfE4dD4369d6f8Ed88A";

benchmarks! {
	register_and_remote_create {
		let w in 0..1000000;
		let caller = whitelisted_caller();
		<T as Config>::RingCurrency::deposit_creating(&caller, U256::from(5000).low_u128().unique_saturated_into());
	}:_(RawOrigin::Signed(caller), SPEC_VERSION, w.into(), U256::from(500).low_u128().unique_saturated_into())

	lock_and_remote_issue {
		let w in 0..1000000;
		let caller = whitelisted_caller();
		<T as Config>::RingCurrency::deposit_creating(&caller, U256::from(5000).low_u128().unique_saturated_into());
		let recipient = hex_into_unchecked("0000000000000000000000000000000000000001");

	}: _(RawOrigin::Signed(caller), SPEC_VERSION, w.into(),
			U256::from(500).low_u128().unique_saturated_into(),
			U256::from(100).low_u128().unique_saturated_into(),
			recipient
	)

	unlock_from_remote {
		let addr_bytes = hex2bytes_unchecked("0x8e13b96a9c9e3b1832f07935be76c2b331251e26445f520ad1c56b24477ed8dd");
		let caller: T::AccountId = T::AccountId::decode(&mut &addr_bytes[..]).unwrap_or_default();
		let addr_bytes = hex2bytes_unchecked("0x6d6f646c64612f73327362610000000000000000000000000000000000000000");
		let pallet_account_id: T::AccountId = T::AccountId::decode(&mut &addr_bytes[..]).unwrap_or_default();
		<T as Config>::RingCurrency::deposit_creating(&pallet_account_id, U256::from(5000).low_u128().unique_saturated_into());
		let register_token_address = hex_into_unchecked("0000000000000000000000000000000000000002");
		let token_option = TokenOption {
			name: [10; 32],
			symbol: [20; 32],
			decimal: 18,
		};
		let token = Token::Native(TokenInfo::new(register_token_address, Some(100.into()), Some(token_option)));
		let addr_bytes = hex2bytes_unchecked("0x8e13b96a9c9e3b1832f07935be76c2b331251e26445f520ad1c56b24477ed8d6");
		let recipient: T::AccountId = T::AccountId::decode(&mut &addr_bytes[..]).unwrap_or_default();
	}:_(RawOrigin::Signed(caller), token, recipient)
}
