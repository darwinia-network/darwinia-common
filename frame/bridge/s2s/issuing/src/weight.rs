// This file is part of Substrate.

// Copyright (C) 2020 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Autogenerated weights for darwinia_s2s_issuing
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 3.0.0
//! DATE: 2021-07-07, STEPS: [100, ], REPEAT: 10, LOW RANGE: [], HIGH RANGE: []
//! EXECUTION: Some(Native), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 128

// Executed Command:
// ./target/release/drml
// benchmark
// --chain
// dev
// --wasm-execution
// compiled
// --pallet
// darwinia_s2s_issuing
// --execution
// native
// --extrinsic=*
// --steps
// 100
// --repeat
// 10
// --raw
// --heap-pages=4096
// --output=./frame/bridge/s2s/issuing/src/weight.rs
// --template=./.maintain/frame-weight-template.hbs

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{
	traits::Get,
	weights::{constants::RocksDbWeight, Weight},
};
use sp_std::marker::PhantomData;

/// Weight functions needed for darwinia_s2s_issuing.
pub trait WeightInfo {
	fn asset_burn_event_handle() -> Weight;
}

/// Weights for darwinia_s2s_issuing using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	fn asset_burn_event_handle() -> Weight {
		(53_890_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(8 as Weight))
			.saturating_add(T::DbWeight::get().writes(4 as Weight))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	fn asset_burn_event_handle() -> Weight {
		(53_890_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(8 as Weight))
			.saturating_add(RocksDbWeight::get().writes(4 as Weight))
	}
}
