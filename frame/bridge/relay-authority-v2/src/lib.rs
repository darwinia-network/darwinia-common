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

//! # Relay Authorities Module

#![cfg_attr(not(feature = "std"), no_std)]

pub mod migration;

// #[cfg(test)]
// mod mock;
// #[cfg(test)]
// mod test;

mod weights;
pub use weights::WeightInfo;

// --- core ---
use core::fmt::Debug;
// --- crates.io ---
use codec::FullCodec;
use scale_info::TypeInfo;
// --- paritytech ---
use frame_support::{pallet_prelude::*, traits::Get};
use frame_system::pallet_prelude::*;
use sp_io::{crypto, hashing};
use sp_runtime::Perbill;
use sp_std::prelude::*;

#[frame_support::pallet]
pub mod pallet {
	// --- darwinia-network ---
	use crate::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		// Overrides.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		// Basics.
		type WeightInfo: WeightInfo;
		// Origins.
		type AddOrigin: EnsureOrigin<Self::Origin>;
		type RemoveOrigin: EnsureOrigin<Self::Origin>;
		// Commitments.
		type MessageRootT: Clone + Debug + PartialEq + Encode + Decode + TypeInfo;
		type MessageRoot: Get<Self::MessageRootT>;
		type Sign: Sign;
		#[pallet::constant]
		type SignThreshold: Get<Perbill>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Dummy,
	}

	#[pallet::error]
	pub enum Error<T> {
		Dummy,
	}

	#[pallet::storage]
	#[pallet::getter(fn mmr_root_to_sign_of)]
	pub type Dummy<T: Config> = StorageMap<_, Identity, T::AccountId, T::AccountId, OptionQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		authorities: Vec<T::AccountId>,
	}
	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { authorities: Default::default() }
		}
	}
	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000_000)]
		pub fn force_new_term(origin: OriginFor<T>) -> DispatchResult {
			T::ResetOrigin::ensure_origin(origin)?;

			Self::apply_authorities_change()?;
			Self::sync_authorities_change()?;

			<NextAuthorities<T>>::kill();

			Ok(())
		}
	}
}
pub use pallet::*;

pub type EcdsaSigner = [u8; 20];
pub type EcdsaMessage = [u8; 32];
pub type EcdsaSignature = [u8; 65];

pub trait Sign {
	type Signature: Clone + Debug + PartialEq + FullCodec + TypeInfo;
	type Message: Clone + Debug + Default + PartialEq + FullCodec + TypeInfo;
	type Signer: Clone + Debug + Default + Ord + PartialEq + FullCodec + TypeInfo;

	fn hash(raw_message: impl AsRef<[u8]>) -> Self::Message;

	fn verify_signature(
		signature: &Self::Signature,
		message: &Self::Message,
		signer: &Self::Signer,
	) -> bool;
}
pub enum EcdsaSign {}
impl Sign for EcdsaSign {
	type Message = EcdsaMessage;
	type Signature = EcdsaSignature;
	type Signer = EcdsaSigner;

	fn hash(raw_message: impl AsRef<[u8]>) -> Self::Message {
		hashing::keccak_256(raw_message.as_ref())
	}

	fn verify_signature(
		signature: &Self::Signature,
		message: &Self::Message,
		signer: &Self::Signer,
	) -> bool {
		fn eth_signable_message(message: &[u8]) -> Vec<u8> {
			let mut l = message.len();
			let mut rev = Vec::new();

			while l > 0 {
				rev.push(b'0' + (l % 10) as u8);
				l /= 10;
			}

			let mut v = b"\x19Ethereum Signed Message:\n".to_vec();

			v.extend(rev.into_iter().rev());
			v.extend_from_slice(message);

			v
		}

		let message = hashing::keccak_256(&eth_signable_message(message));

		if let Ok(public_key) = crypto::secp256k1_ecdsa_recover(signature, &message) {
			hashing::keccak_256(&public_key)[12..] == signer[..]
		} else {
			false
		}
	}
}
