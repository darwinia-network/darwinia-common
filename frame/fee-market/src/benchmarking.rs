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

//! Benchmarking
#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pallet as FeeMarket;
use frame_benchmarking::{account, benchmarks};
use frame_support::assert_ok;
use frame_system::RawOrigin;

const SEED: u32 = 0;

fn fee_market_ready<T: Config>() {
	let caller0: T::AccountId = account("source", 0, SEED);
	let caller1: T::AccountId = account("source", 1, SEED);
	let caller2: T::AccountId = account("source", 2, SEED);
	let caller3: T::AccountId = account("source", 3, SEED);
	let collateral = T::MiniumLockCollateral::get();
	T::RingCurrency::make_free_balance_be(&caller0, collateral.saturating_mul(10u32.into()));
	T::RingCurrency::make_free_balance_be(&caller1, collateral.saturating_mul(10u32.into()));
	T::RingCurrency::make_free_balance_be(&caller2, collateral.saturating_mul(10u32.into()));
	T::RingCurrency::make_free_balance_be(&caller3, collateral.saturating_mul(10u32.into()));
	assert_ne!(caller0, caller1);
	assert_ne!(caller1, caller2);

	assert_ok!(<FeeMarket<T>>::enroll_and_lock_collateral(
		RawOrigin::Signed(caller0).into(),
		collateral,
		None
	));
	assert_ok!(<FeeMarket<T>>::enroll_and_lock_collateral(
		RawOrigin::Signed(caller1).into(),
		collateral,
		None
	));
	assert_ok!(<FeeMarket<T>>::enroll_and_lock_collateral(
		RawOrigin::Signed(caller2).into(),
		collateral,
		None
	));
	assert_ok!(<FeeMarket<T>>::enroll_and_lock_collateral(
		RawOrigin::Signed(caller3).into(),
		collateral,
		None
	));
	assert!(<FeeMarket<T>>::market_fee().is_some());
	assert_eq!(<FeeMarket<T>>::relayers().len(), 4);
}

benchmarks! {
	enroll_and_lock_collateral {
		fee_market_ready::<T>();
		let relayer: T::AccountId = account("source", 100, SEED);
		T::RingCurrency::make_free_balance_be(&relayer, T::MiniumLockCollateral::get().saturating_mul(10u32.into()));
		let lock_collateral = T::MiniumLockCollateral::get().saturating_mul(5u32.into());
	}: enroll_and_lock_collateral(RawOrigin::Signed(relayer.clone()), lock_collateral, None)
	verify {
		assert!(<FeeMarket<T>>::is_enrolled(&relayer));
		assert_eq!(<FeeMarket<T>>::relayers().len(), 5);
	}

	update_locked_collateral {
		fee_market_ready::<T>();
		let caller3: T::AccountId = account("source", 3, SEED);
		let new_collateral = T::MiniumLockCollateral::get().saturating_mul(5u32.into());
	}: update_locked_collateral(RawOrigin::Signed(caller3.clone()), new_collateral)
	verify {
		let relayer = <FeeMarket<T>>::get_relayer(&caller3);
		assert_eq!(relayer.collateral,  T::MiniumLockCollateral::get().saturating_mul(5u32.into()));
	}

	update_relay_fee {
		fee_market_ready::<T>();
		let caller3: T::AccountId = account("source", 3, SEED);
		let new_fee = T::MinimumRelayFee::get().saturating_mul(10u32.into());
	}: update_relay_fee(RawOrigin::Signed(caller3.clone()), new_fee)
	verify {
		let relayer = <FeeMarket<T>>::get_relayer(&caller3);
		assert_eq!(relayer.fee,  T::MinimumRelayFee::get().saturating_mul(10u32.into()));
	}

	cancel_enrollment {
		fee_market_ready::<T>();
		let caller1: T::AccountId = account("source", 1, SEED);
	}: cancel_enrollment(RawOrigin::Signed(caller1.clone()))
	verify {
		assert!(!<FeeMarket<T>>::is_enrolled(&caller1));
		assert_eq!(<FeeMarket<T>>::relayers().len(), 3);
	}

}
