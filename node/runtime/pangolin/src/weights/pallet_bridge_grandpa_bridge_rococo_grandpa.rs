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

//! Autogenerated weights for `pallet_bridge_grandpa`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2022-07-19, STEPS: `2`, REPEAT: 2, LOW RANGE: `[]`, HIGH RANGE: `[]`
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
// pallet_bridge_grandpa


#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for `pallet_bridge_grandpa`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_bridge_grandpa::WeightInfo for WeightInfo<T> {
	// Storage: BridgeRococoGrandpa IsHalted (r:1 w:0)
	// Storage: BridgeRococoGrandpa RequestCount (r:1 w:1)
	// Storage: BridgeRococoGrandpa BestFinalized (r:1 w:1)
	// Storage: BridgeRococoGrandpa ImportedHeaders (r:1 w:2)
	// Storage: BridgeRococoGrandpa CurrentAuthoritySet (r:1 w:0)
	// Storage: BridgeRococoGrandpa ImportedHashesPointer (r:1 w:1)
	// Storage: BridgeRococoGrandpa ImportedHashes (r:1 w:1)
	fn submit_finality_proof(p: u32, v: u32, ) -> Weight {
		(1_357_422_000 as Weight)
			// Standard Error: 369_000
			.saturating_add((32_502_000 as Weight).saturating_mul(p as Weight))
			// Standard Error: 378_000
			.saturating_add((2_210_000 as Weight).saturating_mul(v as Weight))
			.saturating_add(T::DbWeight::get().reads(7 as Weight))
			.saturating_add(T::DbWeight::get().writes(6 as Weight))
	}
}
