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

//! # Relay Authorities Module

#![cfg_attr(not(feature = "std"), no_std)]

mod types {
	// --- darwinia ---
	use crate::*;

	pub type AccountId<T> = <T as frame_system::Trait>::AccountId;
	pub type BlockNumber<T> = <T as frame_system::Trait>::BlockNumber;
	pub type Hash<T> = <T as frame_system::Trait>::Hash;
	pub type MMRRoot<T> = Hash<T>;
	pub type RingBalance<T, I> = <RingCurrency<T, I> as Currency<AccountId<T>>>::Balance;
	pub type RingCurrency<T, I> = <T as Trait<I>>::RingCurrency;

	pub type Signer<T, I> = <<T as Trait<I>>::Sign as Sign<BlockNumber<T>>>::Signer;
	pub type RelaySignature<T, I> = <<T as Trait<I>>::Sign as Sign<BlockNumber<T>>>::Signature;
	pub type RelayAuthorityT<T, I> =
		RelayAuthority<AccountId<T>, Signer<T, I>, RingBalance<T, I>, BlockNumber<T>>;
}

// --- crates ---
use codec::Encode;
// --- substrate ---
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage, ensure,
	traits::{Currency, EnsureOrigin, Get, LockIdentifier},
	weights::Weight,
	StorageValue,
};
use frame_system::ensure_signed;
use sp_runtime::{
	traits::{Hash as HashT, Saturating},
	DispatchError, DispatchResult, Perbill,
};
#[cfg(not(feature = "std"))]
use sp_std::borrow::ToOwned;
use sp_std::prelude::*;
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

	type SignThreshold: Get<Perbill>;

	type SubmitDuration: Get<Self::BlockNumber>;

	type WeightInfo: WeightInfo;
}

pub trait WeightInfo {}
impl WeightInfo for () {}

decl_event! {
	pub enum Event<T, I: Instance = DefaultInstance>
	where
		AccountId = AccountId<T>,
		Hash = Hash<T>,
		MMRRoot = MMRRoot<T>,
		RelaySignature = RelaySignature<T, I>,
	{
		SignedMMRRoot(MMRRoot, Vec<(AccountId, RelaySignature)>),
		SignedAuthoritySet(Hash, Vec<(AccountId, RelaySignature)>),
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
		/// Stake - INSUFFICIENT
		StakeIns,
		/// On Authorities Change - DISABLED
		OnAuthoritiesChangeDis,
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
		pub OldAuthorities get(fn old_authorities): Vec<RelayAuthorityT<T, I>>;

		// (
		// 	is on authorities change,
		// 	signature submit deadline,
		// )
		pub AuthoritiesState get(fn authorities_state): (bool, BlockNumber<T>) = (false, 0.into());

		pub MMRRootsToSign get(fn mmr_roots_to_sign): Vec<MMRRoot<T>>;

		pub SignedMMRRoots
			get(fn signed_mmr_root_of)
			: map hasher(identity) MMRRoot<T>
			=> Option<(BlockNumber<T>, Vec<(AccountId<T>, RelaySignature<T, I>)>)>;

		pub ClosedMMRRootSubmits
			get(fn closed_mmr_root_submit_of)
			: map hasher(identity) BlockNumber<T>
			=> Option<MMRRoot<T>>;

		pub AuthoritiesToSign
			get(fn authorities_to_sign)
			: (Hash<T>, Vec<(AccountId<T>, RelaySignature<T, I>)>);
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

		fn on_initialize(now: BlockNumber<T>) -> Weight {
			Self::check_submit_deadline(now);

			0
		}

		#[weight = 10_000_000]
		pub fn request_authority(
			origin,
			stake: RingBalance<T, I>,
			signer: Signer<T, I>,
		) {
			let account_id = ensure_signed(origin)?;

			ensure!(
				find_authority_position::<T, I>(&<Authorities<T, I>>::get(), &account_id).is_none(),
				<Error<T, I>>::AuthorityAE
			);
			ensure!(
				<RingCurrency<T, I>>::usable_balance(&account_id) > stake,
				<Error<T, I>>::StakeIns
			);

			<Candidates<T, I>>::try_mutate(|candidates| {
				ensure!(
					find_authority_position::<T, I>(candidates, &account_id).is_none(),
					<Error<T, I>>::CandidateAE
				);

				if candidates.len() == T::MaxCandidates::get() {
					ensure!(
						stake >
							candidates
								.iter()
								.map(|candidate| candidate.stake)
								.max()
								.unwrap_or(0.into()),
						<Error<T, I>>::StakeIns
					);

					// TODO: slash the weed out?
					let weep_out = candidates.pop().unwrap();

					<RingCurrency<T, I>>::remove_lock(T::LockId::get(), &weep_out.account_id);
				}

				<RingCurrency<T, I>>::set_lock(
					T::LockId::get(),
					&account_id,
					LockFor::Common { amount: stake },
					WithdrawReasons::all()
				);

				candidates.push(RelayAuthority {
					account_id,
					signer,
					stake,
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

			ensure!(!Self::on_authorities_change(), <Error<T, I>>::OnAuthoritiesChangeDis);

			Self::remove_authority_by_id_with(
				&account_id,
				|authority| if authority.term <= <frame_system::Module<T>>::block_number() {
					Some(<Error<T, I>>::AuthorityIT)
				} else {
					None
				}
			)?;
		}

		#[weight = 10_000_000]
		pub fn add_authority(origin, account_id: AccountId<T>) {
			T::AddOrigin::ensure_origin(origin)?;

			ensure!(!Self::on_authorities_change(), <Error<T, I>>::OnAuthoritiesChangeDis);

			let mut authority = Self::remove_candidate_by_id(&account_id)?;
			authority.term = <frame_system::Module<T>>::block_number() + T::TermDuration::get();

			// Won't check duplicated here, MUST make this authority sure is unique
			// As we already make a check in `request_authority`
			<Authorities<T, I>>::append(authority);
		}

		// No-op if can't find
		#[weight = 10_000_000]
		pub fn remove_authority(origin, account_id: AccountId<T>) {
			T::RemoveOrigin::ensure_origin(origin)?;

			ensure!(!Self::on_authorities_change(), <Error<T, I>>::OnAuthoritiesChangeDis);

			let _ = Self::remove_authority_by_id_with(&account_id, |_| None);
		}

		#[weight = 10_000_000]
		pub fn kill_candidates(origin) {
			T::ResetOrigin::ensure_origin(origin)?;

			let lock_id = T::LockId::get();

			for RelayAuthority { account_id, .. } in <Candidates<T, I>>::take() {
				<RingCurrency<T, I>>::remove_lock(lock_id, &account_id);
			}
		}

		// Dangerous!
		//
		// Authorities don't need to stake any asset
		//
		// This operation is forced to set the authorities,
		// without the authorities change signature requirement
		#[weight = 10_000_000]
		pub fn reset_authorities(origin, authorities: Vec<RelayAuthorityT<T, I>>) {
			T::ResetOrigin::ensure_origin(origin)?;

			ensure!(!Self::on_authorities_change(), <Error<T, I>>::OnAuthoritiesChangeDis);

			<Authorities<T, I>>::mutate(|old_authorities| {
				for authority in old_authorities.iter() {
					<RingCurrency<T, I>>::remove_lock(
						T::LockId::get(),
						&authority.account_id
					);
				}

				*old_authorities = authorities;
			});
		}

		// No-op if already submit
		#[weight = 10_000_000]
		pub fn submit_mmr_root_signature(
			origin,
			mmr_root: MMRRoot<T>,
			signature: RelaySignature<T, I>
		) {
			let authority = ensure_signed(origin)?;

			ensure!(!Self::on_authorities_change(), <Error<T, I>>::OnAuthoritiesChangeDis);

			let (deadline, mut signatures) =
				<SignedMMRRoots<T, I>>::get(&mmr_root).ok_or(<Error<T, I>>::MMRRootNE)?;

			if signatures.iter().position(|(authority_, _)| authority_ == &authority).is_some() {
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

			signatures.push((authority, signature));

			if Perbill::from_rational_approximation(signatures.len() as u32 + 1, authorities.len() as _)
				>= T::SignThreshold::get()
			{
				<MMRRootsToSign<T, I>>::mutate(|mmr_roots_to_sign|
					if let Some(position) = mmr_roots_to_sign
						.iter()
						.position(|mmr_root_| mmr_root_ == &mmr_root)
					{
						mmr_roots_to_sign.remove(position);
					}
				);
				<SignedMMRRoots<T, I>>::remove(&mmr_root);
				<ClosedMMRRootSubmits<T, I>>::remove(&deadline);

				// TODO: clean the mmr root which was contains in this mmr root?

				Self::deposit_event(RawEvent::SignedMMRRoot(mmr_root, signatures));
			} else {
				<SignedMMRRoots<T, I>>::insert(&mmr_root, (deadline, signatures));
			}
		}

		// No-op if already submit
		#[weight = 10_000_000]
		pub fn submit_authorities_signature(origin, signature: RelaySignature<T, I>) {
			let old_authority = ensure_signed(origin)?;

			ensure!(Self::on_authorities_change(), <Error<T, I>>::OnAuthoritiesChangeDis);

			let (hashed_authorities_set, mut signatures) = <AuthoritiesToSign<T, I>>::get();

			if signatures
				.iter()
				.position(|(old_authority_, _)| old_authority_ == &old_authority)
				.is_some()
			{
				return Ok(());
			}

			let old_authorities = <OldAuthorities<T, I>>::get();
			let signer = find_signer::<T, I>(
				&old_authorities,
				&old_authority
			).ok_or(<Error<T, I>>::AuthorityNE)?;

			ensure!(
				T::Sign::verify_signature(&signature, hashed_authorities_set, signer),
				 <Error<T, I>>::SignatureInv
			);

			signatures.push((old_authority, signature));

			if Perbill::from_rational_approximation(signatures.len() as u32 + 1, old_authorities.len() as _)
				>= T::SignThreshold::get()
			{
				<AuthoritiesToSign<T, I>>::kill();
				<AuthoritiesState<T, I>>::kill();

				Self::deposit_event(RawEvent::SignedAuthoritySet(
					hashed_authorities_set,
					signatures
				));
			} else {
				<AuthoritiesToSign<T, I>>::put((hashed_authorities_set, signatures));
			}
		}
	}
}

impl<T, I> Module<T, I>
where
	T: Trait<I>,
	I: Instance,
{
	pub fn on_authorities_change() -> bool {
		<AuthoritiesState<T, I>>::get().0
	}

	pub fn update_authorities_state(
		old_authorities: &[RelayAuthorityT<T, I>],
		new_authorities: &[&AccountId<T>],
	) -> BlockNumber<T> {
		<OldAuthorities<T, I>>::put(old_authorities);

		let deadline = <frame_system::Module<T>>::block_number() + T::SubmitDuration::get();

		<AuthoritiesState<T, I>>::put((true, deadline));
		<AuthoritiesToSign<T, I>>::put((
			T::Hashing::hash(&new_authorities.encode()),
			<Vec<(AccountId<T>, RelaySignature<T, I>)>>::new(),
		));

		deadline
	}

	pub fn remove_authority_by_id_with<F>(
		account_id: &AccountId<T>,
		is_able_to_remove: F,
	) -> Result<RelayAuthorityT<T, I>, DispatchError>
	where
		F: Fn(&RelayAuthorityT<T, I>) -> Option<Error<T, I>>,
	{
		Ok(<Authorities<T, I>>::try_mutate(|authorities| {
			if let Some(position) = find_authority_position::<T, I>(&authorities, account_id) {
				if let Some(e) = is_able_to_remove(&authorities[position]) {
					return Err(e);
				}

				let old_authorities = authorities.clone();
				let removed_authority = authorities.remove(position);
				let new_deadline = Self::update_authorities_state(
					&old_authorities,
					authorities
						.iter()
						.map(|authority| &authority.account_id)
						.collect::<Vec<_>>()
						.as_slice(),
				);

				// TODO: optimize DB R/W, but it's ok in real case, since the set won't grow so large
				for mmr_root in <MMRRootsToSign<T, I>>::get() {
					if let Some((deadline, mut signatures)) = <SignedMMRRoots<T, I>>::get(&mmr_root)
					{
						if let Some(position) = signatures
							.iter()
							.position(|(authority, _)| authority == account_id)
						{
							signatures.remove(position);
						}

						<SignedMMRRoots<T, I>>::insert(&mmr_root, (new_deadline, signatures));

						if let Some(mmr_root) = <ClosedMMRRootSubmits<T, I>>::take(&deadline) {
							<ClosedMMRRootSubmits<T, I>>::insert(new_deadline, mmr_root);
						} else {
							// Should never enter this condition
							// TODO: error log
						}
					} else {
						// Should never enter this condition
						// TODO: error log
					}
				}

				<RingCurrency<T, I>>::remove_lock(T::LockId::get(), account_id);

				return Ok(removed_authority);
			}

			Err(<Error<T, I>>::AuthorityNE)
		})?)
	}

	pub fn remove_candidate_by_id(
		account_id: &AccountId<T>,
	) -> Result<RelayAuthorityT<T, I>, DispatchError> {
		Ok(<Candidates<T, I>>::try_mutate(|candidates| {
			if let Some(position) = find_authority_position::<T, I>(&candidates, account_id) {
				let candidate = candidates.remove(position);

				<RingCurrency<T, I>>::remove_lock(T::LockId::get(), account_id);

				Ok(candidate)
			} else {
				Err(<Error<T, I>>::CandidateNE)
			}
		})?)
	}

	pub fn check_submit_deadline(at: BlockNumber<T>) {
		let find_and_slash_misbehavior = |signatures: Vec<(AccountId<T>, RelaySignature<T, I>)>| {
			for RelayAuthority {
				account_id, stake, ..
			} in <Authorities<T, I>>::get()
			{
				if let None = signatures
					.iter()
					.position(|(authority, _)| authority == &account_id)
				{
					<RingCurrency<T, I>>::slash(&account_id, stake);
				}
			}
		};
		let (on_authorities_change, deadline) = <AuthoritiesState<T, I>>::get();

		if on_authorities_change {
			if deadline == at {
				let (_, signatures) = <AuthoritiesToSign<T, I>>::get();

				find_and_slash_misbehavior(signatures);

				<AuthoritiesState<T, I>>::put((
					true,
					<frame_system::Module<T>>::block_number() + T::SubmitDuration::get(),
				));
			}
		} else {
			if let Some(closed_submit) = Self::closed_mmr_root_submit_of(at) {
				if let Some((_, signatures)) = <SignedMMRRoots<T, I>>::get(&closed_submit) {
					find_and_slash_misbehavior(signatures);
				} else {
					// Should never enter this condition
					// TODO: error log
				}
			}
		}
	}
}

impl<T, I> RelayAuthorityProtocol<MMRRoot<T>> for Module<T, I>
where
	T: Trait<I>,
	I: Instance,
{
	fn new_mmr_to_sign(mmr_root: MMRRoot<T>) {
		if <SignedMMRRoots<T, I>>::get(&mmr_root).is_none() {
			<MMRRootsToSign<T, I>>::append(&mmr_root);

			let (on_authorities_change, authorities_submit_deadline) =
				<AuthoritiesState<T, I>>::get();
			let now = <frame_system::Module<T>>::block_number();
			let mut mmr_root_submit_deadline = now + T::SubmitDuration::get();

			// Delay if on authorities change
			if on_authorities_change {
				mmr_root_submit_deadline += authorities_submit_deadline.saturating_sub(now);
			}

			<SignedMMRRoots<T, I>>::insert(
				&mmr_root,
				(
					mmr_root_submit_deadline,
					<Vec<(AccountId<T>, RelaySignature<T, I>)>>::new(),
				),
			);
			<ClosedMMRRootSubmits<T, I>>::insert(mmr_root_submit_deadline, mmr_root);
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
