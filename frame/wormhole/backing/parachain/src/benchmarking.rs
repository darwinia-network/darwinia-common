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
use array_bytes::hex2bytes_unchecked;
// --- paritytech ---
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;
use sp_runtime::{traits::Zero, SaturatedConversion};
// --- darwinia-network ---
use crate::{Pallet as ParachainBacking, *};

const SPEC_VERSION: u32 = 2_9_00_0;

benchmarks! {
	lock_and_remote_issue {
		let caller = whitelisted_caller();

		<T as Config>::RingCurrency::deposit_creating(
			&caller,
			5_000_000_000_u128.saturated_into::<RingBalance<T>>() * T::RingCurrency::minimum_balance()
		);

		let recipient_bytes = hex2bytes_unchecked("0x000000000000000000000000000000000000000000000000000000000000000001");
		let recipient = T::AccountId::decode(&mut &recipient_bytes[..]).unwrap_or_default();
	}: _(
			RawOrigin::Signed(caller),
			SPEC_VERSION,
			1000000,
			500_u128.saturated_into::<RingBalance<T>>() * T::RingCurrency::minimum_balance(),
			949_643_000_u128.saturated_into::<RingBalance<T>>() * T::RingCurrency::minimum_balance(),
			recipient
	)

	unlock_from_remote {
		let caller_bytes = hex2bytes_unchecked("0xafc06852848eea7f1c02654946058f55e5a8e6a4596af74c08301fc240ea1614");
		let caller = T::AccountId::decode(&mut &caller_bytes[..]).unwrap_or_default();

		<T as Config>::RingCurrency::deposit_creating(&caller, 100_u128.saturated_into::<RingBalance<T>>() * T::RingCurrency::minimum_balance());

		let pallet_account_id = <ParachainBacking<T>>::pallet_account_id();

		<T as Config>::RingCurrency::deposit_creating(&pallet_account_id, 5000_u128.saturated_into::<RingBalance<T>>() * T::RingCurrency::minimum_balance());

		let recipient_bytes = hex2bytes_unchecked("0x8e13b96a9c9e3b1832f07935be76c2b331251e26445f520ad1c56b24477ed8d6");
		let recipient = T::AccountId::decode(&mut &recipient_bytes[..]).unwrap_or_default();
	}:_(RawOrigin::Signed(caller), 100_u128.saturated_into::<RingBalance<T>>() * T::RingCurrency::minimum_balance(), recipient)

	set_secure_limited_period {
		let period = T::BlockNumber::zero();
	}:_(RawOrigin::Root, period)

	set_security_limitation_ring_amount {
		let limitation = <RingBalance<T>>::zero();
	}:_(RawOrigin::Root, limitation)

	set_remote_mapping_token_factory_account {
		let addr_bytes = hex2bytes_unchecked("0x0000000000000000000000000000000000000000000000000000000000000000");
		let address = T::AccountId::decode(&mut &addr_bytes[..]).unwrap_or_default();
	}:_(RawOrigin::Root, address)
}
