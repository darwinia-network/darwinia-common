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

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod primitives;
use primitives::*;

mod weights;
pub use weights::WeightInfo;

// --- crates.io ---
use ethabi::Token;
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
		/// The maximum number of authorities.
		#[pallet::constant]
		type MaxAuthorities: Get<u32>;
		// Commitment relates.
		/// Chain's ID, which is using for constructing the message. (follow EIP-712 SPEC)
		#[pallet::constant]
		type ChainId: Get<&'static [u8]>;
		/// The signing threshold.
		///
		/// Once `signatures_count / authorities_count >= threshold`, we say the message is trusted.
		#[pallet::constant]
		type SignThreshold: Get<Perbill>;
		/// The Darwinia message root.
		///
		/// If it changed, it means there are some new messages which are waiting for relaying.
		type MessageRoot: Get<Option<Hash>>;
		// Checkpoints.
		// `SyncInterval` must be shorter than `MaxPendingPeriod`.
		/// The interval of checking the message root.
		#[pallet::constant]
		type SyncInterval: Get<Self::BlockNumber>;
		/// How long should we wait for the message root to be signed.
		///
		/// If the collecting new message root signatures process takes more than
		/// `MaxPendingPeriod`, we will drop the root. And update the root with a new one.
		#[pallet::constant]
		type MaxPendingPeriod: Get<Self::BlockNumber>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Authorities changed. Collecting authorities change signatures. \[Message\]
		CollectingAuthoritiesChangeSignatures(Message),
		/// Collected enough authorities change signatures. \[(Message, Vec<Address, Signature>)\]
		CollectedEnoughAuthoritiesChangeSignatures((Message, Vec<(Address, Signature)>)),

		/// New message root found. Collecting new message root signatures. \[Message\]
		CollectingNewMessageRootSignatures(Message),
		/// Collected enough new message root signatures. \[(Message, Vec<Address, Signature>)\]
		CollectedEnoughNewMessageRootSignatures((Message, Vec<(Address, Signature)>)),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The authority is already existed.
		AuthorityExisted,
		/// Too many authorities.
		TooManyAuthorities,
		/// This is not an authority.
		NotAuthority,
		/// This is not a previous authority.
		/// Only the old authority can sign for the new authorities set.
		NotPreviousAuthority,
		/// Require at least one authority. Not allow to decrease below one.
		AtLeastOneAuthority,
		/// Currently, the authorities is changing.
		OnAuthoritiesChange,
		/// Didn't find any authorities changes to sign.
		NoAuthoritiesChange,
		/// Didn't find any new message root to sign.
		NoNewMessageRoot,
		/// Failed to verify the signature.
		BadSignature,
	}

	/// Record the previous authorities.
	///
	/// When the `Authorities` changed, the members of previous authorities need to submit the
	/// signatures, and only they could do this.
	#[pallet::storage]
	#[pallet::getter(fn previous_authorities)]
	pub type PreviousAuthorities<T: Config> =
		StorageValue<_, BoundedVec<Address, T::MaxAuthorities>, ValueQuery>;

	/// The current active authorities.
	#[pallet::storage]
	#[pallet::getter(fn authorities)]
	pub type Authorities<T: Config> =
		StorageValue<_, BoundedVec<Address, T::MaxAuthorities>, ValueQuery>;

	/// The nonce of the current active authorities. AKA term/session/era.
	#[pallet::storage]
	#[pallet::getter(fn nonce)]
	pub type Nonce<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// The authorities change waiting for signing.
	#[pallet::storage]
	#[pallet::getter(fn authorities_change_to_sign)]
	pub type AuthoritiesChangeToSign<T: Config> = StorageValue<
		_,
		(Message, BoundedVec<(Address, Signature), T::MaxAuthorities>),
		OptionQuery,
	>;

	/// The new message root waiting for signing.
	#[pallet::storage]
	#[pallet::getter(fn new_message_root_to_sign)]
	pub type NewMessageRootToSign<T: Config> = StorageValue<
		_,
		(Message, BoundedVec<(Address, Signature), T::MaxAuthorities>),
		OptionQuery,
	>;

	/// Record the previous message root.
	///
	/// Use for checking if the message root getter get the same message root as the previous one.
	/// And if this is empty, it means the message root is require to be relayed.
	#[pallet::storage]
	#[pallet::getter(fn previous_message_root)]
	pub type PreviousMessageRoot<T: Config> = StorageValue<_, (T::BlockNumber, Hash), OptionQuery>;

	#[derive(Default)]
	#[pallet::genesis_config]
	pub struct GenesisConfig {
		pub authorities: Vec<Address>,
	}
	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {
			<PreviousAuthorities<T>>::put(BoundedVec::try_from(self.authorities.clone()).unwrap());
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
		/// Add a authority and trigger `on_authorities_change`.
		///
		/// Not allow to call while authorities is changing.
		/// This will insert new authority into the index 0 of authorities.
		#[pallet::weight(10_000_000)]
		#[frame_support::transactional]
		pub fn add_authority(origin: OriginFor<T>, new: Address) -> DispatchResult {
			ensure_root(origin)?;

			Self::ensure_not_on_authorities_change()?;

			let authorities_count = <Authorities<T>>::try_mutate(|authorities| {
				if authorities.contains(&new) {
					return Err(<Error<T>>::AuthorityExisted)?;
				}

				<PreviousAuthorities<T>>::put(&*authorities);
				authorities.try_insert(0, new).map_err(|_| <Error<T>>::TooManyAuthorities)?;

				Ok::<_, DispatchError>(authorities.len() as u32)
			})?;

			Self::on_authorities_change(Method::AddMember { new }, authorities_count);

			Ok(())
		}

		/// Remove a authority and trigger `on_authorities_change`.
		///
		/// Not allow to call while authorities is changing.
		#[pallet::weight(10_000_000)]
		#[frame_support::transactional]
		pub fn remove_authority(origin: OriginFor<T>, old: Address) -> DispatchResult {
			ensure_root(origin)?;

			Self::ensure_not_on_authorities_change()?;

			let (authorities_count, pre) = <Authorities<T>>::try_mutate(|authorities| {
				let i =
					authorities.iter().position(|a| a == &old).ok_or(<Error<T>>::NotAuthority)?;

				if authorities.len() == 1 {
					return Err(<Error<T>>::AtLeastOneAuthority)?;
				}

				<PreviousAuthorities<T>>::put(&*authorities);
				authorities.remove(i);

				Ok::<_, DispatchError>((
					authorities.len() as u32,
					if i == 0 { AUTHORITY_SENTINEL } else { authorities[i - 1] },
				))
			})?;

			Self::on_authorities_change(Method::RemoveMember { pre, old }, authorities_count);

			Ok(())
		}

		/// Swap the old authority with the new authority and trigger `on_authorities_change`.
		///
		/// Not allow to call while authorities is changing.
		#[pallet::weight(10_000_000)]
		#[frame_support::transactional]
		pub fn swap_authority(origin: OriginFor<T>, old: Address, new: Address) -> DispatchResult {
			ensure_root(origin)?;

			Self::ensure_not_on_authorities_change()?;

			let (authorities_count, pre) = <Authorities<T>>::try_mutate(|authorities| {
				let i =
					authorities.iter().position(|a| a == &old).ok_or(<Error<T>>::NotAuthority)?;

				<PreviousAuthorities<T>>::put(&*authorities);
				authorities[i] = new;

				Ok::<_, DispatchError>((
					authorities.len() as u32,
					if i == 0 { AUTHORITY_SENTINEL } else { authorities[i - 1] },
				))
			})?;

			Self::on_authorities_change(Method::SwapMembers { pre, old, new }, authorities_count);

			Ok(())
		}

		/// Submit the authorities change signature.
		#[pallet::weight(10_000_000)]
		#[frame_support::transactional]
		pub fn submit_authorities_change_signature(
			origin: OriginFor<T>,
			address: Address,
			signature: Signature,
		) -> DispatchResult {
			ensure_signed(origin)?;

			let previous_authorities = Self::ensure_previous_authority(&address)?;
			// Take the value here.
			// If collected enough signatures, leave the empty `AuthoritiesChangeToSign` in storage.
			let mut authorities_change_to_sign =
				<AuthoritiesChangeToSign<T>>::take().ok_or(<Error<T>>::NoAuthoritiesChange)?;
			let (message, collected) = &mut authorities_change_to_sign;

			ensure!(
				Sign::verify_signature(&signature, message, &address),
				<Error<T>>::BadSignature
			);

			collected.try_push((address, signature)).map_err(|_| <Error<T>>::TooManyAuthorities)?;

			if Perbill::from_rational(collected.len() as u32, previous_authorities.len() as u32)
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

		/// Submit the new message root signature.
		#[pallet::weight(10_000_000)]
		#[frame_support::transactional]
		pub fn submit_new_message_root_signature(
			origin: OriginFor<T>,
			address: Address,
			signature: Signature,
		) -> DispatchResult {
			ensure_signed(origin)?;

			let authorities = Self::ensure_authority(&address)?;
			// Take the value here.
			// If collected enough signatures, leave the empty `NewMessageRootToSign` in storage
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
		fn ensure_previous_authority(
			address: &Address,
		) -> Result<BoundedVec<Address, T::MaxAuthorities>, DispatchError> {
			let previous_authorities = <PreviousAuthorities<T>>::get();

			ensure!(
				previous_authorities.iter().any(|a| a == address),
				<Error<T>>::NotPreviousAuthority
			);

			Ok(previous_authorities)
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
				match method {
					Method::AddMember { new } => ethabi::encode(&[
						Token::Address(new.into()),
						Token::Uint((T::SignThreshold::get() * authorities_count).into()),
					]),
					Method::RemoveMember { pre, old } => ethabi::encode(&[
						Token::Address(pre.into()),
						Token::Address(old.into()),
						Token::Uint((T::SignThreshold::get() * authorities_count).into()),
					]),
					Method::SwapMembers { pre, old, new } => ethabi::encode(&[
						Token::Address(pre.into()),
						Token::Address(old.into()),
						Token::Address(new.into()),
					]),
				}
			};
			let message = <Nonce<T>>::mutate(|nonce| {
				let message = Sign::eth_signable_message(
					T::ChainId::get(),
					T::Version::get().spec_name.as_ref(),
					&ethabi::encode(&[
						Token::FixedBytes(RELAY_TYPE_HASH.as_ref().into()),
						Token::FixedBytes(method.id().into()),
						Token::Bytes(authorities_changes),
						Token::Uint((*nonce).into()),
					]),
				);

				*nonce += 1;

				message
			});

			<AuthoritiesChangeToSign<T>>::put((message, BoundedVec::default()));

			Self::deposit_event(<Event<T>>::CollectingAuthoritiesChangeSignatures(message));
		}

		fn try_update_message_root(at: T::BlockNumber) -> Option<Hash> {
			if Self::ensure_not_on_authorities_change().is_err() {
				// Not allow to relay the messages if the new authorities set is not verified.
				return None;
			}

			let message_root = if let Some(message_root) = T::MessageRoot::get() {
				message_root
			} else {
				return None;
			};

			<PreviousMessageRoot<T>>::try_mutate(|maybe_previous_message_root| {
				if let Some((recorded_at, previous_message_root)) = maybe_previous_message_root {
					// Only if the chain is still collecting signatures will enter this condition.

					// If this is a new root.
					if &message_root != previous_message_root {
						// Update the root with a new one if exceed the max pending period.
						// Also update the recorded time.
						if at.saturating_sub(*recorded_at) > T::MaxPendingPeriod::get() {
							*recorded_at = at;
							*previous_message_root = message_root.clone();

							return Ok(message_root);
						}
					}
				} else {
					// If no previous message root is recorded, starting to relay the incoming
					// messages.
					*maybe_previous_message_root = Some((at, message_root.clone()));

					return Ok(message_root);
				}

				Err(())
			})
			.ok()
		}

		fn on_new_message_root(message_root: Hash) {
			let message = Sign::eth_signable_message(
				T::ChainId::get(),
				T::Version::get().spec_name.as_ref(),
				&ethabi::encode(&[
					Token::FixedBytes(COMMIT_TYPE_HASH.as_ref().into()),
					Token::FixedBytes(message_root.as_ref().into()),
					Token::Uint(<Nonce<T>>::get().into()),
				]),
			);

			<NewMessageRootToSign<T>>::put((message, BoundedVec::default()));

			Self::deposit_event(<Event<T>>::CollectingNewMessageRootSignatures(message));
		}
	}
}
pub use pallet::*;
