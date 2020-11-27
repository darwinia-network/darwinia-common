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
	pub type RingBalance<T, I> = <RingCurrency<T, I> as Currency<AccountId<T>>>::Balance;
	pub type RingCurrency<T, I> = <T as Trait<I>>::RingCurrency;

	pub type Signer<T, I> = <<T as Trait<I>>::BackableChain as Backable>::Signer;
	pub type RelayAuthorityT<T, I> =
		RelayAuthority<AccountId<T>, Signer<T, I>, RingBalance<T, I>, BlockNumber<T>>;
}

// --- substrate ---
use frame_support::{
	decl_error, decl_module, decl_storage, ensure,
	traits::{Currency, EnsureOrigin, Get, LockIdentifier},
	StorageValue,
};
use frame_system::ensure_signed;
use sp_runtime::{DispatchError, DispatchResult};
// --- darwinia ---
use darwinia_relay_primitives::relay_authorities::*;
use darwinia_support::balance::lock::*;
use types::*;

pub trait Trait<I: Instance = DefaultInstance>: frame_system::Trait {
	type RingCurrency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

	type LockId: Get<LockIdentifier>;

	type TermDuration: Get<BlockNumber<Self>>;

	type MaxCandidates: Get<usize>;

	type BackableChain: Backable;

	type AddOrigin: EnsureOrigin<Self::Origin>;

	type RemoveOrigin: EnsureOrigin<Self::Origin>;

	type ResetOrigin: EnsureOrigin<Self::Origin>;

	type WeightInfo: WeightInfo;
}

pub trait WeightInfo {}
impl WeightInfo for () {}

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
		/// Authority - REQUIRED
		AuthorityR,
		/// Bond - INSUFFICIENT
		BondIns,
	}
}

decl_storage! {
	trait Store for Module<T: Trait<I>, I: Instance = DefaultInstance> as DarwiniaRelayAuthorities {
		pub Candidates get(fn candidates): Vec<RelayAuthorityT<T, I>>;
		pub Authorities get(fn authorities): Vec<RelayAuthorityT<T, I>>;
	}
}

decl_module! {
	pub struct Module<T: Trait<I>, I: Instance = DefaultInstance> for enum Call
	where
		origin: T::Origin
	{
		type Error = Error<T, I>;

		const LOCK_ID: LockIdentifier = T::LockId::get();

		#[weight = 10_000_000]
		pub fn request_authority(
			origin,
			bond: RingBalance<T, I>,
			signer: Signer<T, I>,
		) {
			let account_id = ensure_signed(origin)?;

			ensure!(
				find_authority::<T, I>(&<Authorities<T, I>>::get(), &account_id).is_none(),
				<Error<T, I>>::AuthorityAE
			);
			ensure!(
				<RingCurrency<T, I>>::usable_balance(&account_id) > bond,
				<Error<T, I>>::BondIns
			);

			<Candidates<T, I>>::try_mutate(|candidates| {
				ensure!(
					find_authority::<T, I>(candidates, &account_id).is_none(),
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

		#[weight = 10_000_000]
		pub fn sign(origin) {
			let account_id = ensure_signed(origin)?;
			let authority = find_authority::<T, I>(&account_id).ok_or(<Error<T>>::AuthorityR)?;

			// TODO
		}
	}
}

impl<T: Trait<I>, I: Instance> Module<T, I> {
	pub fn remove_authority_by_id(
		account_id: &AccountId<T>,
	) -> Result<RelayAuthorityT<T, I>, DispatchError> {
		Ok(<Authorities<T, I>>::try_mutate(|authorities| {
			if let Some(position) = find_authority::<T, I>(&authorities, &account_id) {
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
			if let Some(position) = find_authority::<T, I>(&candidates, &account_id) {
				let candidate = candidates.remove(position);

				<RingCurrency<T, I>>::remove_lock(T::LockId::get(), &account_id);

				Ok(candidate)
			} else {
				Err(<Error<T, I>>::CandidateNE)
			}
		})?)
	}
}

pub fn find_authority<T, I>(
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
