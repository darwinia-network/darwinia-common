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

//! Autogenerated weights for `pallet_elections_phragmen`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2022-07-29, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
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
// 50
// --repeat
// 20
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

/// Weight functions for `pallet_elections_phragmen`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_elections_phragmen::WeightInfo for WeightInfo<T> {
	// Storage: PhragmenElection Candidates (r:1 w:0)
	// Storage: PhragmenElection Members (r:1 w:0)
	// Storage: PhragmenElection RunnersUp (r:1 w:0)
	// Storage: PhragmenElection Voting (r:1 w:1)
	// Storage: Balances Locks (r:1 w:1)
	fn vote_equal(_v: u32, ) -> Weight {
		(27_300_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(5 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	// Storage: PhragmenElection Candidates (r:1 w:0)
	// Storage: PhragmenElection Members (r:1 w:0)
	// Storage: PhragmenElection RunnersUp (r:1 w:0)
	// Storage: PhragmenElection Voting (r:1 w:1)
	// Storage: Balances Locks (r:1 w:1)
	// Storage: System Number (r:1 w:0)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Storage: System EventCount (r:1 w:1)
	// Storage: System Events (r:1 w:1)
	fn vote_more(v: u32, ) -> Weight {
		(33_977_000 as Weight)
			// Standard Error: 14_000
			.saturating_add((410_000 as Weight).saturating_mul(v as Weight))
			.saturating_add(T::DbWeight::get().reads(9 as Weight))
			.saturating_add(T::DbWeight::get().writes(4 as Weight))
	}
	// Storage: PhragmenElection Candidates (r:1 w:0)
	// Storage: PhragmenElection Members (r:1 w:0)
	// Storage: PhragmenElection RunnersUp (r:1 w:0)
	// Storage: PhragmenElection Voting (r:1 w:1)
	// Storage: System Number (r:1 w:0)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Storage: System EventCount (r:1 w:1)
	// Storage: System Events (r:1 w:1)
	// Storage: Balances Locks (r:1 w:1)
	fn vote_less(v: u32, ) -> Weight {
		(38_183_000 as Weight)
			// Standard Error: 96_000
			.saturating_add((57_000 as Weight).saturating_mul(v as Weight))
			.saturating_add(T::DbWeight::get().reads(9 as Weight))
			.saturating_add(T::DbWeight::get().writes(4 as Weight))
	}
	// Storage: PhragmenElection Voting (r:1 w:1)
	// Storage: Balances Locks (r:1 w:1)
	// Storage: System Number (r:1 w:0)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Storage: System EventCount (r:1 w:1)
	// Storage: System Events (r:1 w:1)
	fn remove_voter() -> Weight {
		(32_546_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(6 as Weight))
			.saturating_add(T::DbWeight::get().writes(4 as Weight))
	}
	// Storage: PhragmenElection Candidates (r:1 w:1)
	// Storage: PhragmenElection Members (r:1 w:0)
	// Storage: PhragmenElection RunnersUp (r:1 w:0)
	// Storage: Balances Locks (r:1 w:0)
	// Storage: System Number (r:1 w:0)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Storage: System EventCount (r:1 w:1)
	// Storage: System Events (r:1 w:1)
	fn submit_candidacy(c: u32, ) -> Weight {
		(36_191_000 as Weight)
			// Standard Error: 5_000
			.saturating_add((277_000 as Weight).saturating_mul(c as Weight))
			.saturating_add(T::DbWeight::get().reads(8 as Weight))
			.saturating_add(T::DbWeight::get().writes(3 as Weight))
	}
	// Storage: PhragmenElection Candidates (r:1 w:1)
	// Storage: System Number (r:1 w:0)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Storage: System EventCount (r:1 w:1)
	// Storage: System Events (r:1 w:1)
	fn renounce_candidacy_candidate(c: u32, ) -> Weight {
		(35_075_000 as Weight)
			// Standard Error: 3_000
			.saturating_add((147_000 as Weight).saturating_mul(c as Weight))
			.saturating_add(T::DbWeight::get().reads(5 as Weight))
			.saturating_add(T::DbWeight::get().writes(3 as Weight))
	}
	// Storage: PhragmenElection Members (r:1 w:1)
	// Storage: System Number (r:1 w:0)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Storage: System EventCount (r:1 w:1)
	// Storage: System Events (r:1 w:1)
	// Storage: PhragmenElection RunnersUp (r:1 w:1)
	// Storage: Council Prime (r:1 w:1)
	// Storage: Council Proposals (r:1 w:0)
	// Storage: Council Members (r:0 w:1)
	fn renounce_candidacy_members() -> Weight {
		(41_883_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(8 as Weight))
			.saturating_add(T::DbWeight::get().writes(6 as Weight))
	}
	// Storage: PhragmenElection RunnersUp (r:1 w:1)
	// Storage: System Number (r:1 w:0)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Storage: System EventCount (r:1 w:1)
	// Storage: System Events (r:1 w:1)
	fn renounce_candidacy_runners_up() -> Weight {
		(29_803_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(5 as Weight))
			.saturating_add(T::DbWeight::get().writes(3 as Weight))
	}
	// Storage: Benchmark Override (r:0 w:0)
	fn remove_member_without_replacement() -> Weight {
		(2_000_000_000_000 as Weight)
	}
	// Storage: PhragmenElection RunnersUp (r:1 w:1)
	// Storage: PhragmenElection Members (r:1 w:1)
	// Storage: System Account (r:2 w:2)
	// Storage: System Number (r:1 w:0)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Storage: System EventCount (r:1 w:1)
	// Storage: System Events (r:1 w:1)
	// Storage: Council Prime (r:1 w:1)
	// Storage: Council Proposals (r:1 w:0)
	// Storage: Council Members (r:0 w:1)
	fn remove_member_with_replacement() -> Weight {
		(61_022_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(10 as Weight))
			.saturating_add(T::DbWeight::get().writes(8 as Weight))
	}
	// Storage: PhragmenElection RunnersUp (r:1 w:0)
	fn remove_member_wrong_refund() -> Weight {
		(4_069_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
	}
	// Storage: PhragmenElection Voting (r:251 w:250)
	// Storage: PhragmenElection Members (r:1 w:0)
	// Storage: PhragmenElection RunnersUp (r:1 w:0)
	// Storage: PhragmenElection Candidates (r:1 w:0)
	// Storage: Balances Locks (r:250 w:250)
	// Storage: System Account (r:250 w:250)
	// Storage: System Number (r:1 w:0)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Storage: System EventCount (r:1 w:1)
	// Storage: System Events (r:1 w:1)
	fn clean_defunct_voters(v: u32, _d: u32, ) -> Weight {
		(0 as Weight)
			// Standard Error: 184_000
			.saturating_add((61_049_000 as Weight).saturating_mul(v as Weight))
			.saturating_add(T::DbWeight::get().reads(8 as Weight))
			.saturating_add(T::DbWeight::get().reads((3 as Weight).saturating_mul(v as Weight)))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
			.saturating_add(T::DbWeight::get().writes((3 as Weight).saturating_mul(v as Weight)))
	}
	// Storage: PhragmenElection Candidates (r:1 w:1)
	// Storage: PhragmenElection Members (r:1 w:1)
	// Storage: PhragmenElection RunnersUp (r:1 w:1)
	// Storage: Balances TotalIssuance (r:1 w:0)
	// Storage: PhragmenElection Voting (r:502 w:0)
	// Storage: Council Proposals (r:1 w:0)
	// Storage: System Number (r:1 w:0)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Storage: System EventCount (r:1 w:1)
	// Storage: System Events (r:1 w:1)
	// Storage: PhragmenElection ElectionRounds (r:1 w:1)
	// Storage: Council Members (r:0 w:1)
	// Storage: Council Prime (r:0 w:1)
	// Storage: System Account (r:3 w:3)
	fn election_phragmen(c: u32, v: u32, e: u32, ) -> Weight {
		(0 as Weight)
			// Standard Error: 382_000
			.saturating_add((35_978_000 as Weight).saturating_mul(c as Weight))
			// Standard Error: 159_000
			.saturating_add((19_381_000 as Weight).saturating_mul(v as Weight))
			// Standard Error: 10_000
			.saturating_add((876_000 as Weight).saturating_mul(e as Weight))
			.saturating_add(T::DbWeight::get().reads((2 as Weight).saturating_mul(c as Weight)))
			.saturating_add(T::DbWeight::get().reads((1 as Weight).saturating_mul(v as Weight)))
			.saturating_add(T::DbWeight::get().writes((1 as Weight).saturating_mul(c as Weight)))
	}
}
