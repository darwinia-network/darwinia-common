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
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

//! # Relay Authorities Module

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod test;

mod types {
	// --- darwinia ---
	use crate::*;

	pub type AccountId<T> = <T as frame_system::Trait>::AccountId;
	pub type BlockNumber<T> = <T as frame_system::Trait>::BlockNumber;
	pub type MMRRoot<T> = <T as frame_system::Trait>::Hash;
	pub type RingBalance<T, I> = <RingCurrency<T, I> as Currency<AccountId<T>>>::Balance;
	pub type RingCurrency<T, I> = <T as Trait<I>>::RingCurrency;

	pub type RelayAuthoritySigner<T, I> = <<T as Trait<I>>::Sign as Sign<BlockNumber<T>>>::Signer;
	pub type RelayAuthorityMessage<T, I> = <<T as Trait<I>>::Sign as Sign<BlockNumber<T>>>::Message;
	pub type RelayAuthoritySignature<T, I> =
		<<T as Trait<I>>::Sign as Sign<BlockNumber<T>>>::Signature;
	pub type RelayAuthorityT<T, I> =
		RelayAuthority<AccountId<T>, RelayAuthoritySigner<T, I>, RingBalance<T, I>, BlockNumber<T>>;
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
use sp_runtime::{DispatchError, DispatchResult, Perbill};
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
	type DarwiniaMMR: MMR<Self::BlockNumber, Self::Hash>;
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
		BlockNumber = BlockNumber<T>,
		MMRRoot = MMRRoot<T>,
		RelayAuthoritySigner = RelayAuthoritySigner<T, I>,
		RelayAuthorityMessage = RelayAuthorityMessage<T, I>,
		RelayAuthoritySignature = RelayAuthoritySignature<T, I>,
	{
		/// A New MMR Root Request to be Signed. [block number of the mmr root to sign]
		NewMMRRoot(BlockNumber),
		/// MMR Root Signed. [block number of the mmr root, mmr root, signatures]
		MMRRootSigned(BlockNumber, MMRRoot, Vec<(AccountId, RelayAuthoritySignature)>),
		/// A New Authorities Request to be Signed. [message to sign]
		NewAuthorities(RelayAuthorityMessage),
		/// Authorities Signed. [term, new authorities, signatures]
		AuthoritiesSetSigned(Term, Vec<RelayAuthoritySigner>, Vec<(AccountId, RelayAuthoritySignature)>),
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
		/// Scheduled Sign -NOT EXISTED
		ScheduledSignNE,
		/// Darwinia MMR Root - NOT READY YET
		DarwiniaMMRRootNRY,
		/// Signature - INVALID
		SignatureInv,
	}
}

decl_storage! {
	trait Store for Module<T: Trait<I>, I: Instance = DefaultInstance> as DarwiniaRelayAuthorities {
		/// Anyone can request to be an authority with some stake
		/// Also submit your signer at the same time (for ethereum: your ethereum address in H160 format)
		///
		/// Once you requested, you'll enter the candidates
		///
		/// This request can be canceled at any time
		pub Candidates get(fn candidates): Vec<RelayAuthorityT<T, I>>;

		/// Authority must elect from candidates
		///
		/// Only council or root can be the voter of the election
		///
		/// Once you become an authority, you must serve for a specific term. Before that, you can't renounce
		pub Authorities get(fn authorities): Vec<RelayAuthorityT<T, I>>;

		/// A snapshot for the old authorities while authorities changed
		pub OldAuthorities get(fn old_authorities): Vec<RelayAuthorityT<T, I>>;

		/// A term index counter, play the same role as nonce in extrinsic
		pub AuthorityTerm get(fn authority_term): Term,;

		/// The state of current authorities set
		///
		/// Tuple Params
		/// 	1. is on authority change
		/// 	1. the authorities change signature submit deadline, this will be delay indefinitely if can't collect enough signatures
		pub AuthoritiesState get(fn authorities_state): (bool, BlockNumber<T>) = (false, 0.into());

		/// The authorities change requirements
		///
		/// Once the signatures count reaches the sign threshold storage will be killed then raise a signed event
		///
		/// Params
		/// 	1. the message to sign
		/// 	1. collected signatures
		pub AuthoritiesToSign
			get(fn authorities_to_sign)
			: (RelayAuthorityMessage<T, I>, Vec<(AccountId<T>, RelayAuthoritySignature<T, I>)>);

		/// The `MMRRootsToSign` keys cache
		///
		/// Only use for update the `MMRRootsToSign` once the authorities changed
		pub MMRRootsToSignKeys get(fn mmr_root_to_sign_keys): Vec<BlockNumber<T>>;

		/// All the relay requirements from the backing module here
		///
		/// If the map's key has existed, it means the mmr root relay requirement is valid
		///
		/// Once the signatures count reaches the sign threshold storage will be killed then raise a signed event
		///
		/// Params
		/// 	1. collected signatures
		pub MMRRootsToSign
			get(fn mmr_root_to_sign_of)
			: map hasher(identity) BlockNumber<T>
			=> Option<Vec<(AccountId<T>, RelayAuthoritySignature<T, I>)>>;

		/// A cache for the old authorities who was renounce or kicked from authorities
		///
		/// Remove their lock while the submit authorities change signatures finished
		pub OldAuthoritiesLockToRemove get(fn old_authorities_lock_to_remove): Vec<AccountId<T>>;

		/// The mmr root signature submit duration, will be delayed if on authorities change
		pub SubmitDuration get(fn submit_duration): BlockNumber<T> = T::SubmitDuration::get();
	}
	add_extra_genesis {
		config(authorities): Vec<(AccountId<T>, RelayAuthoritySigner<T, I>, RingBalance<T, I>)>;
		build(|config| {
			let mut authorities = vec![];

			for (account_id, signer, stake) in config.authorities.iter() {
				T::RingCurrency::set_lock(
					T::LockId::get(),
					account_id,
					LockFor::Common { amount: *stake },
					WithdrawReasons::all(),
				);

				authorities.push(RelayAuthority {
					account_id: account_id.to_owned(),
					signer: signer.to_owned(),
					stake: *stake,
					term: <frame_system::Module<T>>::block_number() + T::TermDuration::get()
				});
			}

			<Authorities<T, I>>::put(authorities);
		});
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

		// Deal with the slash thing. If authority didn't do his job before the deadline
		fn on_initialize(now: BlockNumber<T>) -> Weight {
			Self::check_misbehavior(now);

			0
		}

		/// Request to be an authority
		///
		/// This will be failed if match one of these sections:
		/// - already is a candidate
		/// - already is an authority
		/// - insufficient stake, required at least more than the last candidate's
		///   if too there're many candidates in the candidates' queue
		#[weight = 10_000_000]
		pub fn request_authority(
			origin,
			stake: RingBalance<T, I>,
			signer: RelayAuthoritySigner<T, I>,
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

				// Max candidates can't be zero
				if candidates.len() == T::MaxCandidates::get() {
					let mut minimum_stake = candidates[0].stake;
					let mut position = 0;

					for (i, candidate) in candidates.iter().skip(1).enumerate() {
						let stake = candidate.stake;

						if stake < minimum_stake {
							minimum_stake = stake;
							position = i;
						}
					}

					ensure!(stake > minimum_stake, <Error<T, I>>::StakeIns);

					// TODO: slash the weed out?
					let weep_out = candidates.remove(position);

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

		/// This would never fail. No-op if can't find the request
		#[weight = 10_000_000]
		pub fn cancel_request(origin) {
			let account_id = ensure_signed(origin)?;
			let _ = Self::remove_candidate_by_id_with(
				&account_id,
				|| <RingCurrency<T, I>>::remove_lock(T::LockId::get(), &account_id)
			);
		}

		// TODO: not allow to renounce, if there's only one authority
		/// Renounce the authority for you
		///
		/// This call is disallowed during the authorities change
		///
		/// No-op if can't find the authority
		///
		/// Will fail if you still in the term
		#[weight = 10_000_000]
		pub fn renounce_authority(origin) {
			let account_id = ensure_signed(origin)?;

			ensure!(!Self::on_authorities_change(), <Error<T, I>>::OnAuthoritiesChangeDis);

			Self::remove_authority_by_id_with(
				&account_id,
				|authority| if authority.term >= <frame_system::Module<T>>::block_number() {
					Some(<Error<T, I>>::AuthorityIT)
				} else {
					None
				}
			)?;
		}

		// TODO: add several authorities once
		/// Require add origin
		///
		/// Add an authority from the candidates
		///
		/// This call is disallowed during the authorities change
		#[weight = 10_000_000]
		pub fn add_authority(origin, account_id: AccountId<T>) {
			T::AddOrigin::ensure_origin(origin)?;

			ensure!(!Self::on_authorities_change(), <Error<T, I>>::OnAuthoritiesChangeDis);

			let mut authority = Self::remove_candidate_by_id_with(&account_id, || ())?;

			authority.term = <frame_system::Module<T>>::block_number() + T::TermDuration::get();

			// Won't check duplicated here, MUST make this authority sure is unique
			// As we already make a check in `request_authority`
			<Authorities<T, I>>::mutate(|authorities| {
				let old_authorities = authorities.clone();

				authorities.push(authority);

				Self::start_authorities_change(&old_authorities, &authorities);
			});
		}

		// TODO: remove several authorities once
		// TODO: not allow to renounce, if there's only one authority
		/// Require remove origin
		///
		/// This call is disallowed during the authorities change
		///
		/// No-op if can't find the authority
		#[weight = 10_000_000]
		pub fn remove_authority(origin, account_id: AccountId<T>) {
			T::RemoveOrigin::ensure_origin(origin)?;

			ensure!(!Self::on_authorities_change(), <Error<T, I>>::OnAuthoritiesChangeDis);

			let _ = Self::remove_authority_by_id_with(&account_id, |_| None);
		}

		/// Require reset origin
		///
		/// Clear the candidates. Also, remember to release the stake
		#[weight = 10_000_000]
		pub fn kill_candidates(origin) {
			T::ResetOrigin::ensure_origin(origin)?;

			let lock_id = T::LockId::get();

			for RelayAuthority { account_id, .. } in <Candidates<T, I>>::take() {
				<RingCurrency<T, I>>::remove_lock(lock_id, &account_id);
			}
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
		#[weight = 10_000_000]
		pub fn submit_signed_mmr_root(
			origin,
			block_number: BlockNumber<T>,
			mmr_root: MMRRoot<T>,
			signature: RelayAuthoritySignature<T, I>
		) {
			let authority = ensure_signed(origin)?;

			// Not allow to submit during the authorities set change
			ensure!(!Self::on_authorities_change(), <Error<T, I>>::OnAuthoritiesChangeDis);

			let mut signatures =
				<MMRRootsToSign<T, I>>::get(block_number).ok_or(<Error<T, I>>::ScheduledSignNE)?;

			// No-op if was already submitted
			if signatures.iter().position(|(authority_, _)| authority_ == &authority).is_some() {
				return Ok(());
			}

			let authorities = <Authorities<T, I>>::get();
			let signer = find_signer::<T, I>(
				&authorities,
				&authority
			).ok_or(<Error<T, I>>::AuthorityNE)?;
			// The message is composed of:
			//
			// hash(codec(spec_name: String, block number: Compact<BlockNumber>, mmr_root: Hash))
			let message = T::Sign::hash(
				&_S {
					_1: T::Version::get().spec_name,
					_2: block_number,
					_3: T::DarwiniaMMR::get_root(block_number).ok_or(<Error<T, I>>::DarwiniaMMRRootNRY)?
				}
				.encode()
			);

			ensure!(
				T::Sign::verify_signature(&signature, &message, &signer),
				 <Error<T, I>>::SignatureInv
			);

			signatures.push((authority, signature));

			if Perbill::from_rational_approximation(signatures.len() as u32 + 1, authorities.len() as _)
				>= T::SignThreshold::get()
			{
				// TODO: clean the mmr root which was contains in this mmr root?

				Self::finish_collect_mmr_root_sign(block_number);
				Self::deposit_event(RawEvent::MMRRootSigned(block_number, mmr_root, signatures));
			} else {
				<MMRRootsToSign<T, I>>::insert(block_number, signatures);
			}
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
		#[weight = 10_000_000]
		pub fn submit_signed_authorities(origin, signature: RelayAuthoritySignature<T, I>) {
			let old_authority = ensure_signed(origin)?;

			ensure!(Self::on_authorities_change(), <Error<T, I>>::OnAuthoritiesChangeDis);

			let (message, mut signatures) = <AuthoritiesToSign<T, I>>::get();

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
				T::Sign::verify_signature(&signature, &message, &signer),
				 <Error<T, I>>::SignatureInv
			);

			signatures.push((old_authority, signature));

			if Perbill::from_rational_approximation(signatures.len() as u32 + 1, old_authorities.len() as _)
				>= T::SignThreshold::get()
			{
				Self::wait_target_chain_authorities_change();
				Self::deposit_event(RawEvent::AuthoritiesSetSigned(
					<AuthorityTerm<I>>::get(),
					<Authorities<T, I>>::get()
						.into_iter()
						.map(|authority| authority.signer)
						.collect(),
					signatures
				));
			} else {
				<AuthoritiesToSign<T, I>>::put((message, signatures));
			}
		}
	}
}

impl<T, I> Module<T, I>
where
	T: Trait<I>,
	I: Instance,
{
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

				Self::start_authorities_change(&old_authorities, &authorities);

				<RingCurrency<T, I>>::remove_lock(T::LockId::get(), account_id);

				// TODO: optimize DB R/W, but it's ok in real case, since the set won't grow so large
				for key in <MMRRootsToSignKeys<T, I>>::get() {
					if let Some(mut signatures) = <MMRRootsToSign<T, I>>::get(key) {
						if let Some(position) = signatures
							.iter()
							.position(|(authority, _)| authority == account_id)
						{
							signatures.remove(position);
						}

						<MMRRootsToSign<T, I>>::insert(key, signatures);
					} else {
						// Should never enter this condition
						// TODO: error log
					}
				}

				<OldAuthoritiesLockToRemove<T, I>>::append(account_id);

				return Ok(removed_authority);
			}

			Err(<Error<T, I>>::AuthorityNE)
		})?)
	}

	pub fn remove_candidate_by_id_with<F>(
		account_id: &AccountId<T>,
		maybe_remove_lock: F,
	) -> Result<RelayAuthorityT<T, I>, DispatchError>
	where
		F: Fn(),
	{
		Ok(<Candidates<T, I>>::try_mutate(|candidates| {
			if let Some(position) = find_authority_position::<T, I>(&candidates, account_id) {
				maybe_remove_lock();

				Ok(candidates.remove(position))
			} else {
				Err(<Error<T, I>>::CandidateNE)
			}
		})?)
	}

	pub fn on_authorities_change() -> bool {
		<AuthoritiesState<T, I>>::get().0
	}

	pub fn start_authorities_change(
		old_authorities: &[RelayAuthorityT<T, I>],
		new_authorities: &[RelayAuthorityT<T, I>],
	) {
		<OldAuthorities<T, I>>::put(old_authorities);

		// The message is composed of:
		//
		// hash(codec(spec_name: String, term: Compact<u32>, new authorities: Vec<Signer>))
		let message = T::Sign::hash(
			&_S {
				_1: T::Version::get().spec_name,
				_2: <AuthorityTerm<I>>::get(),
				_3: new_authorities
					.iter()
					.map(|authority| authority.signer.clone())
					.collect::<Vec<_>>(),
			}
			.encode(),
		);

		<AuthoritiesToSign<T, I>>::put((
			&message,
			<Vec<(AccountId<T>, RelayAuthoritySignature<T, I>)>>::new(),
		));

		Self::deposit_event(RawEvent::NewAuthorities(message));

		let submit_duration = T::SubmitDuration::get();

		<AuthoritiesState<T, I>>::put((
			true,
			<frame_system::Module<T>>::block_number() + submit_duration,
		));
		<SubmitDuration<T, I>>::mutate(|submit_duration_| *submit_duration_ += submit_duration);
	}

	pub fn wait_target_chain_authorities_change() {
		<AuthoritiesToSign<T, I>>::kill();
		<AuthoritiesState<T, I>>::mutate(|authorities_state| authorities_state.1 = 0);

		for account_id in <OldAuthoritiesLockToRemove<T, I>>::take() {
			<RingCurrency<T, I>>::remove_lock(T::LockId::get(), &account_id);
		}

		<SubmitDuration<T, I>>::kill();
	}

	pub fn finish_collect_mmr_root_sign(block_number: BlockNumber<T>) {
		<MMRRootsToSign<T, I>>::remove(block_number);
		<MMRRootsToSignKeys<T, I>>::mutate(|mmr_roots_to_sign_keys| {
			if let Some(position) = mmr_roots_to_sign_keys
				.iter()
				.position(|key| key == &block_number)
			{
				mmr_roots_to_sign_keys.remove(position);
			}
		});
	}

	pub fn check_misbehavior(at: BlockNumber<T>) {
		let find_and_slash_misbehavior =
			|signatures: Vec<(AccountId<T>, RelayAuthoritySignature<T, I>)>| {
				for RelayAuthority {
					account_id, stake, ..
				} in <Authorities<T, I>>::get()
				{
					if let None = signatures
						.iter()
						.position(|(authority, _)| authority == &account_id)
					{
						<RingCurrency<T, I>>::slash(&account_id, stake);

						// TODO: how to deal with the slashed authority
					}
				}
			};
		let (on_authorities_change, deadline) = <AuthoritiesState<T, I>>::get();

		if on_authorities_change {
			if deadline == at {
				let (_, signatures) = <AuthoritiesToSign<T, I>>::get();

				find_and_slash_misbehavior(signatures);

				let submit_duration = T::SubmitDuration::get();

				<AuthoritiesState<T, I>>::put((
					true,
					<frame_system::Module<T>>::block_number() + submit_duration,
				));
				<SubmitDuration<T, I>>::mutate(|submit_duration_| {
					*submit_duration_ += submit_duration
				});
			}
		} else {
			if let Some(signatures) = <MMRRootsToSign<T, I>>::take(
				<frame_system::Module<T>>::block_number() - <SubmitDuration<T, I>>::get(),
			) {
				find_and_slash_misbehavior(signatures);

				// TODO: delay or discard?
			}
		}
	}
}

impl<T, I> RelayAuthorityProtocol<BlockNumber<T>> for Module<T, I>
where
	T: Trait<I>,
	I: Instance,
{
	fn new_mmr_to_sign(block_number: BlockNumber<T>) {
		let _ = <MMRRootsToSign<T, I>>::try_mutate(block_number, |signed_mmr_root| {
			// No-op if the sign was already scheduled
			if signed_mmr_root.is_some() {
				return Err(());
			}

			<MMRRootsToSignKeys<T, I>>::append(block_number);

			*signed_mmr_root = Some(<Vec<(AccountId<T>, RelayAuthoritySignature<T, I>)>>::new());

			Self::deposit_event(RawEvent::NewMMRRoot(block_number));

			Ok(())
		});
	}

	fn finish_authorities_change() {
		<AuthoritiesState<T, I>>::kill();
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
) -> Option<RelayAuthoritySigner<T, I>>
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

#[derive(Encode)]
struct _S<_1, _2, _3>
where
	_1: Encode,
	_2: Encode,
	_3: Encode,
{
	_1: _1,
	#[codec(compact)]
	_2: _2,
	_3: _3,
}
