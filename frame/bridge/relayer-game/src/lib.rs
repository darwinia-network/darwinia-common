//! # Relayer Game Module
//!
//! ## Assumption
//! 1. At least **one** honest relayer
//! 2. Each proposal's header hash is unique at a certain block height
//!
//!
//! ## Flow
//! 1. Request the header in target chain's relay module,
//! weather the header is existed or not you should pay some fees
//! 2. If not, target chain's relay module will ask for a proposal here

#![cfg_attr(not(feature = "std"), no_std)]
#![feature(drain_filter)]

// FIXME: separate long function into several functions

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

mod types {
	// --- darwinia ---
	use crate::*;

	pub type AccountId<T> = <T as frame_system::Trait>::AccountId;
	pub type BlockNumber<T> = <T as frame_system::Trait>::BlockNumber;

	pub type RingBalance<T, I> = <RingCurrency<T, I> as Currency<AccountId<T>>>::Balance;
	pub type RingNegativeImbalance<T, I> =
		<RingCurrency<T, I> as Currency<AccountId<T>>>::NegativeImbalance;

	pub type TcBlockNumber<T, I> = <Tc<T, I> as Relayable>::TcBlockNumber;
	pub type TcHeaderHash<T, I> = <Tc<T, I> as Relayable>::TcHeaderHash;
	pub type TcHeaderMMR<T, I> = <Tc<T, I> as Relayable>::TcHeaderMMR;

	pub type GameId<TcBlockNumber> = TcBlockNumber;

	type RingCurrency<T, I> = <T as Trait<I>>::RingCurrency;

	type Tc<T, I> = <T as Trait<I>>::TargetChain;
}

// --- crates ---
use codec::{Decode, Encode};
// --- substrate ---
use frame_support::{
	debug::{error, info},
	decl_error, decl_event, decl_module, decl_storage, ensure,
	storage::IterableStorageMap,
	traits::{Currency, ExistenceRequirement, Get, OnUnbalanced},
	weights::Weight,
};
use sp_runtime::{
	traits::{CheckedSub, SaturatedConversion, Saturating, Zero},
	DispatchResult, RuntimeDebug,
};
#[cfg(not(feature = "std"))]
use sp_std::borrow::ToOwned;
use sp_std::{collections::btree_map::BTreeMap, prelude::*};
// --- darwinia ---
use darwinia_support::{balance::lock::*, relay::*};
use types::*;

pub const MAX_ACTIVE_GAMES: usize = 32;
pub const RELAYER_GAME_ID: LockIdentifier = *b"da/rgame";

pub trait Trait<I: Instance = DefaultInstance>: frame_system::Trait {
	type Event: From<Event<Self, I>> + Into<<Self as frame_system::Trait>::Event>;

	/// The currency use for bond
	type RingCurrency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

	/// Handler for the unbalanced *RING* reduction when slashing a relayer.
	type RingSlash: OnUnbalanced<RingNegativeImbalance<Self, I>>;

	/// A regulator to adjust relay args for a specific chain
	type RelayerGameAdjustor: AdjustableRelayerGame<
		Balance = RingBalance<Self, I>,
		Moment = Self::BlockNumber,
		TcBlockNumber = TcBlockNumber<Self, I>,
	>;

	/// The target chain's relay module's API
	type TargetChain: Relayable;

	type ConfirmPeriod: Get<Self::BlockNumber>;

	/// Weight information for extrinsics in this pallet.
	type WeightInfo: WeightInfo;
}

decl_event! {
	pub enum Event<T, I: Instance = DefaultInstance>
	where
		TcBlockNumber = TcBlockNumber<T, I>,
		GameId = GameId<TcBlockNumber<T, I>>,
	{
		/// A new round started. [game id, samples, mmr members]
		NewRound(GameId, Vec<TcBlockNumber>, Vec<TcBlockNumber>),

		/// A game has been settled. [game id]
		GameOver(GameId),

		/// Pending header approved. [block number, reason]
		PendingHeaderApproved(TcBlockNumber, Vec<u8>),
		/// Pending header rejected. [block number]
		PendingHeaderRejected(TcBlockNumber),
	}
}

decl_error! {
	pub enum Error for Module<T: Trait<I>, I: Instance> {
		/// Active Game - TOO MANY
		ActiveGameTM,

		/// Can not bond with value less than usable balance.
		InsufficientBond,

		/// Pending Header - NOT FOUND
		PendingHeaderNF,

		/// Proposal - INVALID
		ProposalI,

		/// Proposal - ALREADY EXISTED
		ProposalAE,

		/// Round - MISMATCHED
		RoundMis,

		/// Target Header - ALREADY CONFIRMED
		TargetHeaderAC,
	}
}

decl_storage! {
	trait Store for Module<T: Trait<I>, I: Instance = DefaultInstance> as DarwiniaRelayerGame {
		/// All the proposals here per game
		pub Proposals
			get(fn proposals_of_game)
			: map hasher(blake2_128_concat) GameId<TcBlockNumber<T, I>>
			=> Vec<Proposal<
				AccountId<T>,
				BondedTcHeader<
					RingBalance<T, I>,
					TcHeaderBrief<TcBlockNumber<T, I>, TcHeaderHash<T, I>, TcHeaderMMR<T, I>>
				>,
				TcHeaderHash<T, I>
			>>;

		/// All the proposal relay headers(not brief) here per game
		pub Headers
			get(fn header_of_game_with_hash)
			: double_map
				hasher(blake2_128_concat) GameId<TcBlockNumber<T, I>>,
				hasher(blake2_128_concat) TcHeaderHash<T, I>
			=>  RawHeaderThing;

		/// The last confirmed block number record of a game when it start
		pub LastConfirmeds
			get(fn last_confirmed_of_game)
			: map hasher(blake2_128_concat) GameId<TcBlockNumber<T, I>>
			=> TcBlockNumber<T, I>;

		/// The allow samples for each game
		pub Samples
			get(fn samples_of_game)
			: map hasher(blake2_128_concat) TcBlockNumber<T, I>
			=> Vec<Vec<TcBlockNumber<T, I>>>;

		/// The closed rounds which had passed the challenge time at this moment
		pub ClosedRounds
			get(fn closed_rounds_at)
			: map hasher(blake2_128_concat) BlockNumber<T>
			=> Vec<(GameId<TcBlockNumber<T, I>>, Round)>;

		/// All the bonds per relayer
		pub Bonds
			get(fn bonds_of_relayer)
			: map hasher(blake2_128_concat) AccountId<T>
			=> RingBalance<T, I>;


		// TODO: move into relay
		// TODO: reject submit if the block number already on pending?
		/// Dawinia Relay Guard System
		///
		/// https://github.com/darwinia-network/darwinia-common/issues/150
		pub PendingHeaders
			get(fn pending_headers)
			: Vec<(BlockNumber<T>, TcBlockNumber<T, I>, RawHeaderThing)>;
	}
}

decl_module! {
	pub struct Module<T: Trait<I>, I: Instance = DefaultInstance> for enum Call
	where
		origin: T::Origin
	{
		type Error = Error<T, I>;

		fn deposit_event() = default;

		fn on_initialize(block_number: BlockNumber<T>) -> Weight {
			<PendingHeaders<T, I>>::mutate(|pending_headers|
				pending_headers.retain(|(confirm_at, pending_block_number, pending_header)|
					if *confirm_at == block_number {
						if let Err(_) = T::TargetChain::store_header(pending_header.to_owned()) {
							// TODO: handle error
						} else {
							Self::deposit_event(RawEvent::PendingHeaderApproved(
								*pending_block_number,
								b"Not Enough Coucil Online, Approved By System".to_vec()
							));
						}

						false
					} else {
						true
					}
				)
			);

			0
		}

		// TODO: too many db operations and calc need to move to `offchain_worker`
		// TODO: close the game that its id less than the best number
		fn on_finalize(block_number: BlockNumber<T>) {
			let closed_rounds = <ClosedRounds<T, I>>::take(block_number);

			// `closed_rounds` MUST NOT be empty after this check; qed
			if closed_rounds.len() == 0 {
				return;
			}

			info!(target: "relayer-game", "Found Closed Rounds at `{:?}`", block_number);
			info!(target: "relayer-game", "---");

			let mut pending_headers = vec![];

			let proposals_filter = |round, proposals: &mut Vec<Proposal<_, _, _>>| {
				proposals
					.drain_filter(|proposal|
						T::RelayerGameAdjustor
							::round_from_chain_len(proposal.bonded_chain.len() as _) == round
					)
					.collect::<Vec<_>>()
			};
			let settle_without_challenge = |
				game_id,
				proposal: Proposal<_, _, _>,
				pending_headers: &mut Vec<_>
			| {
				let BondedTcHeader::<_, _> {
					header_brief: TcHeaderBrief::<_, _, _> { hash, .. },
					bond
				} = &proposal.bonded_chain[0];

				Self::update_bonds_with(
					&proposal.relayer,
					|old_bonds| old_bonds.saturating_sub(*bond)
				);

				// TODO: reward if no challenge

				pending_headers.push((
					game_id,
					Self::header_of_game_with_hash(game_id, hash)
				));

				<Samples<T, I>>::take(game_id);
				<LastConfirmeds<T, I>>::take(game_id);
				<Headers<T, I>>::remove_prefix(game_id);
				<Proposals<T, I>>::take(game_id);

				Self::deposit_event(RawEvent::GameOver(game_id));
			};
			let settle_with_challenge = |
				game_id,
				mut extend_at,
				confirmed_proposal: Proposal<
					_,
					BondedTcHeader<_, TcHeaderBrief<_, TcHeaderHash<T, I>, _>>,
					_
				>,
				mut rewards: Vec<_>,
				mut proposals: Vec<_>,
				pending_headers: &mut Vec<_>
			| {
				let mut extend_from_header_hash
					= confirmed_proposal.extend_from_header_hash.unwrap();

				// If there's no extended at first round,
				// that means this proposal MUST be the first proposal
				// Else,
				// it MUST extend from some; qed
				while extend_at > 0 {
					extend_at -= 1;

					let mut maybe_honesty = None;
					let mut evils = vec![];

					for proposal in proposals_filter(extend_at, &mut proposals) {
						let BondedTcHeader::<_, TcHeaderBrief<_, TcHeaderHash<T, I>, _>> {
							header_brief,
							bond
						} = proposal.bonded_chain.last().unwrap();
						let header_hash = header_brief.hash.clone();

						if header_hash == extend_from_header_hash {
							if let Some(header_hash) = proposal.extend_from_header_hash {
								extend_from_header_hash = header_hash;
							}

							if maybe_honesty.is_none() {
								maybe_honesty = Some((proposal.relayer, *bond));
							} else {
								error!("Honest Relayer Count - MORE THAN 1 WITHIN A ROUND");
							}
						} else {
							evils.push((proposal.relayer, *bond));
						}
					}

					if let Some(honesty) = maybe_honesty {
						for evil in evils {
							rewards.push((honesty.to_owned(), evil));
						}
					} else {
						// Should NEVER enter this condition
						for (evil, evil_bonds) in evils {
							Self::update_bonds_with(
								&evil,
								|old_bonds| old_bonds.saturating_sub(evil_bonds)
							);

							let (imbalance, _) = T::RingCurrency::slash(&evil, evil_bonds);
							T::RingSlash::on_unbalanced(imbalance);
						}

						error!("Honest Relayer - NOT FOUND");
					}
				}

				// Use for updating relayers' bonds and locks with just 2 DB writes
				let mut honesties_map = BTreeMap::new();
				// Use for updating evils' bonds, locks and reward relayers
				let mut evils_map = BTreeMap::new();

				for ((honesty, relayer_bond), (evil, evil_bond)) in rewards {
					*honesties_map.entry(honesty.clone()).or_insert(relayer_bond)
						+= relayer_bond;

					{
						let evil_map_ptr = evils_map.entry(evil).or_insert({
							let mut honesties_map = BTreeMap::new();

							honesties_map.insert(honesty.clone(), evil_bond);

							// The first item means total bonds
							// which use for updating bonds and locks with just 2 DB writes
							//
							// The second item use for rewarding relayers
							(evil_bond, honesties_map)
						});

						evil_map_ptr.0 += evil_bond;
						*evil_map_ptr.1.entry(honesty).or_insert(evil_bond) += evil_bond;
					}
				}

				for (honesty, honesty_bonds) in honesties_map {
					Self::update_bonds_with(
						&honesty,
						|old_bonds| old_bonds.saturating_sub(honesty_bonds)
					);
				}

				for (evil, (evil_bonds, honesties_map)) in evils_map {
					Self::update_bonds_with(
						&evil,
						|old_bonds| old_bonds.saturating_sub(evil_bonds)
					);

					if honesties_map.is_empty() {
						Self::update_bonds_with(
							&evil,
							|old_bonds| old_bonds.saturating_sub(evil_bonds)
						);

						let (imbalance, _) = T::RingCurrency::slash(&evil, evil_bonds);
						T::RingSlash::on_unbalanced(imbalance);
					} else {
						for (honesty, evil_bonds) in honesties_map {
							let _ = T::RingCurrency::transfer(
								&evil,
								&honesty,
								evil_bonds,
								ExistenceRequirement::KeepAlive
							);
						}
					}
				}

				// TODO: reward if no challenge

				pending_headers.push((
					game_id,
					Self::header_of_game_with_hash(
						game_id,
						confirmed_proposal.bonded_chain[0].header_brief.hash.clone()
					)
				));

				<Samples<T, I>>::take(game_id);
				<LastConfirmeds<T, I>>::take(game_id);
				<Headers<T, I>>::remove_prefix(game_id);
				<Proposals<T, I>>::take(game_id);

				Self::deposit_event(RawEvent::GameOver(game_id));
			};
			let settle_abandon = |
				proposals: Vec<Proposal<_, BondedTcHeader<RingBalance<T, I>, _>, _>>
			| {
				for proposal in proposals {
					let bonds = proposal
						.bonded_chain
						.iter()
						.fold(Zero::zero(), |bonds, bonded_header| bonds + bonded_header.bond);

					Self::update_bonds_with(
						&proposal.relayer,
						|old_bonds| old_bonds.saturating_sub(bonds)
					);

					let (imbalance, _) = T::RingCurrency::slash(&proposal.relayer, bonds);
					T::RingSlash::on_unbalanced(imbalance);
				}
			};
			let on_chain_arbitrate = |
				game_id,
				last_round,
				last_round_proposals: Vec<Proposal<_, _, _>>,
				proposals: Vec<_>,
				pending_headers: &mut Vec<_>
			| {
				let mut maybe_confirmed_proposal: Option<Proposal<AccountId<T>, _, _>> = None;
				let mut evils = vec![];

				for proposal in last_round_proposals.iter() {
					if T::TargetChain::on_chain_arbitrate(proposal
						.bonded_chain
						.iter()
						.map(|BondedTcHeader::<_, TcHeaderBrief<_, _, _>> { header_brief, .. }|
							header_brief.clone())
						.collect()).is_ok()
					{
						if maybe_confirmed_proposal.is_none() {
							maybe_confirmed_proposal = Some(proposal.to_owned());
						} else {
							error!("Honest Relayer Count - MORE THAN 1 WITHIN A ROUND");
						}
					} else {
						evils.push((
							proposal.relayer.clone(),
							proposal.bonded_chain.last().unwrap().bond
						));
					}
				}

				if let Some(confirmed_proposal) = maybe_confirmed_proposal {
					let rewards = evils
						.into_iter()
						.map(|evil| (
							(
								confirmed_proposal.relayer.clone(),
								confirmed_proposal.bonded_chain.last().unwrap().bond
							),
							evil
						))
						.collect();

					settle_with_challenge(
						game_id,
						last_round,
						confirmed_proposal,
						rewards,
						proposals,
						pending_headers
					);
				} else {
					info!(target: "relayer-game", "   >  No Honest Relayer");

					settle_abandon(last_round_proposals);
				}

				<Samples<T, I>>::take(game_id);
				<LastConfirmeds<T, I>>::take(game_id);
				<Headers<T, I>>::remove_prefix(game_id);
				<Proposals<T, I>>::take(game_id);

				Self::deposit_event(RawEvent::GameOver(game_id));
			};
			let update_samples = |game_id| {
				<Samples<T, I>>::mutate(game_id, |samples| {
						T::RelayerGameAdjustor::update_samples(samples);

						if samples.len() < 2 {
							error!("Sample Points MISSING, \
								Check Your Sample Strategy Implementation");

							return;
						}

						Self::deposit_event(RawEvent::NewRound(
							game_id,
							samples.concat(),
							samples[samples.len() - 1].clone(),
						));
					}
				);
			};

			for (game_id, last_round) in closed_rounds {
				info!(target: "relayer-game", ">  Trying to Settle Game `{:?}` at Round `{}`", game_id, last_round);

				let mut proposals = Self::proposals_of_game(game_id);

				match proposals.len() {
					0 => info!(target: "relayer-game", "   >  No Proposal Found"),
					1 => {
						info!(target: "relayer-game", "   >  No Challenge Found");

						settle_without_challenge(
							game_id,
							proposals.pop().unwrap(),
							&mut pending_headers
						);
					}
					_ => {
						let last_round_proposals = proposals_filter(last_round, &mut proposals);

						match last_round_proposals.len() {
							0 => {
								info!(target: "relayer-game", "   >  All Relayers Abstain");

								// `last_round` MUST NOT be `0`; qed
								settle_abandon(proposals_filter(last_round - 1, &mut proposals));
							}
							1 => {
								let mut last_round_proposals = last_round_proposals;

								settle_with_challenge(
									game_id,
									last_round,
									last_round_proposals.pop().unwrap(),
									vec![],
									proposals,
									&mut pending_headers
								);
							}
							_ => {
								let relay_target = last_round_proposals[0]
									.bonded_chain[0]
									.header_brief
									.number;
								let last_round_proposals_chain_len =
									last_round_proposals[0].bonded_chain.len();
								let full_chain_len =
									(relay_target - Self::last_confirmed_of_game(game_id))
										.saturated_into() as u64;

								if last_round_proposals_chain_len as u64 == full_chain_len {
									info!(target: "relayer-game", "   >  On Chain Arbitrate");

									on_chain_arbitrate(
										game_id,
										last_round,
										last_round_proposals,
										proposals,
										&mut pending_headers
									);
								} else {
									info!(target: "relayer-game", "   >  Update Samples");

									update_samples(relay_target);

									let round = last_round + 1;
									let closed_at = block_number
										+ T::RelayerGameAdjustor::challenge_time(round);

									<ClosedRounds<T, I>>::append(closed_at, (game_id, round));
								}
							}
						}
					}
				}
			}

			let confirm_period = T::ConfirmPeriod::get();

			if confirm_period.is_zero() {
				for (_, pending_header) in pending_headers {
					// TODO: handle error
					let _ = T::TargetChain::store_header(pending_header);
				}
			} else {
				for (pending_block_number, pending_header) in pending_headers {
					<PendingHeaders<T, I>>::append((
						block_number + confirm_period,
						pending_block_number,
						pending_header
					));
				}
			}

			info!(target: "relayer-game", "---");
		}

	}
}

impl<T: Trait<I>, I: Instance> Module<T, I> {
	fn update_bonds_with<F>(relayer: &AccountId<T>, calc_bonds: F)
	where
		F: FnOnce(RingBalance<T, I>) -> RingBalance<T, I>,
	{
		let bonds = calc_bonds(Self::bonds_of_relayer(relayer));

		if bonds.is_zero() {
			T::RingCurrency::remove_lock(RELAYER_GAME_ID, relayer);

			<Bonds<T, I>>::take(relayer);
		} else {
			T::RingCurrency::set_lock(
				RELAYER_GAME_ID,
				relayer,
				LockFor::Common { amount: bonds },
				WithdrawReasons::all(),
			);

			<Bonds<T, I>>::insert(relayer, bonds);
		}
	}

	fn update_pending_headers_with<F>(
		pending_block_number: TcBlockNumber<T, I>,
		f: F,
	) -> DispatchResult
	where
		F: FnOnce(RawHeaderThing) -> DispatchResult,
	{
		<PendingHeaders<T, I>>::mutate(|pending_headers| {
			if let Some(i) = pending_headers
				.iter()
				.position(|(_, block_number, _)| *block_number == pending_block_number)
			{
				let (_, _, header) = pending_headers.remove(i);

				f(header)
			} else {
				Err(<Error<T, I>>::PendingHeaderNF)?
			}
		})
	}
}

impl<T: Trait<I>, I: Instance> RelayerGameProtocol for Module<T, I> {
	type Relayer = AccountId<T>;
	type TcBlockNumber = TcBlockNumber<T, I>;

	// TODO:
	//	The `header_thing_chain` could be very large,
	//	the bond should relate to the bytes fee
	//	that we slash the evil relayer(s) to reward the honest relayer(s) (economic optimize)
	//
	// TODO: compact params? (efficency optimize)
	//
	// TODO: check too far from last confirmed? maybe we can submit some check point (efficency optimize)
	//
	// TODO: drop previous rounds' proof (efficency optimize)
	//
	// TODO: handle uncle block
	fn submit_proposal(
		relayer: Self::Relayer,
		raw_header_thing_chain: Vec<RawHeaderThing>,
	) -> DispatchResult {
		ensure!(!raw_header_thing_chain.is_empty(), <Error<T, I>>::ProposalI);

		let (game_id, proposed_header_hash, proposed_raw_header) = {
			let (proposed_header_brief, proposed_raw_header) =
				T::TargetChain::verify_raw_header_thing(raw_header_thing_chain[0].clone(), true)?;

			(
				proposed_header_brief.number,
				proposed_header_brief.hash,
				proposed_raw_header,
			)
		};
		let best_block_number = T::TargetChain::best_block_number();
		let other_proposals = Self::proposals_of_game(game_id);
		let other_proposals_len = other_proposals.len();
		let extend_bonded_chain = |chain: &[_], extend_at| {
			let mut bonds = <RingBalance<T, I>>::zero();
			let extend_chain = chain
				.iter()
				.cloned()
				.enumerate()
				.map(|(round_offset, header_brief)| {
					let bond = T::RelayerGameAdjustor::estimate_bond(
						extend_at + round_offset as Round,
						other_proposals_len as _,
					);

					bonds = bonds.saturating_add(bond);

					BondedTcHeader { header_brief, bond }
				})
				.collect::<Vec<_>>();

			(bonds, extend_chain)
		};

		info!(target: "relayer-game", "Relayer `{:?}` Submit a Proposal: ", relayer);

		// TODO: accept a chain (length > 1) but without extend
		match (other_proposals_len, raw_header_thing_chain.len()) {
			// New `Game`
			(0, raw_header_thing_chain_len) => {
				ensure!(
					<Proposals<T, I>>::iter().count() <= MAX_ACTIVE_GAMES,
					<Error<T, I>>::ActiveGameTM
				);
				ensure!(game_id > best_block_number, <Error<T, I>>::TargetHeaderAC);
				ensure!(raw_header_thing_chain_len == 1, <Error<T, I>>::RoundMis);

				let chain = T::TargetChain::verify_raw_header_thing_chain(raw_header_thing_chain)?;
				info!(target: "relayer-game", "{:#?}", chain);
				let (bonds, bonded_chain) = extend_bonded_chain(&chain, 0);

				{
					let use_for_bonds = T::RingCurrency::usable_balance(&relayer)
						.checked_sub(&T::RingCurrency::minimum_balance())
						.ok_or(<Error<T, I>>::InsufficientBond)?;
					ensure!(use_for_bonds >= bonds, <Error<T, I>>::InsufficientBond);
				}

				Self::update_bonds_with(&relayer, |old_bonds| old_bonds.saturating_add(bonds));

				<ClosedRounds<T, I>>::append(
					<frame_system::Module<T>>::block_number()
						+ T::RelayerGameAdjustor::challenge_time(0),
					(game_id, 0),
				);
				<Samples<T, I>>::append(game_id, vec![game_id]);
				<LastConfirmeds<T, I>>::insert(game_id, best_block_number);
				<Headers<T, I>>::insert(game_id, proposed_header_hash, proposed_raw_header);
				<Proposals<T, I>>::append(
					game_id,
					Proposal {
						relayer,
						bonded_chain,
						extend_from_header_hash: None,
					},
				);
			}
			// First round
			(_, 1) => {
				if other_proposals
					.iter()
					.any(|proposal| proposal.bonded_chain.len() != 1)
				{
					Err(<Error<T, I>>::RoundMis)?;
				}

				let chain = T::TargetChain::verify_raw_header_thing_chain(raw_header_thing_chain)?;
				info!(target: "relayer-game", "{:#?}", chain);

				ensure!(
					!other_proposals
						.into_iter()
						.all(|proposal| &proposal.bonded_chain[0].header_brief == &chain[0]),
					<Error<T, I>>::ProposalAE
				);

				let (bonds, bonded_chain) = extend_bonded_chain(&chain, 0);

				ensure!(
					(T::RingCurrency::usable_balance(&relayer)
						- T::RingCurrency::minimum_balance())
						>= bonds,
					<Error<T, I>>::InsufficientBond
				);

				Self::update_bonds_with(&relayer, |old_bonds| old_bonds.saturating_add(bonds));

				<Headers<T, I>>::insert(game_id, proposed_header_hash, proposed_raw_header);
				<Proposals<T, I>>::append(
					game_id,
					Proposal {
						relayer,
						bonded_chain,
						extend_from_header_hash: None,
					},
				);
			}
			// Extend
			(_, raw_header_thing_chain_len) => {
				let round =
					T::RelayerGameAdjustor::round_from_chain_len(raw_header_thing_chain_len as _);
				let prev_round = round.checked_sub(1).ok_or(<Error<T, I>>::RoundMis)?;
				let chain = T::TargetChain::verify_raw_header_thing_chain(raw_header_thing_chain)?;
				info!(target: "relayer-game", "{:#?}", chain);
				let samples = {
					// Chain's len is ALWAYS great than 1 under this match pattern; qed
					let game_id = chain[0].number;

					Self::samples_of_game(game_id).concat()
				};

				ensure!(chain.len() == samples.len(), <Error<T, I>>::RoundMis);
				ensure!(
					chain
						.iter()
						.zip(samples.iter())
						.all(|(header_thing, sample_block_number)| header_thing.number
							== *sample_block_number),
					<Error<T, I>>::RoundMis
				);

				let extend_at = T::RelayerGameAdjustor::chain_len_from_round(prev_round) as _;
				let (bonds, extend_chain) = extend_bonded_chain(&chain[extend_at..], prev_round);
				let mut extend_from_proposal = None;

				for proposal in other_proposals {
					let proposal_chain_len = proposal.bonded_chain.len();

					if proposal_chain_len == extend_at {
						if chain[..extend_at]
							.iter()
							.zip(proposal.bonded_chain.iter())
							.all(|(a, b)| a == &b.header_brief)
						{
							extend_from_proposal = Some(proposal);
						}
					} else if proposal_chain_len == chain.len() {
						ensure!(
							!extend_chain
								.iter()
								.zip(proposal.bonded_chain[extend_at..].iter())
								.all(|(a, b)| a.header_brief == b.header_brief),
							<Error<T, I>>::ProposalAE
						);
					}
				}

				if let Some(Proposal {
					bonded_chain: extend_from_chain,
					..
				}) = extend_from_proposal
				{
					ensure!(
						(T::RingCurrency::usable_balance(&relayer)
							- T::RingCurrency::minimum_balance())
							>= bonds,
						<Error<T, I>>::InsufficientBond
					);

					let extend_from_header = extend_from_chain.last().unwrap().header_brief.clone();
					let bonded_chain = [extend_from_chain, extend_chain].concat();

					Self::update_bonds_with(&relayer, |old_bonds| old_bonds.saturating_add(bonds));

					<Proposals<T, I>>::append(
						game_id,
						Proposal {
							relayer,
							bonded_chain,
							// Each proposal MUST contains a NOT empty chain; qed
							extend_from_header_hash: Some(extend_from_header.hash),
						},
					);
				} else {
					Err(<Error<T, I>>::RoundMis)?;
				}
			}
		}

		Ok(())
	}

	fn approve_pending_header(pending: Self::TcBlockNumber) -> DispatchResult {
		Self::update_pending_headers_with(pending, |header| T::TargetChain::store_header(header))?;
		Self::deposit_event(RawEvent::PendingHeaderApproved(
			pending,
			b"Approved By Council".to_vec(),
		));

		Ok(())
	}

	fn reject_pending_header(pending: Self::TcBlockNumber) -> DispatchResult {
		Self::update_pending_headers_with(pending, |_| Ok(()))?;
		Self::deposit_event(RawEvent::PendingHeaderRejected(pending));

		Ok(())
	}
}

// TODO: https://github.com/darwinia-network/darwinia-common/issues/209
pub trait WeightInfo {}
impl WeightInfo for () {}

#[derive(Clone, Encode, Decode, RuntimeDebug)]
pub struct Proposal<AccountId, BondedTcHeader, TcHeaderHash> {
	// TODO: Can this proposal submit by other relayers?
	/// The relayer of these series of headers
	/// The proposer of this proposal
	/// The person who support this proposal with some bonds
	relayer: AccountId,
	/// A series of target chain's header ids and the value that relayer had bonded for it
	bonded_chain: Vec<BondedTcHeader>,
	/// Parents (previous header hash)
	///
	/// If this field is `None` that means this proposal is the first proposal
	extend_from_header_hash: Option<TcHeaderHash>,
}

#[derive(Clone, Encode, Decode, RuntimeDebug)]
pub struct BondedTcHeader<Balance, TcHeaderBrief> {
	header_brief: TcHeaderBrief,
	bond: Balance,
}
