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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

//! Prototype module for basic message outbound.

#![cfg_attr(not(feature = "std"), no_std)]

pub mod weight;
pub use weight::WeightInfo;

// --- crates ---
use codec::Encode;
use ethereum_types::{H160, H256};
// --- substrate ---
use frame_support::{ensure, pallet_prelude::*, traits::Get};
use frame_system::pallet_prelude::*;
use pallet_mmr_primitives::{LeafDataProvider, OnNewRoot};
use sp_io::offchain_index;
use sp_runtime::{
	traits::{Hash, Zero},
	SaturatedConversion,
};
use sp_std::prelude::*;

use dp_contract::basic_channel::{BasicMessage, MmrLeaf};

pub use pallet::*;
use sp_runtime::DigestItem;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	pub mod types {
		/// The type use for indexing a node
		pub type NodeIndex = u64;
	}
	pub use types::*;

	pub const BASIC_MESSAGE_PREFIX: [u8; 12] = *b"BasicMessage";
	pub const BASIC_MMR_PREFIX: [u8; 4] = *b"bmmr";

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type WeightInfo: WeightInfo;

		/// the message key prefix for offchain storage.
		#[pallet::constant]
		type LimitSizeEachMessage: Get<u64>;
		#[pallet::constant]
		type LimitCommittedMessageSize: Get<u64>;

		type Hashing: Hash<Output = H256>;
	}

	/// Size of the MMR
	#[pallet::storage]
	#[pallet::getter(fn mmr_size)]
	pub type MmrSize<T> = StorageValue<_, NodeIndex, ValueQuery>;

	/// Peaks of the MMR
	#[pallet::storage]
	#[pallet::getter(fn peak_of)]
	pub type Peaks<T: Config> = StorageMap<_, Identity, NodeIndex, T::Hash, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	pub enum Event<T> {
		MessageSubmitted(u64),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The message payload exceeds byte limit.
		OverLimitPayload,
		/// No more messages can be queued for the channel during this commit cycle.
		OverLimitCommittedMessage,
		/// Cannot increment nonce
		NonceOverFlow,
		/// Not authorized to send message
		NotAuthorized,
	}

	#[pallet::storage]
	#[pallet::getter(fn interval)]
	pub type Interval<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn pending_message)]
	pub type PendingMessage<T: Config> = StorageValue<_, Vec<BasicMessage>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn nonce)]
	pub type Nonce<T: Config> = StorageValue<_, u64, ValueQuery>;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_now: BlockNumberFor<T>) -> Weight {
			T::DbWeight::get().writes(1)
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {}

	impl<T: Config> Pallet<T> {
		/// Submit message on the outbound channel
		pub fn submit(target: H160, payload: &[u8]) -> DispatchResult {
			ensure!(
				<PendingMessage<T>>::decode_len().unwrap_or(0)
					< T::LimitCommittedMessageSize::get() as usize,
				Error::<T>::OverLimitCommittedMessage,
			);
			ensure!(
				payload.len() <= T::LimitSizeEachMessage::get() as usize,
				Error::<T>::OverLimitPayload,
			);

			<Nonce<T>>::try_mutate(|nonce| {
				if let Some(v) = nonce.checked_add(1) {
					*nonce = v;
				} else {
					return Err(Error::<T>::NonceOverFlow.into());
				}

				<PendingMessage<T>>::append(BasicMessage {
					target,
					nonce: *nonce,
					payload: payload.to_vec(),
				});
				Self::deposit_event(Event::MessageSubmitted(*nonce));
				Ok(())
			})
		}
	}

	impl<T: Config> LeafDataProvider for Pallet<T> {
		type LeafData = H256;
		fn leaf_data() -> Self::LeafData {
			let parent_hash = <frame_system::Pallet<T>>::parent_hash();
			let block_number = <frame_system::Pallet<T>>::block_number();
			let messages: Vec<BasicMessage> = if (block_number % <Interval<T>>::get()).is_zero() {
				<PendingMessage<T>>::take()
			} else {
				vec![]
			};
			let commitment = BasicMessage::encode_messages(&messages);
			let commitment_hash = <T as Config>::Hashing::hash(&commitment);

			let leaf = MmrLeaf::new(
				parent_hash.as_ref(),
				commitment_hash,
				block_number.saturated_into::<u32>(),
			)
			.encode();
			let mmr_leaf_hash = <T as Config>::Hashing::hash(&leaf);

			let key = (BASIC_MESSAGE_PREFIX, commitment_hash).encode();
			offchain_index::set(&*key, &messages.encode());
			mmr_leaf_hash
		}
	}

	impl<T: Config> OnNewRoot<T::Hash> for Pallet<T> {
		fn on_new_root(root: &T::Hash) {
			let mmr_root_log = MmrLog::<T::Hash> {
				prefix: BASIC_MMR_PREFIX,
				mmr: root.clone(),
			};
			let mmr_item = DigestItem::Other(mmr_root_log.encode());
			<frame_system::Pallet<T>>::deposit_log(mmr_item.into());
		}
	}

	#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
	pub struct MmrLog<Hash> {
		/// The prefix for MMRRoot hash in the system log.
		pub prefix: [u8; 4],
		/// The merkle mountain range root hash.
		pub mmr: Hash,
	}
}
