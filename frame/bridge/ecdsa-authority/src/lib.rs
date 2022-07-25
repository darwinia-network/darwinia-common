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

pub mod primitives;
use primitives::*;

mod weights;
pub use weights::WeightInfo;

// --- core ---
use core::fmt::Debug;
// --- crates.io ---
use scale_info::TypeInfo;
// --- paritytech ---
use frame_support::{pallet_prelude::*, traits::Get};
use frame_system::pallet_prelude::*;
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
		// Members.
		type ModifyOrigin: EnsureOrigin<Self::Origin>;
		#[pallet::constant]
		type SentinelMember: Get<Address>;
		#[pallet::constant]
		type MaxMembers: Get<u32>;
		// Commitment relates.
		type MessageRootT: Clone + Debug + PartialEq + Encode + Decode + TypeInfo;
		type MessageRoot: Get<Self::MessageRootT>;
		#[pallet::constant]
		type SignThreshold: Get<Perbill>;
		#[pallet::constant]
		type RelayTypeHash: Get<[u8; 32]>;
		#[pallet::constant]
		type MethodIds: Get<[[u8; 4]; 4]>;
		#[pallet::constant]
		type CommitTypeHash: Get<[u8; 32]>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Dummy,
	}

	#[pallet::error]
	pub enum Error<T> {
		TooManyMembers,
		NotAuthority,
		BadSignature,
	}

	#[pallet::storage]
	#[pallet::getter(fn authorities)]
	pub type Authorities<T: Config> =
		StorageValue<_, BoundedVec<Address, T::MaxMembers>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn authorities_change_to_sign)]
	pub type AuthoritiesChangeToSign<T: Config> =
		StorageValue<_, (Message, BoundedVec<(Address, Signature), T::MaxMembers>), ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn nonce)]
	pub type Nonce<T: Config> = StorageValue<_, u32, ValueQuery>;

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
		pub fn add_authority(origin: OriginFor<T>, new: Address) -> DispatchResult {
			T::ModifyOrigin::ensure_origin(origin)?;

			ensure!(
				<Authorities<T>>::decode_len().map(|l| l as u32).unwrap_or(T::MaxMembers::get())
					< T::MaxMembers::get(),
				<Error<T>>::TooManyMembers
			);

			let authorities_count = <Authorities<T>>::mutate(|authorities| {
				let _ = authorities.try_insert(0, new.clone());

				authorities.len() as u32
			});

			Self::on_authorities_change(T::MethodIds::get().0, &[new], authorities_count);

			Ok(())
		}

		#[pallet::weight(10_000_000)]
		pub fn remove_authority(origin: OriginFor<T>, old: Address) -> DispatchResult {
			T::ModifyOrigin::ensure_origin(origin)?;

			// let (authorities_count) = <Authorities<T>>::mutate(|authorities| {
			// let i = authorities.iter().position(|a| a == &old).unwrap();

			// authorities.remove(i);

			// (authorities.len() as u32, authorities[i - 1])
			// });

			// Self::on_authorities_change(&[who], T::SignThreshold::get() * authorities_count);

			Ok(())
		}

		#[pallet::weight(10_000_000)]
		pub fn swap_authority(origin: OriginFor<T>, old: Address, new: Address) -> DispatchResult {
			T::ModifyOrigin::ensure_origin(origin)?;

			// let (authorities_count) = <Authorities<T>>::mutate(|authorities| {
			// let i = authorities.iter().position(|a| a == &old).unwrap();
			// let old = authorities[i].clone();

			// authorities[i] = new.clone();

			// (authorities.len() as u32, authorities[i - 1])
			// });

			// Self::on_authorities_change(&[who], T::SignThreshold::get() * authorities_count);

			Ok(())
		}

		#[pallet::weight(10_000_000)]
		pub fn submit_signed_authorities_change(
			origin: OriginFor<T>,
			address: Address,
			signature: Signature,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let authorities = Self::ensure_authority(&address)?;
			let mut authorities_change_to_sign = <AuthoritiesChangeToSign<T>>::get();
			let (message, collected) = &mut authorities_change_to_sign;

			ensure!(
				Sign::verify_signature(&signature, message, &address),
				<Error<T>>::BadSignature
			);

			if Perbill::from_rational(collected.len() as u32, authorities.len() as u32)
				>= T::SignThreshold::get()
			{
			} else {
				collected.try_push((address, signature)).map_err(|_| <Error<T>>::TooManyMembers)?;

				<AuthoritiesChangeToSign<T>>::put(authorities_change_to_sign);
			}

			Ok(())
		}
	}
	impl<T: Config> Pallet<T> {
		fn on_authorities_change(
			method_id: [u8; 4],
			authorities_changes: &[Address],
			authorities_count: u32,
		) {
			let require_sign_count = T::SignThreshold::get() * authorities_count;
			let nonce = <Nonce<T>>::mutate(|nonce| {
				*nonce += 1;

				*nonce
			});
			let relay_message = RelayMessage {
				_1: T::RelayTypeHash::get(),
				_2: T::Version::get().spec_name,
				_3: method_id,
				_4: (authorities_changes, require_sign_count),
				_5: (),
				_6: nonce,
			};
		}

		fn ensure_authority(
			address: &Address,
		) -> Result<BoundedVec<Address, T::MaxMembers>, DispatchError> {
			let authorities = <Authorities<T>>::get();

			ensure!(authorities.iter().any(|a| a == address), <Error<T>>::NotAuthority);

			Ok(authorities)
		}
	}
}
pub use pallet::*;
