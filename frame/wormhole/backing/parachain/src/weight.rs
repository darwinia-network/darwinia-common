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

//! Autogenerated weights for to_parachain_backing
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2022-05-31, STEPS: `100`, REPEAT: 10, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 128

// Executed Command:
// ./target/release/drml
// benchmark
// --chain
// dev
// --wasm-execution
// compiled
// --pallet
// to_parachain_backing
// --execution
// wasm
// --extrinsic=*
// --steps
// 100
// --repeat
// 10
// --raw
// --heap-pages=4096
// --output=./frame/wormhole/backing/parachain/src/weight.rs
// --template=./.maintain/frame-weight-template.hbs


#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for to_parachain_backing.
pub trait WeightInfo {
    fn lock_and_remote_issue() -> Weight;
	fn unlock_from_remote() -> Weight;
	fn set_secure_limited_period() -> Weight;
	fn set_security_limitation_ring_amount() -> Weight;
	fn set_remote_mapping_token_factory_account() -> Weight;
}

/// Weights for to_parachain_backing using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	fn lock_and_remote_issue() -> Weight {
		(271_083_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(12 as Weight))
			.saturating_add(T::DbWeight::get().writes(7 as Weight))
	}
	// Storage: ToPangolinParachainBacking RemoteMappingTokenFactoryAccount (r:1 w:0)
	// Storage: ToPangolinParachainBacking SecureLimitedRingAmount (r:1 w:1)
	// Storage: ToPangolinParachainBacking SecureLimitedPeriod (r:1 w:0)
	// Storage: System Account (r:2 w:2)
	// Storage: Balances Locks (r:1 w:0)
	// Storage: System Number (r:1 w:0)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Storage: System EventCount (r:1 w:1)
	// Storage: System Events (r:1 w:1)
	// Storage: BridgePangolinParachainMessages InboundLanes (r:1 w:0)
	fn unlock_from_remote() -> Weight {
		(215_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(11 as Weight))
			.saturating_add(T::DbWeight::get().writes(5 as Weight))
	}
	// Storage: ToPangolinParachainBacking SecureLimitedPeriod (r:0 w:1)
	fn set_secure_limited_period() -> Weight {
		(4_000_000 as Weight)
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	// Storage: ToPangolinParachainBacking SecureLimitedRingAmount (r:1 w:1)
	fn set_security_limitation_ring_amount() -> Weight {
		(13_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	// Storage: System Number (r:1 w:0)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Storage: System EventCount (r:1 w:1)
	// Storage: System Events (r:1 w:1)
	// Storage: ToPangolinParachainBacking RemoteMappingTokenFactoryAccount (r:0 w:1)
	fn set_remote_mapping_token_factory_account() -> Weight {
		(37_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(4 as Weight))
			.saturating_add(T::DbWeight::get().writes(3 as Weight))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	fn lock_and_remote_issue() -> Weight {
		(271_083_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(12 as Weight))
			.saturating_add(RocksDbWeight::get().writes(7 as Weight))
	}
	// Storage: ToPangolinParachainBacking RemoteMappingTokenFactoryAccount (r:1 w:0)
	// Storage: ToPangolinParachainBacking SecureLimitedRingAmount (r:1 w:1)
	// Storage: ToPangolinParachainBacking SecureLimitedPeriod (r:1 w:0)
	// Storage: System Account (r:2 w:2)
	// Storage: Balances Locks (r:1 w:0)
	// Storage: System Number (r:1 w:0)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Storage: System EventCount (r:1 w:1)
	// Storage: System Events (r:1 w:1)
	// Storage: BridgePangolinParachainMessages InboundLanes (r:1 w:0)
	fn unlock_from_remote() -> Weight {
		(215_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(11 as Weight))
			.saturating_add(RocksDbWeight::get().writes(5 as Weight))
	}
	// Storage: ToPangolinParachainBacking SecureLimitedPeriod (r:0 w:1)
	fn set_secure_limited_period() -> Weight {
		(4_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	// Storage: ToPangolinParachainBacking SecureLimitedRingAmount (r:1 w:1)
	fn set_security_limitation_ring_amount() -> Weight {
		(13_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	// Storage: System Number (r:1 w:0)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Storage: System EventCount (r:1 w:1)
	// Storage: System Events (r:1 w:1)
	// Storage: ToPangolinParachainBacking RemoteMappingTokenFactoryAccount (r:0 w:1)
	fn set_remote_mapping_token_factory_account() -> Weight {
		(37_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(4 as Weight))
			.saturating_add(RocksDbWeight::get().writes(3 as Weight))
	}
}
