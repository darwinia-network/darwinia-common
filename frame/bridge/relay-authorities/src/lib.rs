// This file is part of Darwinia.
//
// Copyright (C) 2018-2020 Darwinia Network
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
// along with Darwinia.  If not, see <https://www.gnu.org/licenses/>.

//! # Ecdsa Authorities Module

#![cfg_attr(not(feature = "std"), no_std)]

mod types {
	// --- darwinia ---
	use crate::*;

	pub type AccountId<T> = <T as frame_system::Trait>::AccountId;
	pub type BlockNumber<T> = <T as frame_system::Trait>::BlockNumber;
	pub type MMRRoot<T> = <T as frame_system::Trait>::Hash;
	pub type RingBalance<T, I> = <RingCurrency<T, I> as Currency<AccountId<T>>>::Balance;
	pub type RingCurrency<T, I> = <T as Trait<I>>::RingCurrency;

	pub type Signer<T, I> = <<T as Trait<I>>::Sign as Sign<BlockNumber<T>>>::Signer;
	pub type RelaySignature<T, I> = <<T as Trait<I>>::Sign as Sign<BlockNumber<T>>>::Signature;
	pub type RelayAuthorityT<T, I> =
		RelayAuthority<AccountId<T>, Signer<T, I>, RingBalance<T, I>, BlockNumber<T>>;
}

// --- substrate ---
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage, ensure,
	traits::{Currency, EnsureOrigin, Get, LockIdentifier},
	StorageValue,
};
use frame_system::ensure_signed;
use sp_runtime::{DispatchError, DispatchResult, Perbill};
// --- darwinia ---
use darwinia_relay_primitives::relay_authorities::*;
use darwinia_support::balance::lock::*;
use types::*;

pub trait Trait<I: Instance = DefaultInstance>: frame_system::Trait {
	type Event: From<Event<Self, I>> + Into<<Self as frame_system::Trait>::Event>;

	type RingCurrency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

	type LockId: Get<LockIdentifier>;

	type TermDuration: Get<Self::BlockNumber>;

	type MaxCandidates: Get<usize>;

	type AddOrigin: EnsureOrigin<Self::Origin>;

	type RemoveOrigin: EnsureOrigin<Self::Origin>;

	type ResetOrigin: EnsureOrigin<Self::Origin>;

	type Sign: Sign<Self::BlockNumber>;

	type ApproveThreshold: Get<Perbill>;

	type WeightInfo: WeightInfo;
}

pub trait WeightInfo {}
impl WeightInfo for () {}

decl_event! {
	pub enum Event<T, I: Instance = DefaultInstance>
	where
		MMRRoot = MMRRoot<T>,
		RelaySignature = RelaySignature<T, I>,
	{
		SignedMMRRoot(MMRRoot, Vec<RelaySignature>),
	}
}

decl_error! {
	pub enum Error for Module<T: Trait<I>, I: Instance> {
		/// Candidate - ALREADY EXISTED
		CandidateAE,
		/// Candidate - NOT EXISTED
		CandidateNE,
		/// Authority - ALREADY EXISTED
		AuthorityAE,
		/// Authority - NOT EXISTED
		AuthorityNE,
		/// Authority - IN TERM
		AuthorityIT,
		/// Bond - INSUFFICIENT
		BondIns,
		/// On Member Change - DISABLED
		OnMemberChangeDis,
		/// MMR Root -NOT EXISTED
		MMRRootNE,
		/// Signature - INVALID
		SignatureInv,
	}
}

decl_storage! {
	trait Store for Module<T: Trait<I>, I: Instance = DefaultInstance> as DarwiniaRelayAuthorities {
		pub Candidates get(fn candidates): Vec<RelayAuthorityT<T, I>>;
		pub Authorities get(fn authorities): Vec<RelayAuthorityT<T, I>>;

		pub MMRRootsToSign
			get(fn mmr_root_to_sign_of)
			: map hasher(identity) MMRRoot<T>
			=> Option<Vec<RelaySignature<T, I>>>;

		pub OnMemberChange get(fn on_member_change): bool;
	}
}

decl_module! {
	pub struct Module<T: Trait<I>, I: Instance = DefaultInstance> for enum Call
	where
		origin: T::Origin
	{
		type Error = Error<T, I>;

		const LOCK_ID: LockIdentifier = T::LockId::get();

		fn deposit_event() = default;

		#[weight = 10_000_000]
		pub fn request_authority(
			origin,
			bond: RingBalance<T, I>,
			signer: Signer<T, I>,
		) {
			let account_id = ensure_signed(origin)?;

			ensure!(
				find_authority_position::<T, I>(&<Authorities<T, I>>::get(), &account_id).is_none(),
				<Error<T, I>>::AuthorityAE
			);
			ensure!(
				<RingCurrency<T, I>>::usable_balance(&account_id) > bond,
				<Error<T, I>>::BondIns
			);

			<Candidates<T, I>>::try_mutate(|candidates| {
				ensure!(
					find_authority_position::<T, I>(candidates, &account_id).is_none(),
					<Error<T, I>>::CandidateAE
				);

				if candidates.len() == T::MaxCandidates::get() {
					ensure!(
						bond >
							candidates
								.iter()
								.map(|candidate| candidate.bond)
								.max()
								.unwrap_or(0.into()),
						<Error<T, I>>::BondIns
					);

					// slash the weed out?
					let weep_out = candidates.pop().unwrap();

					<RingCurrency<T, I>>::remove_lock(T::LockId::get(), &weep_out.account_id);
				}

				<RingCurrency<T, I>>::set_lock(
					T::LockId::get(),
					&account_id,
					LockFor::Common { amount: bond },
					WithdrawReasons::all()
				);

				candidates.push(RelayAuthority {
					account_id,
					signer,
					bond,
					term: 0.into()
				});

				DispatchResult::Ok(())
			})?;
		}

		// No-op if can't find
		#[weight = 10_000_000]
		pub fn cancel_request(origin) {
			let account_id = ensure_signed(origin)?;
			let _ = Self::remove_candidate_by_id(&account_id);
		}

		// No-op if can't find
		#[weight = 10_000_000]
		pub fn renounce_authority(origin) {
			let account_id = ensure_signed(origin)?;

			<Authorities<T, I>>::try_mutate(|authorities| {
				if let Some(position) = authorities
					.iter()
					.position(|authority| authority == &account_id)
				{
					if authorities[position].term <= <frame_system::Module<T>>::block_number() {
						return Ok(authorities.remove(position));
					}
				}

				Err(<Error<T, I>>::AuthorityIT)
			})?;

			// TODO on authorities changed
		}

		#[weight = 10_000_000]
		pub fn add_authority(origin, account_id: AccountId<T>) {
			T::AddOrigin::ensure_origin(origin)?;

			let mut authority = Self::remove_authority_by_id(&account_id)?;
			authority.term = <frame_system::Module<T>>::block_number() + T::TermDuration::get();

			// Won't check duplicated here, MUST make this authority sure is unique
			// As we already make a check in `request_authority`
			<Authorities<T, I>>::append(authority);

			// TODO on authorities changed
		}

		// No-op if can't find
		#[weight = 10_000_000]
		pub fn remove_authority(origin, account_id: AccountId<T>) {
			T::RemoveOrigin::ensure_origin(origin)?;

			let _ = Self::remove_authority_by_id(&account_id);

			// TODO on authorities changed
		}

		#[weight = 10_000_000]
		pub fn kill_candidates(origin) {
			T::ResetOrigin::ensure_origin(origin)?;

			let lock_id = T::LockId::get();

			for RelayAuthority { account_id, .. } in <Candidates<T, I>>::take() {
				<RingCurrency<T, I>>::remove_lock(lock_id, &account_id);
			}
		}

		#[weight = 10_000_000]
		pub fn reset_authorities(origin, authorities: Vec<RelayAuthorityT<T, I>>) {
			T::ResetOrigin::ensure_origin(origin)?;

			<Authorities<T, I>>::put(authorities);

			// TODO on authorities changed
		}

		// No-op if already submitted
		#[weight = 10_000_000]
		pub fn submit_mmr_root_signature(
			origin,
			mmr_root: MMRRoot<T>,
			signature: RelaySignature<T, I>
		) {
			let authority = ensure_signed(origin)?;

			ensure!(!<OnMemberChange<I>>::get(), <Error<T, I>>::OnMemberChangeDis);

			let mut signatures = <MMRRootsToSign<T, I>>::get(&mmr_root).ok_or(<Error<T, I>>::MMRRootNE)?;

			if signatures.contains(&signature) {
				return Ok(());
			}

			let authorities = <Authorities<T, I>>::get();
			let signer = find_signer::<T, I>(
				&authorities,
				&authority
			).ok_or(<Error<T, I>>::AuthorityNE)?;

			ensure!(
				T::Sign::verify_signature(&signature, mmr_root, signer),
				 <Error<T, I>>::SignatureInv
			);

			signatures.push(signature);

			if Perbill::from_rational_approximation(signatures.len() as u32 + 1, authorities.len() as _)
				>= T::ApproveThreshold::get()
			{
				<MMRRootsToSign<T, I>>::remove(&mmr_root);

				// TODO: clean the mmr root which was contains in this mmr root?

				Self::deposit_event(RawEvent::SignedMMRRoot(mmr_root, signatures));
			} else {
				<MMRRootsToSign<T, I>>::insert(&mmr_root, signatures);
			}
		}

		#[weight = 10_000_000]
		pub fn submit_member_set_signature(origin, signature: RelaySignature<T, I>) {
			let authority = ensure_signed(origin)?;

			ensure!(<OnMemberChange<I>>::get(), <Error<T, I>>::OnMemberChangeDis);
		}
	}
}

impl<T, I> Module<T, I>
where
	T: Trait<I>,
	I: Instance,
{
	pub fn remove_authority_by_id(
		account_id: &AccountId<T>,
	) -> Result<RelayAuthorityT<T, I>, DispatchError> {
		Ok(<Authorities<T, I>>::try_mutate(|authorities| {
			if let Some(position) = find_authority_position::<T, I>(&authorities, &account_id) {
				let authority = authorities.remove(position);

				<RingCurrency<T, I>>::remove_lock(T::LockId::get(), &account_id);

				Ok(authority)
			} else {
				Err(<Error<T, I>>::AuthorityNE)
			}
		})?)
	}

	pub fn remove_candidate_by_id(
		account_id: &AccountId<T>,
	) -> Result<RelayAuthorityT<T, I>, DispatchError> {
		Ok(<Candidates<T, I>>::try_mutate(|candidates| {
			if let Some(position) = find_authority_position::<T, I>(&candidates, &account_id) {
				let candidate = candidates.remove(position);

				<RingCurrency<T, I>>::remove_lock(T::LockId::get(), &account_id);

				Ok(candidate)
			} else {
				Err(<Error<T, I>>::CandidateNE)
			}
		})?)
	}
}

impl<T, I> RelayAuthorityProtocol<MMRRoot<T>> for Module<T, I>
where
	T: Trait<I>,
	I: Instance,
{
	fn new_mmr_to_sign(mmr_root: MMRRoot<T>) {
		if <MMRRootsToSign<T, I>>::get(&mmr_root).is_none() {
			<MMRRootsToSign<T, I>>::insert(mmr_root, <Vec<RelaySignature<T, I>>>::new());
		}
	}
}

pub fn find_authority_position<T, I>(
	authorities: &[RelayAuthorityT<T, I>],
	account_id: &AccountId<T>,
) -> Option<usize>
where
	T: Trait<I>,
	I: Instance,
{
	authorities
		.iter()
		.position(|relay_authority| relay_authority == account_id)
}

pub fn find_signer<T, I>(
	authorities: &[RelayAuthorityT<T, I>],
	account_id: &AccountId<T>,
) -> Option<Signer<T, I>>
where
	T: Trait<I>,
	I: Instance,
{
	if let Some(position) = authorities
		.iter()
		.position(|relay_authority| relay_authority == account_id)
	{
		Some(authorities[position].signer.to_owned())
	} else {
		None
	}
}
