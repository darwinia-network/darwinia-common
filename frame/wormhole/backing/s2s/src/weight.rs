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

//! Autogenerated weights for to_substrate_backing
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 3.0.0
//! DATE: 2021-09-24, STEPS: [100, ], REPEAT: 10, LOW RANGE: [], HIGH RANGE: []
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 128

// Executed Command:
// ./target/release/drml
// benchmark
// --chain
// dev
// --wasm-execution
// compiled
// --pallet
// to_substrate_backing
// --execution
// wasm
// --extrinsic=*
// --steps
// 100
// --repeat
// 10
// --raw
// --heap-pages=4096
// --output=./frame/wormhole/backing/s2s/src/weight.rs
// --template=./.maintain/frame-weight-template.hbs

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{
	traits::Get,
	weights::{constants::RocksDbWeight, Weight},
};
use sp_std::marker::PhantomData;

/// Weight functions needed for to_substrate_backing.
pub trait WeightInfo {
	fn register_and_remote_create() -> Weight;
	fn lock_and_remote_issue() -> Weight;
	fn unlock_from_remote() -> Weight;
	fn set_secure_limited_period() -> Weight;
	fn set_security_limitation_ring_amount() -> Weight;
	fn set_remote_mapping_token_factory_account() -> Weight;
	fn set_backing_contract_address() -> Weight;
}

/// Weights for to_substrate_backing using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	fn register_and_remote_create() -> Weight {
		(210_170_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(11 as Weight))
			.saturating_add(T::DbWeight::get().writes(6 as Weight))
	}
	fn lock_and_remote_issue() -> Weight {
		(271_083_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(12 as Weight))
			.saturating_add(T::DbWeight::get().writes(7 as Weight))
	}
	fn unlock_from_remote() -> Weight {
		(115_133_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(7 as Weight))
			.saturating_add(T::DbWeight::get().writes(4 as Weight))
	}
	fn set_secure_limited_period() -> Weight {
		(4_000_000 as Weight).saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	fn set_security_limitation_ring_amount() -> Weight {
		(4_000_000 as Weight).saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	fn set_remote_mapping_token_factory_account() -> Weight {
		(4_000_000 as Weight).saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	fn set_backing_contract_address() -> Weight {
		(4_000_000 as Weight).saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	fn register_and_remote_create() -> Weight {
		(210_170_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(11 as Weight))
			.saturating_add(RocksDbWeight::get().writes(6 as Weight))
	}
	fn lock_and_remote_issue() -> Weight {
		(271_083_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(12 as Weight))
			.saturating_add(RocksDbWeight::get().writes(7 as Weight))
	}
	fn unlock_from_remote() -> Weight {
		(115_133_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(7 as Weight))
			.saturating_add(RocksDbWeight::get().writes(4 as Weight))
	}
	fn set_secure_limited_period() -> Weight {
		(4_000_000 as Weight).saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	fn set_security_limitation_ring_amount() -> Weight {
		(4_000_000 as Weight).saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	fn set_remote_mapping_token_factory_account() -> Weight {
		(4_000_000 as Weight).saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	fn set_backing_contract_address() -> Weight {
		(4_000_000 as Weight).saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
}
