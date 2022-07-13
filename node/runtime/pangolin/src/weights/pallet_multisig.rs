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

//! Autogenerated weights for `pallet_multisig`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2022-07-13, STEPS: `2`, REPEAT: 2, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("pangolin-dev"), DB CACHE: 128

// Executed Command:
// ./target/release/drml
// benchmark
// --header
// .maintain/lincense-header
// --execution
// wasm
// --heap-pages
// 4096
// --steps
// 2
// --repeat
// 2
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

/// Weight functions for `pallet_multisig`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_multisig::WeightInfo for WeightInfo<T> {
	// Storage: TransactionPause PausedTransactions (r:1 w:0)
	fn as_multi_threshold_1(_z: u32, ) -> Weight {
		(19_242_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
	}
	// Storage: Multisig Multisigs (r:1 w:1)
	// Storage: Balances Locks (r:1 w:0)
	// Storage: System Number (r:1 w:0)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Storage: System EventCount (r:1 w:1)
	// Storage: System Events (r:1 w:1)
	// Storage: unknown [0x3a65787472696e7369635f696e646578] (r:1 w:0)
	fn as_multi_create(s: u32, z: u32, ) -> Weight {
		(36_411_000 as Weight)
			// Standard Error: 17_000
			.saturating_add((143_000 as Weight).saturating_mul(s as Weight))
			// Standard Error: 0
			.saturating_add((1_000 as Weight).saturating_mul(z as Weight))
			.saturating_add(T::DbWeight::get().reads(7 as Weight))
			.saturating_add(T::DbWeight::get().writes(3 as Weight))
	}
	// Storage: Multisig Multisigs (r:1 w:1)
	// Storage: Multisig Calls (r:1 w:1)
	// Storage: Balances Locks (r:1 w:0)
	// Storage: System Number (r:1 w:0)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Storage: System EventCount (r:1 w:1)
	// Storage: System Events (r:1 w:1)
	// Storage: unknown [0x3a65787472696e7369635f696e646578] (r:1 w:0)
	fn as_multi_create_store(s: u32, z: u32, ) -> Weight {
		(37_799_000 as Weight)
			// Standard Error: 16_000
			.saturating_add((168_000 as Weight).saturating_mul(s as Weight))
			// Standard Error: 0
			.saturating_add((1_000 as Weight).saturating_mul(z as Weight))
			.saturating_add(T::DbWeight::get().reads(8 as Weight))
			.saturating_add(T::DbWeight::get().writes(4 as Weight))
	}
	// Storage: Multisig Multisigs (r:1 w:1)
	// Storage: System Number (r:1 w:0)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Storage: System EventCount (r:1 w:1)
	// Storage: System Events (r:1 w:1)
	fn as_multi_approve(s: u32, z: u32, ) -> Weight {
		(22_116_000 as Weight)
			// Standard Error: 13_000
			.saturating_add((143_000 as Weight).saturating_mul(s as Weight))
			// Standard Error: 0
			.saturating_add((1_000 as Weight).saturating_mul(z as Weight))
			.saturating_add(T::DbWeight::get().reads(5 as Weight))
			.saturating_add(T::DbWeight::get().writes(3 as Weight))
	}
	// Storage: Multisig Multisigs (r:1 w:1)
	// Storage: Multisig Calls (r:1 w:1)
	// Storage: Balances Locks (r:1 w:0)
	// Storage: System Number (r:1 w:0)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Storage: System EventCount (r:1 w:1)
	// Storage: System Events (r:1 w:1)
	fn as_multi_approve_store(s: u32, z: u32, ) -> Weight {
		(36_564_000 as Weight)
			// Standard Error: 24_000
			.saturating_add((167_000 as Weight).saturating_mul(s as Weight))
			// Standard Error: 0
			.saturating_add((1_000 as Weight).saturating_mul(z as Weight))
			.saturating_add(T::DbWeight::get().reads(7 as Weight))
			.saturating_add(T::DbWeight::get().writes(4 as Weight))
	}
	// Storage: Multisig Multisigs (r:1 w:1)
	// Storage: Multisig Calls (r:1 w:1)
	// Storage: System Account (r:1 w:1)
	// Storage: System Number (r:1 w:0)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Storage: System EventCount (r:1 w:1)
	// Storage: System Events (r:1 w:1)
	// Storage: TransactionPause PausedTransactions (r:1 w:0)
	fn as_multi_complete(s: u32, z: u32, ) -> Weight {
		(51_173_000 as Weight)
			// Standard Error: 53_000
			.saturating_add((240_000 as Weight).saturating_mul(s as Weight))
			// Standard Error: 0
			.saturating_add((1_000 as Weight).saturating_mul(z as Weight))
			.saturating_add(T::DbWeight::get().reads(8 as Weight))
			.saturating_add(T::DbWeight::get().writes(5 as Weight))
	}
	// Storage: Multisig Multisigs (r:1 w:1)
	// Storage: Balances Locks (r:1 w:0)
	// Storage: System Number (r:1 w:0)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Storage: System EventCount (r:1 w:1)
	// Storage: System Events (r:1 w:1)
	// Storage: unknown [0x3a65787472696e7369635f696e646578] (r:1 w:0)
	fn approve_as_multi_create(s: u32, ) -> Weight {
		(31_836_000 as Weight)
			// Standard Error: 22_000
			.saturating_add((217_000 as Weight).saturating_mul(s as Weight))
			.saturating_add(T::DbWeight::get().reads(7 as Weight))
			.saturating_add(T::DbWeight::get().writes(3 as Weight))
	}
	// Storage: Multisig Multisigs (r:1 w:1)
	// Storage: Multisig Calls (r:1 w:0)
	// Storage: System Number (r:1 w:0)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Storage: System EventCount (r:1 w:1)
	// Storage: System Events (r:1 w:1)
	fn approve_as_multi_approve(s: u32, ) -> Weight {
		(22_198_000 as Weight)
			// Standard Error: 52_000
			.saturating_add((136_000 as Weight).saturating_mul(s as Weight))
			.saturating_add(T::DbWeight::get().reads(6 as Weight))
			.saturating_add(T::DbWeight::get().writes(3 as Weight))
	}
	// Storage: Multisig Multisigs (r:1 w:1)
	// Storage: Multisig Calls (r:1 w:1)
	// Storage: System Account (r:1 w:1)
	// Storage: System Number (r:1 w:0)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Storage: System EventCount (r:1 w:1)
	// Storage: System Events (r:1 w:1)
	// Storage: TransactionPause PausedTransactions (r:1 w:0)
	fn approve_as_multi_complete(s: u32, ) -> Weight {
		(63_180_000 as Weight)
			// Standard Error: 56_000
			.saturating_add((243_000 as Weight).saturating_mul(s as Weight))
			.saturating_add(T::DbWeight::get().reads(8 as Weight))
			.saturating_add(T::DbWeight::get().writes(5 as Weight))
	}
	// Storage: Multisig Multisigs (r:1 w:1)
	// Storage: System Number (r:1 w:0)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Storage: System EventCount (r:1 w:1)
	// Storage: System Events (r:1 w:1)
	// Storage: Multisig Calls (r:1 w:1)
	fn cancel_as_multi(s: u32, ) -> Weight {
		(46_372_000 as Weight)
			// Standard Error: 9_000
			.saturating_add((148_000 as Weight).saturating_mul(s as Weight))
			.saturating_add(T::DbWeight::get().reads(6 as Weight))
			.saturating_add(T::DbWeight::get().writes(4 as Weight))
	}
}
