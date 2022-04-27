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

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	pub mod types {
		/// The type use for indexing a node
		pub type NodeIndex = u64;
	}
	pub use types::*;

	// --- crates.io ---
	use scale_info::TypeInfo;
	#[cfg(feature = "std")]
	use serde::Serialize;
	// --- paritytech ---
	use frame_support::{log, pallet_prelude::*};
	use frame_system::pallet_prelude::*;
	use sp_runtime::generic::DigestItem;
	#[cfg(any(test, feature = "easy-testing"))]
	use sp_runtime::{generic::OpaqueDigestItemId, traits::Header};
	use sp_std::prelude::*;
	// --- darwinia-network ---
	use crate::{primitives::*, weights::WeightInfo};
	use darwinia_header_mmr_rpc_runtime_api::{Proof, RuntimeDispatchInfo};
	use darwinia_relay_primitives::MMR as MMRT;

	/// The prefix of [`MerkleMountainRangeRootLog`]
	pub const LOG_PREFIX: [u8; 4] = *b"MMRR";

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type WeightInfo: WeightInfo;

		/// The offchain-indexing prefix
		const INDEXING_PREFIX: &'static [u8];
	}

	/// Size of the MMR
	#[pallet::storage]
	#[pallet::getter(fn mmr_size)]
	pub type MmrSize<T> = StorageValue<_, NodeIndex, ValueQuery>;

	/// Peaks of the MMR
	#[pallet::storage]
	#[pallet::getter(fn peak_of)]
	pub type Peaks<T: Config> = StorageMap<_, Identity, NodeIndex, T::Hash, OptionQuery>;

	#[pallet::pallet]
	pub struct Pallet<T>(_);
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(_: BlockNumberFor<T>) {
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
	}
	impl<T: Config> Pallet<T> {
		pub fn offchain_key(position: NodeIndex) -> Vec<u8> {
			(T::INDEXING_PREFIX, position).encode()
		}

		darwinia_support::impl_rpc! {
			pub fn gen_proof_rpc(
				block_number_of_member_leaf: NodeIndex,
				block_number_of_last_leaf: NodeIndex,
			) -> RuntimeDispatchInfo<T::Hash> {
				if block_number_of_member_leaf <= block_number_of_last_leaf {
					let mmr_size = mmr::leaf_index_to_mmr_size(block_number_of_last_leaf);

					if mmr_size <= <MmrSize<T>>::get() {
						let mmr = <Mmr<OffchainStorage, T>>::with_size(mmr_size);

						if let Ok(merkle_proof) = mmr.gen_proof(block_number_of_member_leaf) {
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
				d.try_to(OpaqueDigestItemId::Other).and_then(find_parent_mmr_root)
			})
		}
	}
	impl<T: Config> MMRT<BlockNumberFor<T>, T::Hash> for Pallet<T> {
		fn get_root() -> Option<T::Hash> {
			<Mmr<RuntimeStorage, T>>::with_size(<MmrSize<T>>::get()).get_root().ok()
		}
	}

	#[cfg_attr(feature = "std", derive(Serialize))]
	#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
	pub struct MerkleMountainRangeRootLog<Hash> {
		/// Specific prefix to identify the mmr root log in the digest items with Other type.
		pub prefix: [u8; 4],
		/// The merkle mountain range root hash.
		pub parent_mmr_root: Hash,
	}
}
pub use pallet::*;

pub mod primitives;

pub mod weights;

pub mod migration {
	#[cfg(feature = "try-runtime")]
	pub mod try_runtime {
		pub fn pre_migrate() -> Result<(), &'static str> {
			Ok(())
		}
	}

	pub fn migrate() {}
}
