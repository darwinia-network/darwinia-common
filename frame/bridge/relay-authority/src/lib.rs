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

#[cfg(test)]
mod mock;
#[cfg(test)]
mod test;

mod primitives;
pub use primitives::*;

mod weights;
pub use weights::WeightInfo;

// --- core ---
use core::fmt::Debug;
// --- crates.io ---
use scale_info::TypeInfo;
// --- paritytech ---
use frame_support::{
	log,
	pallet_prelude::*,
	traits::{Currency, Get, LockIdentifier, LockableCurrency, WithdrawReasons},
};
use frame_system::pallet_prelude::*;
use sp_runtime::{
	traits::{Saturating, Zero},
	Perbill, SaturatedConversion,
};
use sp_std::{borrow::ToOwned, prelude::*};

#[frame_support::pallet]
pub mod pallet {
	// --- darwinia-network ---
	use crate::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		// Overrides.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		// Basics.
		type Currency: LockableCurrency<Self::AccountId>;
		type WeightInfo: WeightInfo;
		// Origins.
		type AddOrigin: EnsureOrigin<Self::Origin>;
		type RemoveOrigin: EnsureOrigin<Self::Origin>;
		type ResetOrigin: EnsureOrigin<Self::Origin>;
		// Commitments.
		type MmrRootT: Clone + Debug + PartialEq + Encode + Decode + TypeInfo;
		type MmrRoot: Get<Option<Self::MmrRootT>>;
		type Sign: Sign;
		// Constants.
		#[pallet::constant]
		type MaxMembers: Get<u32>;
		#[pallet::constant]
		type LockId: Get<LockIdentifier>;
		#[pallet::constant]
		type TermDuration: Get<Self::BlockNumber>;
		#[pallet::constant]
		type OpCodes: Get<(OpCode, OpCode)>;
		#[pallet::constant]
		type SignThreshold: Get<Perbill>;
		#[pallet::constant]
		type SubmitDuration: Get<Self::BlockNumber>;
		#[pallet::constant]
		type MaxSchedules: Get<u32>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A New MMR Root Scheduled Request to be Signed. \[block number of the mmr root to sign\]
		ScheduleMmrRoot(T::BlockNumber),
		/// MMR Root Signed. \[block number of the mmr root, mmr root, signatures\]
		MmrRootSigned(T::BlockNumber, T::MmrRootT, Vec<(T::AccountId, RelayAuthoritySignature<T>)>),
		/// A New Authority Set Change Scheduled Request to be Signed. \[message to sign\]
		ScheduleAuthoritiesChange(RelayAuthorityMessage<T>),
		/// The Next Authorities Signed. \[term, next authorities, signatures\]
		AuthoritiesChangeSigned(
			Term,
			Vec<RelayAuthoritySigner<T>>,
			Vec<(T::AccountId, RelayAuthoritySignature<T>)>,
		),
		/// Slash on Misbehavior. \[who, slashed\]
		SlashOnMisbehavior(T::AccountId, Balance<T>),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Too many members
		TooManyMembers,
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
		/// Authorities Count - TOO LOW
		AuthoritiesCountTL,
		/// Stake - INSUFFICIENT
		StakeIns,
		/// On Authorities Change - DISABLED
		OnAuthoritiesChangeDis,
		/// Scheduled Sign -NOT EXISTED
		ScheduledSignNE,
		/// Signature - INVALID
		SignatureInv,
		/// Term - MISMATCHED
		TermMis,
		/// Authorities - MISMATCHED
		AuthoritiesMis,
		/// Next Authorities - NOT EXISTED
		NextAuthoritiesNE,
		/// Too many scheduled `MmrToSign`.
		TooManySchedules,
	}

	/// Anyone can request to be an authority with some stake.
	///
	/// Once you requested, you'll enter the candidates.
	/// This request can be canceled at any time.
	#[pallet::storage]
	#[pallet::getter(fn candidates)]
	pub type Candidates<T: Config> =
		StorageValue<_, BoundedVec<RelayAuthorityT<T>, T::MaxMembers>, ValueQuery>;

	/// Authority must elect from candidates
	///
	/// Once you become an authority, you must serve for a specific term.
	/// Before that, you can't renounce.
	#[pallet::storage]
	#[pallet::getter(fn authorities)]
	pub type Authorities<T: Config> =
		StorageValue<_, BoundedVec<RelayAuthorityT<T>, T::MaxMembers>, ValueQuery>;

	/// The incoming authorities for the next term.
	#[pallet::storage]
	#[pallet::getter(fn next_authorities)]
	pub type NextAuthorities<T: Config> =
		StorageValue<_, ScheduledAuthoritiesChangeT<T>, OptionQuery>;

	/// Term index counter, play the same role as nonce in extrinsic.
	#[pallet::storage]
	#[pallet::getter(fn next_term)]
	pub type NextTerm<T: Config> = StorageValue<_, Term, ValueQuery>;

	/// The authorities change requirements.
	///
	/// Once the signatures count reaches the sign threshold storage will be killed then raise a
	/// signed event Params.
	/// 	1. the message to sign
	/// 	1. collected signatures
	#[pallet::storage]
	#[pallet::getter(fn authorities_to_sign)]
	pub type AuthoritiesToSign<T: Config> = StorageValue<
		_,
		(
			RelayAuthorityMessage<T>,
			BoundedVec<(T::AccountId, RelayAuthoritySignature<T>), T::MaxMembers>,
		),
		OptionQuery,
	>;

	/// The `MmrRootsToSign` keys cache.
	///
	/// Only use for update the `MmrRootsToSign` once the authorities changed.
	#[pallet::storage]
	#[pallet::getter(fn mmr_root_to_sign_keys)]
	pub type MmrRootsToSignKeys<T: Config> =
		StorageValue<_, BoundedVec<T::BlockNumber, T::MaxSchedules>, ValueQuery>;

	/// All the relay requirements from the backing module here.
	///
	/// If the map's key has existed, it means the mmr root relay requirement is valid.
	///
	/// Once the signatures count reaches the sign threshold storage will be killed then raise a
	/// signed event Params.
	/// 	1. the mmr root to be signed, collected signatures
	#[pallet::storage]
	#[pallet::getter(fn mmr_root_to_sign_of)]
	pub type MmrRootsToSign<T: Config> = StorageMap<
		_,
		Identity,
		T::BlockNumber,
		MmrRootToSign<T::MmrRootT, T::AccountId, RelayAuthoritySignature<T>, T::MaxMembers>,
		OptionQuery,
	>;

	/// The mmr root signature submit duration, will be delayed if on authorities change.
	#[pallet::storage]
	#[pallet::getter(fn submit_duration)]
	pub type SubmitDuration<T: Config> =
		StorageValue<_, T::BlockNumber, ValueQuery, DefaultSubmitDuration<T>>;
	#[pallet::type_value]
	pub fn DefaultSubmitDuration<T: Config>() -> T::BlockNumber {
		T::SubmitDuration::get()
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub authorities: Vec<(T::AccountId, RelayAuthoritySigner<T>, Balance<T>)>,
	}
	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { authorities: Default::default() }
		}
	}
	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			let mut authorities = vec![];

			for (account_id, signer, stake) in self.authorities.iter() {
				T::Currency::set_lock(T::LockId::get(), account_id, *stake, WithdrawReasons::all());

				authorities.push(Authority {
					account_id: account_id.to_owned(),
					signer: signer.to_owned(),
					stake: *stake,
					term: <frame_system::Pallet<T>>::block_number() + T::TermDuration::get(),
				});
			}

			<Authorities<T>>::put(
				BoundedVec::try_from(authorities).expect("genesis authorities overflowed"),
			);
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);
	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
		fn on_initialize(now: T::BlockNumber) -> Weight {
			Self::prepare_mmr_root_to_sign(now);
			// Deal with the slash thing.
			// If authority didn't do his job before the deadline.
			Self::check_misbehavior(now);

			0
		}
	}
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Request to be an authority.
		///
		/// This will be failed if match one of these sections:
		/// - already is a candidate
		/// - already is an authority
		/// - insufficient stake, required at least more than the last candidate's if too there're
		///   many candidates in the candidates' queue
		#[pallet::weight(10_000_000)]
		#[frame_support::transactional]
		pub fn request_authority(
			origin: OriginFor<T>,
			stake: Balance<T>,
			signer: RelayAuthoritySigner<T>,
		) -> DispatchResult {
			let account_id = ensure_signed(origin)?;

			if let Some(scheduled_authorities_change) = <NextAuthorities<T>>::get() {
				ensure!(
					find_authority_position::<T>(
						&scheduled_authorities_change.next_authorities,
						&account_id
					)
					.is_none(),
					<Error<T>>::AuthorityAE
				);
			}

			ensure!(
				find_authority_position::<T>(&<Authorities<T>>::get(), &account_id).is_none(),
				<Error<T>>::AuthorityAE
			);
			ensure!(T::Currency::free_balance(&account_id) > stake, <Error<T>>::StakeIns);

			<Candidates<T>>::try_mutate(|candidates| {
				ensure!(
					find_authority_position::<T>(candidates, &account_id).is_none(),
					<Error<T>>::CandidateAE
				);

				if candidates.len() as u32 == T::MaxMembers::get() {
					let mut minimum_stake = candidates[0].stake;
					let mut position = 0;

					for (i, candidate) in candidates.iter().skip(1).enumerate() {
						let stake = candidate.stake;

						if stake < minimum_stake {
							minimum_stake = stake;
							position = i;
						}
					}

					ensure!(stake > minimum_stake, <Error<T>>::StakeIns);

					// TODO: slash the weed out?
					let weep_out = candidates.remove(position);

					T::Currency::remove_lock(T::LockId::get(), &weep_out.account_id);
				}

				T::Currency::set_lock(T::LockId::get(), &account_id, stake, WithdrawReasons::all());

				candidates
					.try_push(Authority { account_id, signer, stake, term: 0_u32.into() })
					.map_err(|_| <Error<T>>::TooManyMembers)?;

				DispatchResult::Ok(())
			})?;

			Ok(())
		}

		/// Cancel the request to be an authority.
		///
		/// This will remove you from the candidates queue immediately.
		#[pallet::weight(10_000_000)]
		pub fn cancel_request(origin: OriginFor<T>) -> DispatchResult {
			let account_id = ensure_signed(origin)?;

			Self::remove_candidate_by_id_with(&account_id, || {
				T::Currency::remove_lock(T::LockId::get(), &account_id)
			})?;

			Ok(())
		}

		/// Remove all the candidates.
		#[pallet::weight(10_000_000)]
		pub fn kill_candidates(origin: OriginFor<T>) -> DispatchResult {
			T::ResetOrigin::ensure_origin(origin)?;

			let lock_id = T::LockId::get();

			for Authority { account_id, .. } in <Candidates<T>>::take() {
				T::Currency::remove_lock(lock_id, &account_id);
			}

			Ok(())
		}

		/// Add an authority from the candidates.
		///
		/// This call is disallowed during the authorities change.
		#[pallet::weight(10_000_000)]
		#[frame_support::transactional]
		pub fn add_authorities(
			origin: OriginFor<T>,
			account_ids: Vec<T::AccountId>,
		) -> DispatchResult {
			T::AddOrigin::ensure_origin(origin)?;

			ensure!(!Self::on_authorities_change(), <Error<T>>::OnAuthoritiesChangeDis);
			// Won't check duplicated here, MUST make sure this authority is unique
			// As we already make a check in `request_authority`
			let next_authorities = {
				let mut authorities = <Authorities<T>>::get();

				for account_id in account_ids {
					let mut authority = Self::remove_candidate_by_id_with(&account_id, || ())?;

					authority.term =
						<frame_system::Pallet<T>>::block_number() + T::TermDuration::get();

					authorities.try_push(authority).map_err(|_| <Error<T>>::TooManyMembers)?;
				}

				authorities
			};

			Self::schedule_authorities_change(next_authorities);

			Ok(())
		}

		/// Renounce the authority of the account.
		///
		/// This call is disallowed during the authorities change.
		/// No-op if can't find the authority.
		/// Will fail if you still in the term.
		#[pallet::weight(10_000_000)]
		#[frame_support::transactional]
		pub fn renounce_authority(origin: OriginFor<T>) -> DispatchResult {
			let account_id = ensure_signed(origin)?;

			ensure!(!Self::on_authorities_change(), <Error<T>>::OnAuthoritiesChangeDis);

			let next_authorities =
				Self::remove_authority_by_ids_with(&[account_id], |authority| {
					if authority.term >= <frame_system::Pallet<T>>::block_number() {
						Some(<Error<T>>::AuthorityIT)
					} else {
						None
					}
				})?;

			if next_authorities.is_empty() {
				Err(<Error<T>>::AuthoritiesCountTL)?;
			}

			Self::schedule_authorities_change(next_authorities);

			Ok(())
		}

		/// Remove the given authorities.
		///
		/// This call is disallowed during the authorities change.
		#[pallet::weight(10_000_000)]
		pub fn remove_authorities(
			origin: OriginFor<T>,
			account_ids: Vec<T::AccountId>,
		) -> DispatchResult {
			T::RemoveOrigin::ensure_origin(origin)?;

			ensure!(!Self::on_authorities_change(), <Error<T>>::OnAuthoritiesChangeDis);

			let next_authorities = Self::remove_authority_by_ids_with(&account_ids, |_| None)?;

			if next_authorities.is_empty() {
				Err(<Error<T>>::AuthoritiesCountTL)?;
			}

			Self::schedule_authorities_change(next_authorities);

			Ok(())
		}

		#[pallet::weight(10_000_000)]
		#[frame_support::transactional]
		pub fn kill_authorities(origin: OriginFor<T>) -> DispatchResult {
			T::ResetOrigin::ensure_origin(origin)?;

			let lock_id = T::LockId::get();

			for Authority { account_id, .. } in <Authorities<T>>::take() {
				T::Currency::remove_lock(lock_id, &account_id);
			}

			<NextAuthorities<T>>::kill();
			<AuthoritiesToSign<T>>::kill();
			{
				<MmrRootsToSign<T>>::remove_all(None);
				let schedule = (<frame_system::Pallet<T>>::block_number().saturated_into::<u64>()
					/ 10 * 10 + 10)
					.saturated_into();
				<MmrRootsToSignKeys<T>>::mutate(|schedules| {
					let new_schedules = BoundedVec::try_from(vec![schedule])
						.map_err(|_| <Error<T>>::TooManySchedules)?;

					*schedules = new_schedules;

					DispatchResult::Ok(())
				})?;
				Self::schedule_mmr_root(schedule)?;
			}
			<SubmitDuration<T>>::kill();

			Ok(())
		}

		#[pallet::weight(10_000_000)]
		pub fn force_new_term(origin: OriginFor<T>) -> DispatchResult {
			T::ResetOrigin::ensure_origin(origin)?;

			Self::apply_authorities_change()?;
			Self::sync_authorities_change()?;

			<NextAuthorities<T>>::kill();

			Ok(())
		}

		/// Require authority origin
		///
		/// This call is disallowed during the authorities change
		///
		/// No-op if already submit
		///
		/// Verify
		/// - the relay requirement is valid
		/// - the signature is signed by the submitter
		#[pallet::weight(10_000_000)]
		pub fn submit_signed_mmr_root(
			origin: OriginFor<T>,
			block_number: T::BlockNumber,
			signature: RelayAuthoritySignature<T>,
		) -> DispatchResult {
			let authority = ensure_signed(origin)?;

			// Not allow to submit during the authority set change
			ensure!(!Self::on_authorities_change(), <Error<T>>::OnAuthoritiesChangeDis);

			let mut to_sign =
				<MmrRootsToSign<T>>::get(block_number).ok_or(<Error<T>>::ScheduledSignNE)?;

			// No-op if was already submitted
			if to_sign.signatures.iter().any(|(signer, _)| signer == &authority) {
				return Ok(());
			}

			let authorities = <Authorities<T>>::get();
			let signer =
				find_signer::<T>(&authorities, &authority).ok_or(<Error<T>>::AuthorityNE)?;

			// The message is composed of:
			//
			// hash(
			// 	codec(
			// 		spec_name: String,
			// 		op_code: OpCode,
			// 		block number: Compact<BlockNumber>,
			// 		mmr_root: Hash
			// 	)
			// )
			let message = T::Sign::hash(
				&Message {
					_1: T::Version::get().spec_name,
					_2: T::OpCodes::get().0,
					_3: block_number,
					_4: to_sign.mmr_root.clone(),
				}
				.encode(),
			);

			ensure!(
				T::Sign::verify_signature(&signature, &message, &signer),
				<Error<T>>::SignatureInv
			);

			to_sign
				.signatures
				.try_push((authority, signature))
				.map_err(|_| <Error<T>>::TooManyMembers)?;

			if Perbill::from_rational(to_sign.signatures.len() as u32, authorities.len() as _)
				>= T::SignThreshold::get()
			{
				// TODO: clean the mmr root which was contains in this mmr root?

				Self::mmr_root_signed(block_number);
				Self::deposit_event(<Event<T>>::MmrRootSigned(
					block_number,
					to_sign.mmr_root,
					to_sign.signatures.to_vec(),
				));
			} else {
				<MmrRootsToSign<T>>::insert(block_number, to_sign);
			}

			Ok(())
		}

		/// Require authority origin
		///
		/// This call is only allowed during the authorities change
		///
		/// No-op if already submit
		///
		/// Verify
		/// - the relay requirement is valid
		/// - the signature is signed by the submitter
		#[pallet::weight(10_000_000)]
		pub fn submit_signed_authorities(
			origin: OriginFor<T>,
			signature: RelayAuthoritySignature<T>,
		) -> DispatchResult {
			let authority = ensure_signed(origin)?;

			ensure!(Self::on_authorities_change(), <Error<T>>::OnAuthoritiesChangeDis);

			let (message, mut signatures) = if let Some(signatures) = <AuthoritiesToSign<T>>::get()
			{
				signatures
			} else {
				return Ok(());
			};

			if signatures.iter().any(|(authority_, _)| authority_ == &authority) {
				return Ok(());
			}

			let authorities = <Authorities<T>>::get();
			let signer =
				find_signer::<T>(&authorities, &authority).ok_or(<Error<T>>::AuthorityNE)?;

			ensure!(
				T::Sign::verify_signature(&signature, &message, &signer),
				<Error<T>>::SignatureInv
			);

			signatures.try_push((authority, signature)).map_err(|_| <Error<T>>::TooManyMembers)?;

			if Perbill::from_rational(signatures.len() as u32, authorities.len() as _)
				>= T::SignThreshold::get()
			{
				Self::apply_authorities_change()?;
				Self::deposit_event(<Event<T>>::AuthoritiesChangeSigned(
					<NextTerm<T>>::get(),
					<NextAuthorities<T>>::get()
						.ok_or(<Error<T>>::NextAuthoritiesNE)?
						.next_authorities
						.into_iter()
						.map(|authority| authority.signer)
						.collect(),
					signatures.to_vec(),
				));
			} else {
				<AuthoritiesToSign<T>>::put((message, signatures));
			}

			Ok(())
		}
	}
}
pub use pallet::*;

impl<T: Config> Pallet<T> {
	fn remove_candidate_by_id_with<F>(
		account_id: &T::AccountId,
		f: F,
	) -> Result<RelayAuthorityT<T>, DispatchError>
	where
		F: Fn(),
	{
		Ok(<Candidates<T>>::try_mutate(|candidates| {
			if let Some(position) = find_authority_position::<T>(candidates, account_id) {
				f();

				Ok(candidates.remove(position))
			} else {
				Err(<Error<T>>::CandidateNE)
			}
		})?)
	}

	fn on_authorities_change() -> bool {
		<NextAuthorities<T>>::exists()
	}

	fn schedule_authorities_change(
		next_authorities: BoundedVec<RelayAuthorityT<T>, T::MaxMembers>,
	) {
		// The message is composed of:
		//
		// hash(
		// 	codec(
		// 		spec_name: String,
		// 		op_code: OpCode,
		// 		term: Compact<u32>,
		// 		next authorities: Vec<Signer>
		// 	)
		// )
		let message = T::Sign::hash(
			&Message {
				_1: T::Version::get().spec_name,
				_2: T::OpCodes::get().1,
				_3: <NextTerm<T>>::get(),
				_4: next_authorities
					.iter()
					.map(|authority| authority.signer.clone())
					.collect::<Vec<_>>(),
			}
			.encode(),
		);

		<AuthoritiesToSign<T>>::put((
			&message,
			<BoundedVec<(T::AccountId, RelayAuthoritySignature<T>), T::MaxMembers>>::default(),
		));

		let submit_duration = T::SubmitDuration::get();

		<NextAuthorities<T>>::put(ScheduledAuthoritiesChange {
			next_authorities,
			deadline: <frame_system::Pallet<T>>::block_number() + submit_duration,
		});
		<SubmitDuration<T>>::mutate(|submit_duration_| *submit_duration_ += submit_duration);

		Self::deposit_event(<Event<T>>::ScheduleAuthoritiesChange(message));
	}

	fn remove_authority_by_ids_with<F>(
		account_ids: &[T::AccountId],
		f: F,
	) -> Result<BoundedVec<RelayAuthorityT<T>, T::MaxMembers>, DispatchError>
	where
		F: Fn(&RelayAuthorityT<T>) -> Option<Error<T>>,
	{
		let mut authorities = <Authorities<T>>::get();
		let mut remove_authorities = Vec::new();

		for account_id in account_ids.iter() {
			let position = find_authority_position::<T>(&authorities, account_id)
				.ok_or(<Error<T>>::AuthorityNE)?;

			if let Some(e) = f(&authorities[position]) {
				Err(e)?;
			}

			authorities.remove(position);
			remove_authorities.push(account_id);
		}

		if remove_authorities.is_empty() {
			Err(<Error<T>>::AuthorityNE)?
		}

		// TODO: optimize DB R/W, but it's ok in real case, since the set won't grow so large
		for key in <MmrRootsToSignKeys<T>>::get() {
			if let Some(mut mmr_root_to_sign) = <MmrRootsToSign<T>>::get(key) {
				for account_id in &remove_authorities {
					if let Some(position) = mmr_root_to_sign
						.signatures
						.iter()
						.position(|(authority, _)| &authority == account_id)
					{
						mmr_root_to_sign.signatures.remove(position);
					}

					<MmrRootsToSign<T>>::insert(key, &mmr_root_to_sign);
				}
			} else {
				// Should never enter this condition
				// TODO: error log
			}
		}

		Ok(authorities)
	}

	fn prepare_mmr_root_to_sign(block_number: T::BlockNumber) {
		if let Some(schedule) = <MmrRootsToSignKeys<T>>::get()
			.into_iter()
			// In order to get the schedule block number's MMR root.
			// 	1. MMR root doesn't contain itself(header hash)
			// 	1. MMR's state change on finalize
			// That's why we need to plus `2` to the scheduled block number.
			.find(|schedule| *schedule + 2_u32.into() == block_number)
		{
			if let Some(mmr_root) = T::MmrRoot::get() {
				let _ = <MmrRootsToSign<T>>::try_mutate(schedule, |maybe_mmr_root_to_sign| {
					if maybe_mmr_root_to_sign.is_none() {
						*maybe_mmr_root_to_sign = Some(MmrRootToSign::new(mmr_root));

						Ok(())
					} else {
						Err(())
					}
				});
			} else {
				log::error!(
						"Failed to `get_root` while `prepare_mmr_root_to_sign` for block `{:?}` at block `{:?}`",
						schedule,
						block_number
					);
			}
		}
	}

	fn apply_authorities_change() -> DispatchResult {
		let next_authorities =
			<NextAuthorities<T>>::get().ok_or(<Error<T>>::NextAuthoritiesNE)?.next_authorities;
		let authorities = <Authorities<T>>::get();

		for Authority { account_id, .. } in authorities {
			if !next_authorities
				.iter()
				.any(|Authority { account_id: account_id_, .. }| account_id_ == &account_id)
			{
				T::Currency::remove_lock(T::LockId::get(), &account_id);
			}
		}

		<AuthoritiesToSign<T>>::kill();
		<SubmitDuration<T>>::kill();

		Ok(())
	}

	fn mmr_root_signed(block_number: T::BlockNumber) {
		<MmrRootsToSign<T>>::remove(block_number);
		<MmrRootsToSignKeys<T>>::mutate(|mmr_roots_to_sign_keys| {
			if let Some(position) =
				mmr_roots_to_sign_keys.iter().position(|key| key == &block_number)
			{
				mmr_roots_to_sign_keys.remove(position);
			}
		});
	}

	fn check_misbehavior(now: T::BlockNumber) {
		let find_and_slash_misbehavior =
			|signatures: BoundedVec<(T::AccountId, RelayAuthoritySignature<T>), T::MaxMembers>| {
				let _ = <Authorities<T>>::try_mutate(|authorities| {
					let mut storage_changed = false;

					for Authority { account_id, stake, .. } in authorities.as_mut().iter_mut() {
						if !signatures.iter().any(|(authority, _)| authority == account_id) {
							Self::deposit_event(<Event<T>>::SlashOnMisbehavior(
								account_id.to_owned(),
								*stake,
							));

							if !stake.is_zero() {
								// Can not set lock 0, so remove the lock
								T::Currency::remove_lock(T::LockId::get(), account_id);
								T::Currency::slash(account_id, *stake);

								*stake = 0_u32.into();
								storage_changed = true;
							}

							// TODO: schedule a new set
						}
					}

					if storage_changed {
						Ok(())
					} else {
						Err(())
					}
				});
			};

		if let Some(mut scheduled_authorities_change) = <NextAuthorities<T>>::get() {
			if scheduled_authorities_change.deadline == now {
				if let Some((_, signatures)) = <AuthoritiesToSign<T>>::get() {
					find_and_slash_misbehavior(signatures);
				} else {
					// Should never enter this condition
					// TODO: error log
				}

				let submit_duration = T::SubmitDuration::get();

				scheduled_authorities_change.deadline += submit_duration;

				<NextAuthorities<T>>::put(scheduled_authorities_change);
				<SubmitDuration<T>>::mutate(|submit_duration_| {
					*submit_duration_ += submit_duration
				});
			}
		} else {
			let at = now.saturating_sub(<SubmitDuration<T>>::get());

			if let Some(mmr_root_to_sign) = <MmrRootsToSign<T>>::take(at) {
				let _ = <MmrRootsToSignKeys<T>>::try_mutate(|keys| {
					if let Some(position) = keys.iter().position(|key| key == &at) {
						keys.remove(position);

						Ok(())
					} else {
						Err(())
					}
				});

				find_and_slash_misbehavior(mmr_root_to_sign.signatures);

				// TODO: schedule a new mmr root (greatest one in the keys)
			}
		}
	}
}
impl<T> RelayAuthorityProtocol<T::BlockNumber> for Pallet<T>
where
	T: Config,
{
	type Signer = RelayAuthoritySigner<T>;

	fn schedule_mmr_root(block_number: T::BlockNumber) -> DispatchResult {
		if <MmrRootsToSignKeys<T>>::exists() {
			if let Some(scheduled_num) = <MmrRootsToSignKeys<T>>::decode_len() {
				if scheduled_num as u32 == T::MaxSchedules::get() {
					Err(<Error<T>>::TooManySchedules)?;
				}
			} else {
				Err("Failed to get `decode_len`")?;
			}
		}

		match <MmrRootsToSignKeys<T>>::try_mutate(|mmr_roots_to_sign_keys| {
			// No-op if the sign was already scheduled
			if mmr_roots_to_sign_keys.contains(&block_number) {
				return Err(false);
			}

			mmr_roots_to_sign_keys.try_push(block_number).map_err(|_| false)?;

			Self::deposit_event(<Event<T>>::ScheduleMmrRoot(block_number));

			Ok(())
		}) {
			Err(true) => Err(<Error<T>>::TooManySchedules)?,
			_ => Ok(()),
		}
	}

	fn check_authorities_change_to_sync(
		term: Term,
		mut authorities_change_to_sync: Vec<Self::Signer>,
	) -> DispatchResult {
		ensure!(term == <NextTerm<T>>::get(), <Error<T>>::TermMis);

		let mut next_authorities = <NextAuthorities<T>>::get()
			.ok_or(<Error<T>>::NextAuthoritiesNE)?
			.next_authorities
			.into_iter()
			.map(|authority| authority.signer)
			.collect::<Vec<_>>();

		authorities_change_to_sync.sort();
		next_authorities.sort();

		if authorities_change_to_sync == next_authorities {
			Ok(())
		} else {
			Err(<Error<T>>::AuthoritiesMis)?
		}
	}

	fn sync_authorities_change() -> DispatchResult {
		let next_authorities =
			<NextAuthorities<T>>::take().ok_or(<Error<T>>::NextAuthoritiesNE)?.next_authorities;

		<Authorities<T>>::put(next_authorities);
		<NextTerm<T>>::mutate(|next_term| *next_term += 1);

		Ok(())
	}
}

fn find_authority_position<T>(
	authorities: &[RelayAuthorityT<T>],
	account_id: &T::AccountId,
) -> Option<usize>
where
	T: Config,
{
	authorities.iter().position(|relay_authority| relay_authority == account_id)
}

fn find_signer<T>(
	authorities: &[RelayAuthorityT<T>],
	account_id: &T::AccountId,
) -> Option<RelayAuthoritySigner<T>>
where
	T: Config,
{
	authorities
		.iter()
		.position(|relay_authority| relay_authority == account_id)
		.map(|position| authorities[position].signer.to_owned())
}
