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

//! Autogenerated weights for `pallet_membership`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2022-08-12, STEPS: `3`, REPEAT: 3, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("pangolin-dev"), DB CACHE: 128

// Executed Command:
// target/release/drml
// benchmark
// --header
// .maintain/lincense-header
// --execution
// wasm
// --heap-pages
// 4096
// --steps
// 3
// --repeat
// 3
// --chain
// pangolin-dev
// --output
// node/runtime/pangolin/src/weights/
// --extrinsic
// *
// --pallet
// *

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for `pallet_membership`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_membership::WeightInfo for WeightInfo<T> {
	// Storage: TechnicalMembership Members (r:1 w:1)
	// Storage: TechnicalCommittee Proposals (r:1 w:0)
	// Storage: DarwiniaEthereumRelay PendingRelayHeaderParcels (r:1 w:0)
	// Storage: TechnicalCommittee Members (r:0 w:1)
	// Storage: TechnicalCommittee Prime (r:0 w:1)
	fn add_member(m: u32, ) -> Weight {
		(20_031_000 as Weight)
			// Standard Error: 46_000
			.saturating_add((214_000 as Weight).saturating_mul(m as Weight))
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().writes(3 as Weight))
	}
	// Storage: TechnicalMembership Members (r:1 w:1)
	// Storage: TechnicalCommittee Proposals (r:1 w:0)
	// Storage: DarwiniaEthereumRelay PendingRelayHeaderParcels (r:1 w:0)
	// Storage: TechnicalMembership Prime (r:1 w:0)
	// Storage: TechnicalCommittee Members (r:0 w:1)
	// Storage: TechnicalCommittee Prime (r:0 w:1)
	fn remove_member(m: u32, ) -> Weight {
		(26_039_000 as Weight)
			// Standard Error: 10_000
			.saturating_add((80_000 as Weight).saturating_mul(m as Weight))
			.saturating_add(T::DbWeight::get().reads(4 as Weight))
			.saturating_add(T::DbWeight::get().writes(3 as Weight))
	}
	// Storage: TechnicalMembership Members (r:1 w:1)
	// Storage: TechnicalCommittee Proposals (r:1 w:0)
	// Storage: DarwiniaEthereumRelay PendingRelayHeaderParcels (r:1 w:0)
	// Storage: TechnicalMembership Prime (r:1 w:0)
	// Storage: TechnicalCommittee Members (r:0 w:1)
	// Storage: TechnicalCommittee Prime (r:0 w:1)
	fn swap_member(m: u32, ) -> Weight {
		(25_757_000 as Weight)
			// Standard Error: 9_000
			.saturating_add((99_000 as Weight).saturating_mul(m as Weight))
			.saturating_add(T::DbWeight::get().reads(4 as Weight))
			.saturating_add(T::DbWeight::get().writes(3 as Weight))
	}
	// Storage: TechnicalMembership Members (r:1 w:1)
	// Storage: TechnicalCommittee Proposals (r:1 w:0)
	// Storage: DarwiniaEthereumRelay PendingRelayHeaderParcels (r:1 w:0)
	// Storage: TechnicalMembership Prime (r:1 w:0)
	// Storage: TechnicalCommittee Members (r:0 w:1)
	// Storage: TechnicalCommittee Prime (r:0 w:1)
	fn reset_member(m: u32, ) -> Weight {
		(28_088_000 as Weight)
			// Standard Error: 30_000
			.saturating_add((167_000 as Weight).saturating_mul(m as Weight))
			.saturating_add(T::DbWeight::get().reads(4 as Weight))
			.saturating_add(T::DbWeight::get().writes(3 as Weight))
	}
	// Storage: TechnicalMembership Members (r:1 w:1)
	// Storage: TechnicalCommittee Proposals (r:1 w:0)
	// Storage: DarwiniaEthereumRelay PendingRelayHeaderParcels (r:1 w:0)
	// Storage: TechnicalMembership Prime (r:1 w:1)
	// Storage: TechnicalCommittee Members (r:0 w:1)
	// Storage: TechnicalCommittee Prime (r:0 w:1)
	fn change_key(m: u32, ) -> Weight {
		(27_523_000 as Weight)
			// Standard Error: 9_000
			.saturating_add((86_000 as Weight).saturating_mul(m as Weight))
			.saturating_add(T::DbWeight::get().reads(4 as Weight))
			.saturating_add(T::DbWeight::get().writes(4 as Weight))
	}
	// Storage: TechnicalMembership Members (r:1 w:0)
	// Storage: TechnicalMembership Prime (r:0 w:1)
	// Storage: TechnicalCommittee Prime (r:0 w:1)
	fn set_prime(m: u32, ) -> Weight {
		(6_409_000 as Weight)
			// Standard Error: 3_000
			.saturating_add((65_000 as Weight).saturating_mul(m as Weight))
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	// Storage: TechnicalMembership Prime (r:0 w:1)
	// Storage: TechnicalCommittee Prime (r:0 w:1)
	fn clear_prime(_m: u32, ) -> Weight {
		(2_567_000 as Weight)
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
}
