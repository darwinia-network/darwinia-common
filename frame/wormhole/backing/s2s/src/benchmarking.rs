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
// --- paritytech ---
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;
use sp_runtime::traits::Zero;
// --- darwinia-network ---
use crate::*;

const SPEC_VERSION: u32 = 123;
const INIT_COIN: u128 = 5_000_000_000_000_000_000;

benchmarks! {
	register_and_remote_create {
		let caller = whitelisted_caller();
		<T as Config>::RingCurrency::deposit_creating(&caller, INIT_COIN.saturated_into());
	}:_(RawOrigin::Signed(caller), SPEC_VERSION, 1000000, 949_643_000u128.saturated_into())

	lock_and_remote_issue {
		let caller = whitelisted_caller();
		<T as Config>::RingCurrency::deposit_creating(&caller, INIT_COIN.saturated_into());
		let recipient = hex_into_unchecked("0000000000000000000000000000000000000001");
	}: _(RawOrigin::Signed(caller), SPEC_VERSION, 1000000,
			500u128.saturated_into(),
			949_643_000u128.saturated_into(),
			recipient
	)

	unlock_from_remote {
		let caller_bytes = hex2bytes_unchecked("0x8e13b96a9c9e3b1832f07935be76c2b331251e26445f520ad1c56b24477ed8dd");
		let caller: T::AccountId = T::AccountId::decode(&mut &caller_bytes[..]).unwrap_or_default();
		let addr_bytes = hex2bytes_unchecked("0x6d6f646c64612f73327362610000000000000000000000000000000000000000");
		let pallet_account_id: T::AccountId = T::AccountId::decode(&mut &addr_bytes[..]).unwrap_or_default();
		<T as Config>::RingCurrency::deposit_creating(&pallet_account_id, U256::from(5000).low_u128().saturated_into());
		let recipient_bytes = hex2bytes_unchecked("0x8e13b96a9c9e3b1832f07935be76c2b331251e26445f520ad1c56b24477ed8d6");

		let register_token_address = hex_into_unchecked("0000000000000000000000000000000000000002");
	}:_(RawOrigin::Signed(caller), register_token_address, 100.into(), recipient_bytes)

	set_secure_limited_period {
		let period: BlockNumberFor<T> = Zero::zero();
	}:_(RawOrigin::Root, period)

	set_security_limitation_ring_amount {
		let limitation: RingBalance<T> = Zero::zero();
	}:_(RawOrigin::Root, limitation)
}
