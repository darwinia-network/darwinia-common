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

use super::*;
use crate::adapter::{RemainBalanceOp, RingRemainBalance};
use darwinia_evm::CurrencyAdapt;
use darwinia_support::evm::{decimal_convert, DeriveSubstrateAddress};

macro_rules! assert_balance {
	($evm_address:expr, $balance:expr, $left:expr, $right:expr) => {
		let account_id =
			<Test as darwinia_evm::Config>::IntoAccountId::derive_substrate_address($evm_address);
		assert_eq!(RingBalanceAdapter::evm_balance(&$evm_address), $balance);
		assert_eq!(Ring::free_balance(&account_id), $left);
		assert_eq!(
			<RingRemainBalance as RemainBalanceOp<Test>>::remaining_balance(&account_id),
			$right
		);
	};
}

#[test]
fn mutate_account_works_well() {
	let (_, mut ext) = new_test_ext(1);
	ext.execute_with(|| {
		let test_addr = H160::from_str("1000000000000000000000000000000000000001").unwrap();
		let origin = decimal_convert(123456789, Some(90));
		RingBalanceAdapter::mutate_evm_balance(&test_addr, origin);
		assert_balance!(&test_addr, origin, 123456789, 90);
	});
}

#[test]
fn mutate_account_inc_balance_by_10() {
	let (_, mut ext) = new_test_ext(1);
	ext.execute_with(|| {
		let test_addr = H160::from_str("1000000000000000000000000000000000000001").unwrap();
		let origin = decimal_convert(600, Some(90));
		RingBalanceAdapter::mutate_evm_balance(&test_addr, origin);

		let new = origin.saturating_add(U256::from(10));
		RingBalanceAdapter::mutate_evm_balance(&test_addr, new);
		assert_balance!(&test_addr, new, 600, 100);
	});
}

#[test]
fn mutate_account_inc_balance_by_999_999_910() {
	let (_, mut ext) = new_test_ext(1);
	ext.execute_with(|| {
		let test_addr = H160::from_str("1000000000000000000000000000000000000001").unwrap();
		let origin = decimal_convert(600, Some(90));
		RingBalanceAdapter::mutate_evm_balance(&test_addr, origin);

		let new = origin.saturating_add(U256::from(999999910u128));
		RingBalanceAdapter::mutate_evm_balance(&test_addr, new);
		assert_balance!(&test_addr, new, 601, 0);
	});
}

#[test]
fn mutate_account_inc_by_1000_000_000() {
	let (_, mut ext) = new_test_ext(1);
	ext.execute_with(|| {
		let test_addr = H160::from_str("1000000000000000000000000000000000000001").unwrap();
		let origin = decimal_convert(600, Some(90));
		RingBalanceAdapter::mutate_evm_balance(&test_addr, origin);

		let new = origin.saturating_add(U256::from(1000_000_000u128));
		RingBalanceAdapter::mutate_evm_balance(&test_addr, new);
		assert_balance!(&test_addr, new, 601, 90);
	});
}

#[test]
fn mutate_account_dec_balance_by_90() {
	let (_, mut ext) = new_test_ext(1);
	ext.execute_with(|| {
		let test_addr = H160::from_str("1000000000000000000000000000000000000001").unwrap();
		let origin = decimal_convert(600, Some(90));
		RingBalanceAdapter::mutate_evm_balance(&test_addr, origin);

		let new = origin.saturating_sub(U256::from(90));
		RingBalanceAdapter::mutate_evm_balance(&test_addr, new);
		assert_balance!(&test_addr, new, 600, 0);
	});
}
#[test]
fn mutate_account_dec_balance_by_990() {
	let (_, mut ext) = new_test_ext(1);
	ext.execute_with(|| {
		let test_addr = H160::from_str("1000000000000000000000000000000000000001").unwrap();
		let origin = decimal_convert(600, Some(90));
		RingBalanceAdapter::mutate_evm_balance(&test_addr, origin);

		let new = origin.saturating_sub(U256::from(990));
		RingBalanceAdapter::mutate_evm_balance(&test_addr, new);
		assert_balance!(&test_addr, new, 599, 1_000_000_090 - 990);
	});
}
#[test]
fn mutate_account_dec_balance_existential_by_90() {
	let (_, mut ext) = new_test_ext(1);
	ext.execute_with(|| {
		let test_addr = H160::from_str("1000000000000000000000000000000000000001").unwrap();
		let origin = decimal_convert(500, Some(90));
		RingBalanceAdapter::mutate_evm_balance(&test_addr, origin);

		let new = origin.saturating_sub(U256::from(90));
		RingBalanceAdapter::mutate_evm_balance(&test_addr, new);
		assert_balance!(&test_addr, new, 500, 0);
	});
}
