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

use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_system::RawOrigin;
use sp_runtime::traits::Bounded;
use sp_std::vec;

use crate::Pallet as Issuing;

benchmarks! {
	// dispatch handle benchmark
	dispatch_handle {
		// let caller = whitelisted_caller();
		// System::<T>::set_block_number(0u32.into());

	}:dispatch_handle(RawOrigin::None, vec![1, 2, 3, 4, 5, 6, 7, 8])
	// }: {
	// 	// dispatch_handle(RawOrigin::None, vec![1, 2, 3])

	// }
	verify {
		assert_eq!(1, 1);
	}

	// // remote register benchmark
	// remote_register {
	// 	// let caller = whitelisted_caller();

	// }: {
	// 	// remote_register(origin: OriginFor<T>, token: Token)
	// 	todo!()
	// }
	// verify {
	// 	assert_eq!(1, 1);
	// }

	// // remote_issue
	// remote_issue {
	// 	// let caller = whitelisted_caller();

	// }: {
	// 	// pub fn remote_issue(origin: OriginFor<T>, token: Token, recipient: H160)
	// 	todo!()
	// }
	// verify {
	// 	assert_eq!(1, 1);
	// }
}

impl_benchmark_test_suite!(Issuing, crate::tests::new_test_ext(), crate::tests::Test,);
