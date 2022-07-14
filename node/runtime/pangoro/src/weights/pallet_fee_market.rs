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

//! Autogenerated weights for `pallet_fee_market`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2022-07-13, STEPS: `2`, REPEAT: 2, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("pangoro-dev"), DB CACHE: 128

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
// pangoro-dev
// --output
// node/runtime/pangoro/src/weights/
// --extrinsic
// *
// --pallet
// *


#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for `pallet_fee_market`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_fee_market::WeightInfo for WeightInfo<T> {
	// Storage: PangolinFeeMarket Relayers (r:1 w:1)
	// Storage: System Account (r:1 w:1)
	// Storage: Balances Locks (r:1 w:1)
	// Storage: PangolinFeeMarket RelayersMap (r:4 w:1)
	// Storage: PangolinFeeMarket Orders (r:1 w:0)
	// Storage: PangolinFeeMarket AssignedRelayersNumber (r:1 w:0)
	// Storage: System Number (r:1 w:0)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Storage: System EventCount (r:1 w:1)
	// Storage: System Events (r:1 w:1)
	// Storage: PangolinFeeMarket AssignedRelayers (r:0 w:1)
	fn enroll_and_lock_collateral() -> Weight {
		(65_769_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(13 as Weight))
			.saturating_add(T::DbWeight::get().writes(7 as Weight))
	}
	// Storage: PangolinFeeMarket Relayers (r:1 w:0)
	// Storage: System Account (r:1 w:0)
	// Storage: PangolinFeeMarket RelayersMap (r:4 w:1)
	// Storage: Balances Locks (r:1 w:1)
	// Storage: PangolinFeeMarket Orders (r:1 w:0)
	// Storage: PangolinFeeMarket AssignedRelayersNumber (r:1 w:0)
	// Storage: System Number (r:1 w:0)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Storage: System EventCount (r:1 w:1)
	// Storage: System Events (r:1 w:1)
	// Storage: PangolinFeeMarket AssignedRelayers (r:0 w:1)
	fn update_locked_collateral() -> Weight {
		(58_800_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(13 as Weight))
			.saturating_add(T::DbWeight::get().writes(5 as Weight))
	}
	// Storage: PangolinFeeMarket Relayers (r:1 w:0)
	// Storage: PangolinFeeMarket RelayersMap (r:4 w:1)
	// Storage: PangolinFeeMarket Orders (r:1 w:0)
	// Storage: PangolinFeeMarket AssignedRelayersNumber (r:1 w:0)
	// Storage: System Number (r:1 w:0)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Storage: System EventCount (r:1 w:1)
	// Storage: System Events (r:1 w:1)
	// Storage: PangolinFeeMarket AssignedRelayers (r:0 w:1)
	fn update_relay_fee() -> Weight {
		(48_448_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(11 as Weight))
			.saturating_add(T::DbWeight::get().writes(4 as Weight))
	}
	// Storage: PangolinFeeMarket Relayers (r:1 w:1)
	// Storage: PangolinFeeMarket Orders (r:1 w:0)
	// Storage: Balances Locks (r:1 w:1)
	// Storage: System Account (r:1 w:1)
	// Storage: PangolinFeeMarket AssignedRelayers (r:1 w:1)
	// Storage: PangolinFeeMarket RelayersMap (r:3 w:1)
	// Storage: PangolinFeeMarket AssignedRelayersNumber (r:1 w:0)
	// Storage: System Number (r:1 w:0)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Storage: System EventCount (r:1 w:1)
	// Storage: System Events (r:1 w:1)
	fn cancel_enrollment() -> Weight {
		(61_317_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(13 as Weight))
			.saturating_add(T::DbWeight::get().writes(7 as Weight))
	}
	// Storage: System Number (r:1 w:0)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Storage: System EventCount (r:1 w:1)
	// Storage: System Events (r:1 w:1)
	// Storage: PangolinFeeMarket CollateralSlashProtect (r:0 w:1)
	fn set_slash_protect() -> Weight {
		(13_405_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(4 as Weight))
			.saturating_add(T::DbWeight::get().writes(3 as Weight))
	}
	// Storage: PangolinFeeMarket Relayers (r:1 w:0)
	// Storage: PangolinFeeMarket RelayersMap (r:4 w:0)
	// Storage: PangolinFeeMarket Orders (r:1 w:0)
	// Storage: System Number (r:1 w:0)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Storage: System EventCount (r:1 w:1)
	// Storage: System Events (r:1 w:1)
	// Storage: PangolinFeeMarket AssignedRelayers (r:0 w:1)
	// Storage: PangolinFeeMarket AssignedRelayersNumber (r:0 w:1)
	fn set_assigned_relayers_number() -> Weight {
		(44_182_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(10 as Weight))
			.saturating_add(T::DbWeight::get().writes(4 as Weight))
	}
}