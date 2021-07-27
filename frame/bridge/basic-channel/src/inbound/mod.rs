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

//! Prototype module for basic message inbound.

#![cfg_attr(not(feature = "std"), no_std)]

pub mod weight;
pub use weight::WeightInfo;

// --- crates ---
use codec::{Decode, Encode};
use ethereum_types::{H160, H256};
// --- substrate ---
use frame_support::{pallet_prelude::*, traits::Get};
use frame_system::pallet_prelude::*;

use dp_contract::basic_channel::BasicInboundMessage;
use ethereum_primitives::receipt::{EthereumReceipt, EthereumReceiptProof};

pub use pallet::*;
use sp_std::fmt::Debug;

pub trait ChainHeader<H> {
	fn hash(&self) -> H;
	fn transaction_root(&self) -> H;
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type WeightInfo: WeightInfo;
		type RemoteHeader: ChainHeader<H256> + Clone + Encode + PartialEq + Decode + Debug;
		type SourceChannel: Get<H160>;
	}

	#[pallet::event]
	pub enum Event<T> {}

	#[pallet::error]
	pub enum Error<T> {
		/// Invalid Nonce
		InvalidNonce,
		/// Receipt Proof Invalid
		ReceiptProofInv,
	}

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
	impl<T: Config> Pallet<T> {
		/// Submit message on the inbound channel
		#[pallet::weight(0)]
		pub fn submit(
			origin: OriginFor<T>,
			header: T::RemoteHeader,
			proof: EthereumReceiptProof,
		) -> DispatchResult {
			ensure_signed(origin)?;
			//TODO
			// we need verify this message first by relay pallet such bsc, heco...
			//T::Relay::verify(header);

			let verified_receipt =
				EthereumReceipt::verify_proof_and_generate(&header.transaction_root(), &proof)
					.map_err(|_| <Error<T>>::ReceiptProofInv)?;
			let source_channel = T::SourceChannel::get();

			let message: BasicInboundMessage =
				BasicInboundMessage::parse_channel_event(&source_channel, &verified_receipt)
					.map_err(|_| <Error<T>>::InvalidNonce)?;
			<Nonce<T>>::try_mutate(|nonce| {
				if message.nonce != *nonce + 1 {
					Err(Error::<T>::InvalidNonce.into())
				} else {
					*nonce += 1;
					Ok(())
				}
			})
		}
	}
}
