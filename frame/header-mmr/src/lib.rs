// This file is part of Darwinia.
//
// Copyright (C) 2018-2021 Darwinia Network
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

//! # Chain MMR Pallet
//!
//! ## Overview
//! This is the pallet to maintain accumulate headers Merkle Mountain Range
//! and push the mmr root in to the digest of block headers on finalize.
//! MMR can be used for light client to implement super light clients,
//! and can also be used in other chains to implement chain relay for
//! cross-chain verification purpose.
//!
//! ## Terminology
//!
//! ### Merkle Mountain Range
//! For more details about the MMR struct, refer https://github.com/mimblewimble/grin/blob/master/doc/mmr.md#structure
//!
//! ### MMR Proof
//! Using the MMR Store Storage, MMR Proof can be generated for specific
//! block header hash. Proofs can be used to verify block inclusion together with
//! the mmr root in the header digest.
//!
//! ### Digest Item
//! The is a ```MerkleMountainRangeRoot(Hash)``` digest item pre-subscribed in Digest.
//! This is implemented in Darwinia's fork of substrate: https://github.com/darwinia-network/substrate
//! The Pull request link is https://github.com/darwinia-network/substrate/pull/1
//!
//! ## Implementation
//! We are using the MMR library from https://github.com/nervosnetwork/merkle-mountain-range
//! Pull request: https://github.com/darwinia-network/darwinia/pull/358
//!
//! ## References
//! Darwinia Relay's Technical Paper:
//! https://github.com/darwinia-network/rfcs/blob/master/paper/Darwinia_Relay_Sublinear_Optimistic_Relay_for_Interoperable_Blockchains_v0.7.pdf
//!
//! https://github.com/mimblewimble/grin/blob/master/doc/mmr.md#structure
//! https://github.com/mimblewimble/grin/blob/0ff6763ee64e5a14e70ddd4642b99789a1648a32/core/src/core/pmmr.rs#L606
//! https://github.com/nervosnetwork/merkle-mountain-range/blob/master/src/tests/test_accumulate_headers.rs
//! https://eprint.iacr.org/2019/226.pdf
//!

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod types {
	/// The type use for indexing a node
	pub type NodeIndex = u64;
}
pub use types::*;

pub mod primitives;

pub mod weights;

pub mod migration {
	// --- paritytech ---
	use frame_support::migration;
	// --- darwinia ---
	use crate::*;

	#[cfg(test)]
	pub fn initialize_new_mmr_state<T>(size: NodeIndex, mmr: Vec<T::Hash>, pruning_step: NodeIndex)
	where
		T: Config,
	{
		MmrSize::put(size);
		PruningConfiguration::put(MmrNodesPruningConfiguration {
			step: pruning_step,
			progress: 0,
			last_position: size,
		});

		for position in mmr::helper::get_peaks(size) {
			<Peaks<T>>::insert(position, mmr[position as usize]);
		}
		for (position, hash) in mmr.into_iter().enumerate() {
			<MMRNodeList<T>>::insert(position as NodeIndex, hash);
		}
	}

	#[cfg(not(test))]
	pub fn initialize_new_mmr_state<T>(module: &[u8], pruning_step: NodeIndex)
	where
		T: Config,
	{
		let size = migration::take_storage_value::<NodeIndex>(module, b"MMRCounter", &[])
			.expect("`MMRCounter` MUST be existed; qed");

		migration::remove_storage_prefix(module, b"MMRCounter", &[]);

		MmrSize::put(size);
		PruningConfiguration::put(MmrNodesPruningConfiguration {
			step: pruning_step,
			progress: 0,
			last_position: size,
		});

		for position in mmr::helper::get_peaks(size) {
			<Peaks<T>>::insert(
				position,
				<MMRNodeList<T>>::get(position).expect("Node MUST be existed; qed"),
			);
		}
	}
}

#[cfg(feature = "std")]
use serde::Serialize;

// --- paritytech ---
use codec::{Decode, Encode};
use frame_support::{decl_module, decl_storage, traits::Get, weights::Weight};
use frame_system::ensure_root;
use sp_io::offchain_index;
use sp_runtime::{generic::DigestItem, traits::SaturatedConversion, DispatchResult, RuntimeDebug};
#[cfg(any(test, feature = "easy-testing"))]
use sp_runtime::{generic::OpaqueDigestItemId, traits::Header};
use sp_std::prelude::*;
// --- darwinia ---
use darwinia_header_mmr_rpc_runtime_api::{Proof, RuntimeDispatchInfo};
use darwinia_relay_primitives::MMR as MMRT;
use darwinia_support::impl_rpc;
use primitives::*;
use weights::WeightInfo;

/// The prefix of [`MerkleMountainRangeRootLog`]
pub const LOG_PREFIX: [u8; 4] = *b"MMRR";

pub trait Config: frame_system::Config {
	type WeightInfo: WeightInfo;

	/// The offchain-indexing prefix
	const INDEXING_PREFIX: &'static [u8];
}

decl_storage! {
	trait Store for Module<T: Config> as DarwiniaHeaderMMR {
		/// Size of the MMR
		pub MmrSize get(fn mmr_size): NodeIndex;

		/// MMR struct of the previous blocks, from first(genesis) to parent hash.
		pub MMRNodeList get(fn mmr_node_list): map hasher(identity) NodeIndex => Option<T::Hash>;

		/// Peaks of the MMR
		pub Peaks get(fn peak_of): map hasher(identity) NodeIndex => Option<T::Hash>;

		pub PruningConfiguration get(fn pruning_configuration): MmrNodesPruningConfiguration;
	}
}

decl_module! {
	pub struct Module<T: Config> for enum Call
	where
		origin: T::Origin
	{
		fn on_initialize(_block_number: T::BlockNumber) -> Weight {
			PruningConfiguration::try_mutate(
				|MmrNodesPruningConfiguration {
					 step,
					 progress,
					 last_position,
				 }| {
					if *progress > *last_position {
						return Err(T::DbWeight::get().reads(1));
					}

					for position in *progress..progress.saturating_add(*step) {
						if let Some(hash) = <MMRNodeList<T>>::take(position) {
							hash.using_encoded(|hash| {
								offchain_index::set(&<Pallet<T>>::offchain_key(position), hash)
							});

							log::trace!("Pruned node `{:?}` at position `{}`", hash, position);
						}
					}

					*progress = progress.saturating_add(*step);

					Ok(T::DbWeight::get().reads_writes(1, *step * 2))
				},
			)
			.map_or_else(|weight| weight, |weight| weight)
		}

		fn on_finalize(_block_number: T::BlockNumber) {
			let parent_hash = <frame_system::Pallet<T>>::parent_hash();
			let mut mmr = <Mmr<RuntimeStorage, T>>::new();
			let _ = mmr.push(parent_hash);

			match mmr.finalize() {
				Ok(parent_mmr_root) => {
					let mmr_root_log = MerkleMountainRangeRootLog::<T::Hash> {
						prefix: LOG_PREFIX,
						parent_mmr_root,
					};
					let mmr_item = DigestItem::Other(mmr_root_log.encode());

					<frame_system::Pallet<T>>::deposit_log(mmr_item.into());
				}
				Err(e) => {
					log::error!("Failed to finalize MMR due to {}", e);
				}
			}
		}

		#[weight = T::DbWeight::get().writes(1)]
		pub fn config_pruning(
			origin,
			step: Option<NodeIndex>,
			progress: Option<NodeIndex>,
			last_position: Option<NodeIndex>
		) {
			ensure_root(origin)?;

			PruningConfiguration::try_mutate(|c| {
				let mut modified = false;

				if let Some(step) = step {
					c.step = step;
					modified = true;
				}
				if let Some(progress) = progress {
					c.progress = progress;
					modified = true;
				}
				if let Some(last_position) = last_position {
					c.last_position = last_position;
					modified = true;
				}

				if modified {
					Ok(().into())
				} else {
					DispatchResult::Err("No changes".into())
				}
			})?;
		}
	}
}

impl<T: Config> Module<T> {
	pub fn offchain_key(position: NodeIndex) -> Vec<u8> {
		(T::INDEXING_PREFIX, position).encode()
	}

	impl_rpc! {
		pub fn gen_proof_rpc(
			block_number_of_member_leaf: NodeIndex,
			block_number_of_last_leaf: NodeIndex,
		) -> RuntimeDispatchInfo<T::Hash> {
			if block_number_of_member_leaf <= block_number_of_last_leaf {
				let mmr_size = mmr::leaf_index_to_mmr_size(block_number_of_last_leaf);

				if mmr_size <= MmrSize::get() {
						let position = mmr::leaf_index_to_pos(block_number_of_member_leaf);
						let mmr = <Mmr<OffchainStorage, T>>::with_size(MmrSize::get());

						if let Ok(merkle_proof) = mmr.gen_proof(position) {
						return RuntimeDispatchInfo {
							mmr_size,
							proof: Proof(merkle_proof.proof_items().to_vec()),
						};
					}
				}
			}

			Default::default()
		}
	}

	// Remove the cfg, once there's a requirement from runtime usage
	#[cfg(any(test, feature = "easy-testing"))]
	pub fn find_parent_mmr_root(header: &T::Header) -> Option<T::Hash> {
		let find_parent_mmr_root = |m: MerkleMountainRangeRootLog<_>| match m.prefix {
			LOG_PREFIX => Some(m.parent_mmr_root),
			_ => None,
		};

		// find the first other digest with the right prefix which converts to
		// the right kind of mmr root log.
		header.digest().convert_first(|d| {
			d.try_to(OpaqueDigestItemId::Other)
				.and_then(find_parent_mmr_root)
		})
	}
}

impl<T: Config> MMRT<T::BlockNumber, T::Hash> for Pallet<T> {
	fn get_root(block_number: T::BlockNumber) -> Option<T::Hash> {
		let size = mmr::leaf_index_to_mmr_size(block_number.saturated_into());

		<Mmr<RuntimeStorage, T>>::with_size(size).get_root().ok()
	}
}

#[cfg_attr(feature = "std", derive(Serialize))]
#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct MerkleMountainRangeRootLog<Hash> {
	/// Specific prefix to identify the mmr root log in the digest items with Other type.
	pub prefix: [u8; 4],
	/// The merkle mountain range root hash.
	pub parent_mmr_root: Hash,
}

#[derive(Clone, Default, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct MmrNodesPruningConfiguration {
	/// The nodes num that should be pruned each block
	pub step: NodeIndex,
	/// The progress of last time pruning
	pub progress: NodeIndex,
	/// Should stop pruning after reach the last node's position
	pub last_position: NodeIndex,
}
