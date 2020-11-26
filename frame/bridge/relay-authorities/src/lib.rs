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
	pub type RelayAuthorityT<T, I> = RelayAuthority<AccountId<T>, Signer<T, I>>;
}

// --- substrate ---
use frame_support::{
	decl_error, decl_module, decl_storage, ensure,
	traits::{Currency, EnsureOrigin, Get, LockIdentifier},
};
use frame_system::{ensure_root, ensure_signed};
use sp_runtime::DispatchResult;
// --- darwinia ---
use darwinia_relay_primitives::relay_authorities::*;
use darwinia_support::balance::lock::*;
use types::*;

pub trait Trait<I: Instance = DefaultInstance>: frame_system::Trait {
	type RingCurrency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

	type LockId: Get<LockIdentifier>;

	type TermDuration: Get<Self::BlockNumber>;

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
				<Candidates<T, I>>::get()
					.into_iter()
					.position(|relay_authority| relay_authority.account_id == account_id)
					.is_some(),
				<Error<T, I>>::CandidateAE
			);
			ensure!(
				<RingCurrency<T, I>>::usable_balance(&account_id) > bond,
				<Error<T, I>>::BondIns
			);

			<RingCurrency<T, I>>::set_lock(
				T::LockId::get(),
				&account_id,
				LockFor::Common { amount: bond },
				WithdrawReasons::all()
			);
			<Candidates<T, I>>::append(RelayAuthority { account_id, signer });
		}

		#[weight = 10_000_000]
		pub fn approve_authority(origin, candidate_index: u32) {}

		#[weight = 10_000_000]
		pub fn remove_authority(origin, authority_index: u32) {}

		#[weight = 10_000_000]
		pub fn reset_authorities(origin, authorities: Vec<RelayAuthorityT<T, I>>) {
			ensure_root(origin)?;

			<Authorities<T, I>>::put(authorities);
		}
	}
}

impl<T: Trait<I>, I: Instance> Module<T, I> {}
