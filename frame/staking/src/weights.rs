//! Autogenerated weights for pallet_staking
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 3.0.0
//! DATE: 2021-06-15, STEPS: `[50, ]`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 128

// Executed Command:
// target/release/substrate
// benchmark
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet_staking
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output=./frame/staking/src/weights.rs
// --template=./.maintain/frame-weight-template.hbs

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{
	traits::Get,
	weights::{constants::RocksDbWeight, Weight},
};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_staking.
pub trait WeightInfo {
	fn bond() -> Weight;
	fn bond_extra() -> Weight;
	fn deposit_extra() -> Weight;
	fn unbond() -> Weight;
	fn claim_mature_deposits() -> Weight;
	fn try_claim_deposits_with_punish() -> Weight;
	fn validate() -> Weight;
	fn kick(k: u32) -> Weight;
	fn nominate(n: u32) -> Weight;
	fn chill() -> Weight;
	fn set_payee() -> Weight;
	fn set_controller() -> Weight;
	fn set_validator_count() -> Weight;
	fn force_no_eras() -> Weight;
	fn force_new_era() -> Weight;
	fn force_new_era_always() -> Weight;
	fn set_invulnerables(v: u32) -> Weight;
	fn force_unstake(s: u32) -> Weight;
	fn cancel_deferred_slash(s: u32) -> Weight;
	fn payout_stakers_dead_controller(n: u32) -> Weight;
	fn payout_stakers_alive_staked(n: u32) -> Weight;
	fn rebond(l: u32) -> Weight;
	fn set_history_depth(e: u32) -> Weight;
	fn reap_stash(s: u32) -> Weight;
	fn new_era(v: u32, n: u32) -> Weight;
	fn get_npos_voters(v: u32, n: u32, s: u32) -> Weight;
	fn get_npos_targets(v: u32) -> Weight;
	fn set_staking_limits() -> Weight;
	fn chill_other() -> Weight;
}

/// Weights for pallet_staking using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	fn bond() -> Weight {
		(91_278_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(5 as Weight))
			.saturating_add(T::DbWeight::get().writes(4 as Weight))
	}

	fn bond_extra() -> Weight {
		(69_833_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}

	fn deposit_extra() -> Weight {
		(69_833_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}

	fn claim_mature_deposits() -> Weight {
		(69_833_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}

	fn try_claim_deposits_with_punish() -> Weight {
		(69_833_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}

	fn unbond() -> Weight {
		(75_020_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(6 as Weight))
			.saturating_add(T::DbWeight::get().writes(3 as Weight))
	}

	fn validate() -> Weight {
		(40_702_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(6 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}

	fn kick(k: u32) -> Weight {
		(33_572_000 as Weight)
			// Standard Error: 18_000
			.saturating_add((20_771_000 as Weight).saturating_mul(k as Weight))
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().reads((1 as Weight).saturating_mul(k as Weight)))
			.saturating_add(T::DbWeight::get().writes((1 as Weight).saturating_mul(k as Weight)))
	}

	fn nominate(n: u32) -> Weight {
		(53_561_000 as Weight)
			// Standard Error: 34_000
			.saturating_add((6_652_000 as Weight).saturating_mul(n as Weight))
			.saturating_add(T::DbWeight::get().reads(7 as Weight))
			.saturating_add(T::DbWeight::get().reads((1 as Weight).saturating_mul(n as Weight)))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}

	fn chill() -> Weight {
		(21_489_000 as Weight).saturating_add(T::DbWeight::get().reads(3 as Weight))
	}

	fn set_payee() -> Weight {
		(14_514_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}

	fn set_controller() -> Weight {
		(32_598_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().writes(3 as Weight))
	}

	fn set_validator_count() -> Weight {
		(2_477_000 as Weight).saturating_add(T::DbWeight::get().writes(1 as Weight))
	}

	fn force_no_eras() -> Weight {
		(2_743_000 as Weight).saturating_add(T::DbWeight::get().writes(1 as Weight))
	}

	fn force_new_era() -> Weight {
		(2_784_000 as Weight).saturating_add(T::DbWeight::get().writes(1 as Weight))
	}

	fn force_new_era_always() -> Weight {
		(2_749_000 as Weight).saturating_add(T::DbWeight::get().writes(1 as Weight))
	}

	fn set_invulnerables(v: u32) -> Weight {
		(2_798_000 as Weight)
			// Standard Error: 0
			.saturating_add((5_000 as Weight).saturating_mul(v as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}

	fn force_unstake(s: u32) -> Weight {
		(70_372_000 as Weight)
			// Standard Error: 13_000
			.saturating_add((3_029_000 as Weight).saturating_mul(s as Weight))
			.saturating_add(T::DbWeight::get().reads(6 as Weight))
			.saturating_add(T::DbWeight::get().writes(6 as Weight))
			.saturating_add(T::DbWeight::get().writes((1 as Weight).saturating_mul(s as Weight)))
	}

	fn cancel_deferred_slash(s: u32) -> Weight {
		(3_436_822_000 as Weight)
			// Standard Error: 221_000
			.saturating_add((19_799_000 as Weight).saturating_mul(s as Weight))
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}

	fn payout_stakers_dead_controller(n: u32) -> Weight {
		(132_018_000 as Weight)
			// Standard Error: 27_000
			.saturating_add((61_340_000 as Weight).saturating_mul(n as Weight))
			.saturating_add(T::DbWeight::get().reads(10 as Weight))
			.saturating_add(T::DbWeight::get().reads((3 as Weight).saturating_mul(n as Weight)))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
			.saturating_add(T::DbWeight::get().writes((1 as Weight).saturating_mul(n as Weight)))
	}

	fn payout_stakers_alive_staked(n: u32) -> Weight {
		(158_346_000 as Weight)
			// Standard Error: 61_000
			.saturating_add((77_147_000 as Weight).saturating_mul(n as Weight))
			.saturating_add(T::DbWeight::get().reads(11 as Weight))
			.saturating_add(T::DbWeight::get().reads((5 as Weight).saturating_mul(n as Weight)))
			.saturating_add(T::DbWeight::get().writes(3 as Weight))
			.saturating_add(T::DbWeight::get().writes((3 as Weight).saturating_mul(n as Weight)))
	}

	fn rebond(l: u32) -> Weight {
		(57_756_000 as Weight)
			// Standard Error: 2_000
			.saturating_add((79_000 as Weight).saturating_mul(l as Weight))
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().writes(3 as Weight))
	}

	fn set_history_depth(e: u32) -> Weight {
		(0 as Weight)
			// Standard Error: 100_000
			.saturating_add((44_873_000 as Weight).saturating_mul(e as Weight))
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(4 as Weight))
			.saturating_add(T::DbWeight::get().writes((7 as Weight).saturating_mul(e as Weight)))
	}

	fn reap_stash(s: u32) -> Weight {
		(75_073_000 as Weight)
			// Standard Error: 4_000
			.saturating_add((2_988_000 as Weight).saturating_mul(s as Weight))
			.saturating_add(T::DbWeight::get().reads(6 as Weight))
			.saturating_add(T::DbWeight::get().writes(6 as Weight))
			.saturating_add(T::DbWeight::get().writes((1 as Weight).saturating_mul(s as Weight)))
	}

	fn new_era(v: u32, n: u32) -> Weight {
		(0 as Weight)
			// Standard Error: 1_146_000
			.saturating_add((362_986_000 as Weight).saturating_mul(v as Weight))
			// Standard Error: 57_000
			.saturating_add((60_216_000 as Weight).saturating_mul(n as Weight))
			.saturating_add(T::DbWeight::get().reads(10 as Weight))
			.saturating_add(T::DbWeight::get().reads((3 as Weight).saturating_mul(v as Weight)))
			.saturating_add(T::DbWeight::get().reads((3 as Weight).saturating_mul(n as Weight)))
			.saturating_add(T::DbWeight::get().writes(4 as Weight))
			.saturating_add(T::DbWeight::get().writes((3 as Weight).saturating_mul(v as Weight)))
	}

	fn get_npos_voters(v: u32, n: u32, s: u32) -> Weight {
		(0 as Weight)
			// Standard Error: 230_000
			.saturating_add((35_891_000 as Weight).saturating_mul(v as Weight))
			// Standard Error: 230_000
			.saturating_add((37_854_000 as Weight).saturating_mul(n as Weight))
			// Standard Error: 7_842_000
			.saturating_add((32_492_000 as Weight).saturating_mul(s as Weight))
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().reads((3 as Weight).saturating_mul(v as Weight)))
			.saturating_add(T::DbWeight::get().reads((3 as Weight).saturating_mul(n as Weight)))
			.saturating_add(T::DbWeight::get().reads((1 as Weight).saturating_mul(s as Weight)))
	}

	fn get_npos_targets(v: u32) -> Weight {
		(0 as Weight)
			// Standard Error: 74_000
			.saturating_add((16_370_000 as Weight).saturating_mul(v as Weight))
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().reads((1 as Weight).saturating_mul(v as Weight)))
	}

	fn set_staking_limits() -> Weight {
		(6_398_000 as Weight).saturating_add(T::DbWeight::get().writes(4 as Weight))
	}

	fn chill_other() -> Weight {
		(44_694_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(5 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	fn bond() -> Weight {
		(91_278_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(5 as Weight))
			.saturating_add(RocksDbWeight::get().writes(4 as Weight))
	}

	fn bond_extra() -> Weight {
		(69_833_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(3 as Weight))
			.saturating_add(RocksDbWeight::get().writes(2 as Weight))
	}

	fn deposit_extra() -> Weight {
		(69_833_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(3 as Weight))
			.saturating_add(RocksDbWeight::get().writes(2 as Weight))
	}

	fn claim_mature_deposits() -> Weight {
		(69_833_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(3 as Weight))
			.saturating_add(RocksDbWeight::get().writes(2 as Weight))
	}

	fn try_claim_deposits_with_punish() -> Weight {
		(69_833_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(3 as Weight))
			.saturating_add(RocksDbWeight::get().writes(2 as Weight))
	}

	fn unbond() -> Weight {
		(75_020_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(6 as Weight))
			.saturating_add(RocksDbWeight::get().writes(3 as Weight))
	}

	fn validate() -> Weight {
		(40_702_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(6 as Weight))
			.saturating_add(RocksDbWeight::get().writes(2 as Weight))
	}

	fn kick(k: u32) -> Weight {
		(33_572_000 as Weight)
			// Standard Error: 18_000
			.saturating_add((20_771_000 as Weight).saturating_mul(k as Weight))
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			.saturating_add(RocksDbWeight::get().reads((1 as Weight).saturating_mul(k as Weight)))
			.saturating_add(RocksDbWeight::get().writes((1 as Weight).saturating_mul(k as Weight)))
	}

	fn nominate(n: u32) -> Weight {
		(53_561_000 as Weight)
			// Standard Error: 34_000
			.saturating_add((6_652_000 as Weight).saturating_mul(n as Weight))
			.saturating_add(RocksDbWeight::get().reads(7 as Weight))
			.saturating_add(RocksDbWeight::get().reads((1 as Weight).saturating_mul(n as Weight)))
			.saturating_add(RocksDbWeight::get().writes(2 as Weight))
	}

	fn chill() -> Weight {
		(21_489_000 as Weight).saturating_add(RocksDbWeight::get().reads(3 as Weight))
	}

	fn set_payee() -> Weight {
		(14_514_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}

	fn set_controller() -> Weight {
		(32_598_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(3 as Weight))
			.saturating_add(RocksDbWeight::get().writes(3 as Weight))
	}

	fn set_validator_count() -> Weight {
		(2_477_000 as Weight).saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}

	fn force_no_eras() -> Weight {
		(2_743_000 as Weight).saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}

	fn force_new_era() -> Weight {
		(2_784_000 as Weight).saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}

	fn force_new_era_always() -> Weight {
		(2_749_000 as Weight).saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}

	fn set_invulnerables(v: u32) -> Weight {
		(2_798_000 as Weight)
			// Standard Error: 0
			.saturating_add((5_000 as Weight).saturating_mul(v as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}

	fn force_unstake(s: u32) -> Weight {
		(70_372_000 as Weight)
			// Standard Error: 13_000
			.saturating_add((3_029_000 as Weight).saturating_mul(s as Weight))
			.saturating_add(RocksDbWeight::get().reads(6 as Weight))
			.saturating_add(RocksDbWeight::get().writes(6 as Weight))
			.saturating_add(RocksDbWeight::get().writes((1 as Weight).saturating_mul(s as Weight)))
	}

	fn cancel_deferred_slash(s: u32) -> Weight {
		(3_436_822_000 as Weight)
			// Standard Error: 221_000
			.saturating_add((19_799_000 as Weight).saturating_mul(s as Weight))
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}

	fn payout_stakers_dead_controller(n: u32) -> Weight {
		(132_018_000 as Weight)
			// Standard Error: 27_000
			.saturating_add((61_340_000 as Weight).saturating_mul(n as Weight))
			.saturating_add(RocksDbWeight::get().reads(10 as Weight))
			.saturating_add(RocksDbWeight::get().reads((3 as Weight).saturating_mul(n as Weight)))
			.saturating_add(RocksDbWeight::get().writes(2 as Weight))
			.saturating_add(RocksDbWeight::get().writes((1 as Weight).saturating_mul(n as Weight)))
	}

	fn payout_stakers_alive_staked(n: u32) -> Weight {
		(158_346_000 as Weight)
			// Standard Error: 61_000
			.saturating_add((77_147_000 as Weight).saturating_mul(n as Weight))
			.saturating_add(RocksDbWeight::get().reads(11 as Weight))
			.saturating_add(RocksDbWeight::get().reads((5 as Weight).saturating_mul(n as Weight)))
			.saturating_add(RocksDbWeight::get().writes(3 as Weight))
			.saturating_add(RocksDbWeight::get().writes((3 as Weight).saturating_mul(n as Weight)))
	}

	fn rebond(l: u32) -> Weight {
		(57_756_000 as Weight)
			// Standard Error: 2_000
			.saturating_add((79_000 as Weight).saturating_mul(l as Weight))
			.saturating_add(RocksDbWeight::get().reads(3 as Weight))
			.saturating_add(RocksDbWeight::get().writes(3 as Weight))
	}

	fn set_history_depth(e: u32) -> Weight {
		(0 as Weight)
			// Standard Error: 100_000
			.saturating_add((44_873_000 as Weight).saturating_mul(e as Weight))
			.saturating_add(RocksDbWeight::get().reads(2 as Weight))
			.saturating_add(RocksDbWeight::get().writes(4 as Weight))
			.saturating_add(RocksDbWeight::get().writes((7 as Weight).saturating_mul(e as Weight)))
	}

	fn reap_stash(s: u32) -> Weight {
		(75_073_000 as Weight)
			// Standard Error: 4_000
			.saturating_add((2_988_000 as Weight).saturating_mul(s as Weight))
			.saturating_add(RocksDbWeight::get().reads(6 as Weight))
			.saturating_add(RocksDbWeight::get().writes(6 as Weight))
			.saturating_add(RocksDbWeight::get().writes((1 as Weight).saturating_mul(s as Weight)))
	}

	fn new_era(v: u32, n: u32) -> Weight {
		(0 as Weight)
			// Standard Error: 1_146_000
			.saturating_add((362_986_000 as Weight).saturating_mul(v as Weight))
			// Standard Error: 57_000
			.saturating_add((60_216_000 as Weight).saturating_mul(n as Weight))
			.saturating_add(RocksDbWeight::get().reads(10 as Weight))
			.saturating_add(RocksDbWeight::get().reads((3 as Weight).saturating_mul(v as Weight)))
			.saturating_add(RocksDbWeight::get().reads((3 as Weight).saturating_mul(n as Weight)))
			.saturating_add(RocksDbWeight::get().writes(4 as Weight))
			.saturating_add(RocksDbWeight::get().writes((3 as Weight).saturating_mul(v as Weight)))
	}

	fn get_npos_voters(v: u32, n: u32, s: u32) -> Weight {
		(0 as Weight)
			// Standard Error: 230_000
			.saturating_add((35_891_000 as Weight).saturating_mul(v as Weight))
			// Standard Error: 230_000
			.saturating_add((37_854_000 as Weight).saturating_mul(n as Weight))
			// Standard Error: 7_842_000
			.saturating_add((32_492_000 as Weight).saturating_mul(s as Weight))
			.saturating_add(RocksDbWeight::get().reads(3 as Weight))
			.saturating_add(RocksDbWeight::get().reads((3 as Weight).saturating_mul(v as Weight)))
			.saturating_add(RocksDbWeight::get().reads((3 as Weight).saturating_mul(n as Weight)))
			.saturating_add(RocksDbWeight::get().reads((1 as Weight).saturating_mul(s as Weight)))
	}

	fn get_npos_targets(v: u32) -> Weight {
		(0 as Weight)
			// Standard Error: 74_000
			.saturating_add((16_370_000 as Weight).saturating_mul(v as Weight))
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			.saturating_add(RocksDbWeight::get().reads((1 as Weight).saturating_mul(v as Weight)))
	}

	fn set_staking_limits() -> Weight {
		(6_398_000 as Weight).saturating_add(RocksDbWeight::get().writes(4 as Weight))
	}

	fn chill_other() -> Weight {
		(44_694_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(5 as Weight))
			.saturating_add(RocksDbWeight::get().writes(2 as Weight))
	}
}
