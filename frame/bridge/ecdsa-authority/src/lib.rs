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
//! Works with https://github.com/darwinia-network/darwinia-messages-sol/pull/217

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

// --- crates.io ---
use ethabi::Token;
// --- darwinia-network ---
use dp_message::network_ids::{self, NetworkId};
// --- paritytech ---
use frame_support::{pallet_prelude::*, traits::Get};
use frame_system::pallet_prelude::*;
use sp_runtime::{
	traits::{Saturating, Zero},
	Perbill,
};
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
		// Members
		#[pallet::constant]
		type MaxAuthorities: Get<u32>;
		// Commitment relates.
		type MessageRoot: Get<Option<Hash>>;
		#[pallet::constant]
		type SignThreshold: Get<Perbill>;
		//
		#[pallet::constant]
		type SyncInterval: Get<Self::BlockNumber>;
		#[pallet::constant]
		type MaxPendingPeriod: Get<Self::BlockNumber>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		CollectingAuthoritiesChangeSignature(Message),
		CollectedEnoughAuthoritiesChangeSignatures((Message, Vec<(Address, Signature)>)),

		CollectingNewMessageRootSignature(Message),
		CollectedEnoughNewMessageRootSignatures((Message, Vec<(Address, Signature)>)),
	}

	#[pallet::error]
	pub enum Error<T> {
		TooManyAuthorities,
		NotAuthority,
		OnAuthoritiesChange,
		NoAuthoritiesChange,
		NoNewMessageRoot,
		BadSignature,
	}

	#[pallet::storage]
	#[pallet::getter(fn authorities)]
	pub type Authorities<T: Config> =
		StorageValue<_, BoundedVec<Address, T::MaxAuthorities>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn nonce)]
	pub type Nonce<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn authorities_change_to_sign)]
	pub type AuthoritiesChangeToSign<T: Config> = StorageValue<
		_,
		(Message, BoundedVec<(Address, Signature), T::MaxAuthorities>),
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn new_message_root_to_sign)]
	pub type NewMessageRootToSign<T: Config> = StorageValue<
		_,
		(Message, BoundedVec<(Address, Signature), T::MaxAuthorities>),
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn previous_message_root)]
	pub type PreviousMessageRoot<T: Config> = StorageValue<_, (T::BlockNumber, Hash), ValueQuery>;

	#[derive(Default)]
	#[pallet::genesis_config]
	pub struct GenesisConfig {
		authorities: Vec<Address>,
	}
	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {
			<Authorities<T>>::put(BoundedVec::try_from(self.authorities.clone()).unwrap());
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);
	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
		fn on_initialize(now: T::BlockNumber) -> Weight {
			if (now % T::SyncInterval::get()).is_zero() {
				if let Some(message_root) = Self::try_update_message_root(now) {
					Self::on_new_message_root(message_root);
				}
			}

			10_000_000
		}
	}
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000_000)]
		pub fn add_authority(origin: OriginFor<T>, new: Address) -> DispatchResult {
			ensure_root(origin)?;

			Self::ensure_not_on_authorities_change()?;

			ensure!(
				<Authorities<T>>::decode_len()
					.map(|l| l as u32)
					.unwrap_or(T::MaxAuthorities::get())
					< T::MaxAuthorities::get(),
				<Error<T>>::TooManyAuthorities
			);

			let authorities_count = <Authorities<T>>::mutate(|authorities| {
				let _ = authorities.try_insert(0, new);

				authorities.len() as u32
			});

			Self::on_authorities_change(Method::AddMember { new }, authorities_count);

			Ok(())
		}

		#[pallet::weight(10_000_000)]
		pub fn remove_authority(origin: OriginFor<T>, old: Address) -> DispatchResult {
			ensure_root(origin)?;

			Self::ensure_not_on_authorities_change()?;

			let (authorities_count, pre) = <Authorities<T>>::try_mutate(|authorities| {
				let i =
					authorities.iter().position(|a| a == &old).ok_or(<Error<T>>::NotAuthority)?;

				authorities.remove(i);

				Ok::<_, DispatchError>((
					authorities.len() as u32,
					authorities.get(i - 1).map(Clone::clone).unwrap_or(AUTHORITY_SENTINEL),
				))
			})?;

			Self::on_authorities_change(Method::RemoveMember { pre, old }, authorities_count);

			Ok(())
		}

		#[pallet::weight(10_000_000)]
		pub fn swap_authority(origin: OriginFor<T>, old: Address, new: Address) -> DispatchResult {
			ensure_root(origin)?;

			Self::ensure_not_on_authorities_change()?;

			let (authorities_count, pre) = <Authorities<T>>::try_mutate(|authorities| {
				let i =
					authorities.iter().position(|a| a == &old).ok_or(<Error<T>>::NotAuthority)?;

				authorities[i] = new;

				Ok::<_, DispatchError>((
					authorities.len() as u32,
					authorities.get(i - 1).map(Clone::clone).unwrap_or(AUTHORITY_SENTINEL),
				))
			})?;

			Self::on_authorities_change(Method::SwapMembers { pre, old, new }, authorities_count);

			Ok(())
		}

		#[pallet::weight(10_000_000)]
		pub fn submit_authorities_change_signature(
			origin: OriginFor<T>,
			address: Address,
			signature: Signature,
		) -> DispatchResult {
			ensure_signed(origin)?;

			let authorities = Self::ensure_authority(&address)?;
			let mut authorities_change_to_sign =
				<AuthoritiesChangeToSign<T>>::take().ok_or(<Error<T>>::NoAuthoritiesChange)?;
			let (message, collected) = &mut authorities_change_to_sign;

			ensure!(
				Sign::verify_signature(&signature, message, &address),
				<Error<T>>::BadSignature
			);

			collected.try_push((address, signature)).map_err(|_| <Error<T>>::TooManyAuthorities)?;

			if Perbill::from_rational(collected.len() as u32, authorities.len() as u32)
				>= T::SignThreshold::get()
			{
				let authorities_change_to_sign =
					(authorities_change_to_sign.0, authorities_change_to_sign.1.to_vec());

				Self::deposit_event(<Event<T>>::CollectedEnoughAuthoritiesChangeSignatures(
					authorities_change_to_sign,
				));
			} else {
				<AuthoritiesChangeToSign<T>>::put(authorities_change_to_sign);
			}

			Ok(())
		}

		#[pallet::weight(10_000_000)]
		pub fn submit_new_message_root_signature(
			origin: OriginFor<T>,
			address: Address,
			signature: Signature,
		) -> DispatchResult {
			ensure_signed(origin)?;

			let authorities = Self::ensure_authority(&address)?;
			let mut new_message_root_to_sign =
				<NewMessageRootToSign<T>>::take().ok_or(<Error<T>>::NoNewMessageRoot)?;
			let (message, collected) = &mut new_message_root_to_sign;

			ensure!(
				Sign::verify_signature(&signature, message, &address),
				<Error<T>>::BadSignature
			);

			collected.try_push((address, signature)).map_err(|_| <Error<T>>::TooManyAuthorities)?;

			if Perbill::from_rational(collected.len() as u32, authorities.len() as u32)
				>= T::SignThreshold::get()
			{
				let new_message_root_to_sign =
					(new_message_root_to_sign.0, new_message_root_to_sign.1.to_vec());

				Self::deposit_event(<Event<T>>::CollectedEnoughNewMessageRootSignatures(
					new_message_root_to_sign,
				));
			} else {
				<NewMessageRootToSign<T>>::put(new_message_root_to_sign);
			}

			Ok(())
		}
	}
	impl<T: Config> Pallet<T> {
		fn network_id() -> NetworkId {
			network_ids::convert(T::Version::get().spec_name.as_ref())
		}

		fn ensure_authority(
			address: &Address,
		) -> Result<BoundedVec<Address, T::MaxAuthorities>, DispatchError> {
			let authorities = <Authorities<T>>::get();

			ensure!(authorities.iter().any(|a| a == address), <Error<T>>::NotAuthority);

			Ok(authorities)
		}

		fn ensure_not_on_authorities_change() -> DispatchResult {
			ensure!(!<AuthoritiesChangeToSign<T>>::exists(), <Error<T>>::OnAuthoritiesChange);

			Ok(())
		}

		fn on_authorities_change(method: Method, authorities_count: u32) {
			let authorities_changes = {
				let require_sign_count = (T::SignThreshold::get() * authorities_count).into();

				match method {
					Method::AddMember { new } => ethabi::encode(&[
						Token::Address(new.into()),
						Token::Uint(require_sign_count),
					]),
					Method::RemoveMember { pre, old } => ethabi::encode(&[
						Token::Address(pre.into()),
						Token::Address(old.into()),
						Token::Uint(require_sign_count),
					]),
					Method::SwapMembers { pre, old, new } => ethabi::encode(&[
						Token::Address(pre.into()),
						Token::Address(old.into()),
						Token::Address(new.into()),
						Token::Uint(require_sign_count),
					]),
				}
			};
			let nonce = <Nonce<T>>::mutate(|nonce| {
				*nonce += 1;

				*nonce
			});
			let message = Sign::hash(&ethabi::encode(&[
				Token::Bytes(RELAY_TYPE_HASH.as_ref().into()),
				Token::Bytes(Self::network_id().into()),
				Token::Bytes(method.id().into()),
				Token::Bytes(authorities_changes),
				Token::Uint(nonce.into()),
			]));

			<AuthoritiesChangeToSign<T>>::put((message, BoundedVec::default()));

			Self::deposit_event(<Event<T>>::CollectingAuthoritiesChangeSignature(message));
		}

		fn try_update_message_root(at: T::BlockNumber) -> Option<Hash> {
			let message_root = if let Some(message_root) = T::MessageRoot::get() {
				message_root
			} else {
				return None;
			};

			<PreviousMessageRoot<T>>::try_mutate(|(recorded_at, previous_message_root)| {
				// if this is a new root
				if &message_root != previous_message_root {
					// use a new root if reach the max pending period
					if at.saturating_sub(*recorded_at) > T::MaxPendingPeriod::get() {
						*previous_message_root = message_root.clone();

						return Ok(message_root);
					}
				}

				Err(())
			})
			.ok()
		}

		fn on_new_message_root(message_root: Hash) {
			let message = Sign::hash(&ethabi::encode(&[
				Token::Bytes(COMMIT_TYPE_HASH.as_ref().into()),
				Token::Bytes(Self::network_id().into()),
				Token::Bytes(message_root.as_ref().into()),
				Token::Uint(<Nonce<T>>::get().into()),
			]));

			<NewMessageRootToSign<T>>::put((message, BoundedVec::default()));

			Self::deposit_event(<Event<T>>::CollectingNewMessageRootSignature(message));
		}
	}
}
pub use pallet::*;
