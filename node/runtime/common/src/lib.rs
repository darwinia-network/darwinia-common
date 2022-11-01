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

//! Common runtime code for Darwinia and Crab.

#![cfg_attr(not(feature = "std"), no_std)]

pub mod gov_origin;
pub use gov_origin::*;

/// Implementations of some helper traits passed into runtime modules as associated types.
pub mod impls;
pub use impls::*;

pub use darwinia_balances::{Instance1 as RingInstance, Instance2 as KtonInstance};
pub use frame_support::weights::constants::{ExtrinsicBaseWeight, RocksDbWeight};

/// Primitives of the Pangolin chain.
pub use bp_darwinia_core as bp_pangolin;
/// Primitives of the Pangolin-parachain and Pangolin-parachain-alpha chain.
pub use bp_darwinia_core as bp_pangolin_parachain;
/// Primitives of the Pangoro chain.
pub use bp_darwinia_core as bp_pangoro;
/// Re-export DarwiniaLike as different chain type.
pub use bp_darwinia_core::{
	DarwiniaLike as Pangolin, DarwiniaLike as Pangoro, DarwiniaLike as PangolinParaChain,
};

/// Primitives of the Rococo chain.
pub use bp_polkadot_core as bp_rococo;
/// Re-export PolkadotLike as different relay chain type.
pub use bp_polkadot_core::{PolkadotLike as Rococo, PolkadotLike as MoonbaseRelay};

// --- crates.io ---
use static_assertions::const_assert;
// --- paritytech ---
use frame_election_provider_support::onchain::OnChainSequentialPhragmen;
use frame_support::{
	traits::Currency,
	weights::{
		constants::{BlockExecutionWeight, WEIGHT_PER_SECOND},
		DispatchClass, Weight,
	},
};
use frame_system::limits::{BlockLength, BlockWeights};
use pallet_transaction_payment::{Multiplier, TargetedFeeAdjustment};
use sp_runtime::{FixedPointNumber, Perbill, Perquintill};
// --- darwinia-network ---
use drml_primitives::BlockNumber;

pub type RingNegativeImbalance<T> = <darwinia_balances::Pallet<T, RingInstance> as Currency<
	<T as frame_system::Config>::AccountId,
>>::NegativeImbalance;

/// Parameterized slow adjusting fee updated based on
/// https://research.web3.foundation/en/latest/polkadot/overview/2-token-economics.html#-2.-slow-adjusting-mechanism
pub type SlowAdjustingFeeUpdate<R> =
	TargetedFeeAdjustment<R, TargetBlockFullness, AdjustmentVariable, MinimumMultiplier>;

/// The accuracy type used for genesis election provider;
pub type OnOnChainAccuracy = Perbill;

/// The election provider of the genesis
pub type GenesisElectionOf<T> = OnChainSequentialPhragmen<T>;

/// We assume that an on-initialize consumes 2.5% of the weight on average, hence a single extrinsic
/// will not be allowed to consume more than `AvailableBlockRatio - 2.5%`.
pub const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_perthousand(25);
/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used
/// by  Operational  extrinsics.
pub const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
/// We allow for 2 seconds of compute with a 6 second average block time.
pub const MAXIMUM_BLOCK_WEIGHT: Weight = 2 * WEIGHT_PER_SECOND;
const_assert!(NORMAL_DISPATCH_RATIO.deconstruct() >= AVERAGE_ON_INITIALIZE_RATIO.deconstruct());

/// Maximum number of iterations for balancing that will be executed in the embedded miner of
/// pallet-election-provider-multi-phase.
pub const MINER_MAX_ITERATIONS: u32 = 10;

// According to the EVM gas benchmark, 1 gas ~= 40_000 weight.
pub const WEIGHT_PER_GAS: u64 = 40_000;

frame_support::parameter_types! {
	pub const BlockHashCountForPangolin: BlockNumber = 256;
	pub const BlockHashCountForPangoro: BlockNumber = 2400;
	/// The portion of the `NORMAL_DISPATCH_RATIO` that we adjust the fees with. Blocks filled less
	/// than this will decrease the weight and more will increase.
	pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
	/// The adjustment variable of the runtime. Higher values will cause `TargetBlockFullness` to
	/// change the fees more rapidly.
	pub AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(3, 100_000);
	/// Minimum amount of the multiplier. This value cannot be too low. A test case should ensure
	/// that combined with `AdjustmentVariable`, we can recover from the minimum.
	/// See `multiplier_can_grow_from_zero`.
	pub MinimumMultiplier: Multiplier = Multiplier::saturating_from_rational(1, 1_000_000_000u128);
	/// Maximum length of block. Up to 5MB.
	pub RuntimeBlockLength: BlockLength =
		BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	/// Block weights base values and limits.
	pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
		.base_block(BlockExecutionWeight::get())
		.for_class(DispatchClass::all(), |weights| {
			weights.base_extrinsic = ExtrinsicBaseWeight::get();
		})
		.for_class(DispatchClass::Normal, |weights| {
			weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
		})
		.for_class(DispatchClass::Operational, |weights| {
			weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
			// Operational transactions have some extra reserved space, so that they
			// are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
			weights.reserved = Some(
				MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
			);
		})
		.avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
		.build_or_panic();
}

frame_support::parameter_types! {
	/// A limit for off-chain phragmen unsigned solution submission.
	///
	/// We want to keep it as high as possible, but can't risk having it reject,
	/// so we always subtract the base block execution weight.
	pub OffchainSolutionWeightLimit: Weight = RuntimeBlockWeights::get()
		.get(DispatchClass::Normal)
		.max_extrinsic
		.expect("Normal extrinsics have weight limit configured by default; qed")
		.saturating_sub(BlockExecutionWeight::get());

	/// A limit for off-chain phragmen unsigned solution length.
	///
	/// We allow up to 90% of the block's size to be consumed by the solution.
	pub OffchainSolutionLengthLimit: u32 = Perbill::from_rational(90_u32, 100) *
		*RuntimeBlockLength::get()
		.max
		.get(DispatchClass::Normal);
}
