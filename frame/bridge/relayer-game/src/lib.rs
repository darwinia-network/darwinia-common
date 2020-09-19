//! # Relayer Game Module

#![cfg_attr(not(feature = "std"), no_std)]

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

	pub type TcHeaderThingWithProof<T, I> = <Tc<T, I> as Relayable>::HeaderThingWithProof;
	pub type TcHeaderThing<T, I> = <Tc<T, I> as Relayable>::HeaderThing;
	pub type TcBlockNumber<T, I> = <TcHeaderThing<T, I> as HeaderThing>::Number;
	pub type TcHeaderHash<T, I> = <TcHeaderThing<T, I> as HeaderThing>::Hash;

	pub type GameId<TcBlockNumber> = TcBlockNumber;

	pub type RelayProposalT<T, I> =
		RelayProposal<AccountId<T>, RingBalance<T, I>, TcHeaderThing<T, I>, TcHeaderHash<T, I>>;

	type RingCurrency<T, I> = <T as Trait<I>>::RingCurrency;

	type Tc<T, I> = <T as Trait<I>>::TargetChain;
}

// --- substrate ---
use frame_support::{
	debug::{error, info},
	decl_error, decl_event, decl_module, decl_storage, ensure,
	storage::IterableStorageMap,
	traits::{Currency, ExistenceRequirement, Get, OnUnbalanced},
	weights::Weight,
};
use sp_runtime::{
	traits::{SaturatedConversion, Saturating, Zero},
	DispatchError, DispatchResult,
};
#[cfg(not(feature = "std"))]
use sp_std::borrow::ToOwned;
use sp_std::prelude::*;
// --- darwinia ---
use darwinia_relay_primitives::*;
use darwinia_support::balance::lock::*;
use types::*;

pub const MAX_ACTIVE_GAMES: usize = 32;
pub const RELAYER_GAME_ID: LockIdentifier = *b"da/rgame";

pub struct Game<T, I> {
	id: TcBlockNumber<T, I>,
	current_round: Round<T, I>,
	rounds: Vec<Round<T, I>>,
	state: Option<GameState>,
	last_confirmed: TcBlockNumber<T, I>
}

impl<T, I> Game<T, I> {
	pub fn new(id: TcBlockNumber<T, I>, samples: Vec<TcBlockNumber<T, I>>, proposals: Vec<RelayProposalT<T, I>>, last_confirmed: TcBlockNumber<T, I>) -> Game<T, I> {
		let round_number = 0;
		let round = Round {
			number: round_number,
			game_id: id,
			samples,
			proposals,
			state: RoundState::Open,
		};
		let mut game = Game {
			id,
			current_round: round,
			rounds: vec![round],
			state: None,
			last_confirmed
		};
		game.set_state(GameState::WaitingChallenges);
		game
	}

	pub fn set_state(&mut self, new_state: GameState) {
		let ori_state = &self.state;
		self.state = new_state.clone();
		match ori_state {
			None => self.on_created(),
			Some(old_state) => {
				if old_state == GameState::WaitingChallenges && new_state == GameState::Settled(SettleReason::NoChallenge) {
					self.on_settled(old_state, &new_state);
				} else if old_state == GameState::WaitingChallenges && new_state == GameState::WaitingProofs {
					self.on_challenged(old_state, &new_state);
					self.on_started(old_state, &new_state);
					self.on_new_round(old_state, &new_state);
				} else if old_state == GameState::WaitingProofs && new_state == GameState::Settled(SettleReason::AllAbstain) {
					self.on_settled(old_state, &new_state);
				} else if old_state == GameState::WaitingProofs && new_state == GameState::Settled(SettleReason::OnlyOneProof) {
					self.on_settled(old_state, &new_state);
				} else if old_state == GameState::WaitingProofs && new_state == GameState::WaitingProofs {
					self.on_new_round(old_state, &new_state);
				} else {
					self.unsupported(old_state, &new_state);
				}
			}
		}
	}

	fn on_created(&self) {

	}

	fn on_challenged(&self, old_state: &GameState, new_state: &GameState) {

	}

	fn on_started(&self, old_state: &GameState, new_state: &GameState) {

	}

	fn on_new_round(&self, old_state: &GameState, new_state: &GameState) {

	}

	fn on_settled(&self, old_state: &GameState, new_state: &GameState) {

	}

	fn unsupported(&self, old_state: &GameState, new_state: &GameState) {

	}

}

pub enum SettleReason {
	NoChallenge,
	AllAbstain,
	OnlyOneProof,
}
pub enum GameState {
	WaitingChallenges,
	WaitingProofs,
	Settled(SettleReason),
	Unknown
}

pub struct Round<T, I> {
	game_id: TcBlockNumber<T, I>,
	number: RoundNumber,
	samples: Vec<TcBlockNumber<T, I>>,
	proposals: Vec<RelayProposalT<T, I>>,
	state: RoundState,
}

pub enum RoundState {
	Open,
	Closed
}

impl<T, I> Round<T, I> {
	pub fn new(game_id: TcBlockNumber<T, I>, number: RoundNumber, samples: Vec<TcBlockNumber<T, I>>) -> Round<T, I> {
		Round {
			game_id,
			number,
			samples,
			proposals: vec![],
			state: RoundState::Open,
		}
	}

	pub fn add_proposal(&mut self, proposal: RelayProposalT<T, I>) {
		self.proposals.push(proposal);
	}

	pub fn next(&self) -> Round<T, I> {
		let mut samples = self.samples.clone();
		T::RelayerGameAdjustor::update_samples(&samples);
		Round {
			game_id: self.game_id,
			number: self.number + 1,
			samples: samples,
			proposals: vec![],
			state: RoundState::Open,
		}
	}

	pub fn set_state(&self, new_state: RoundState) {
		let game = T::game_by_id(self.game_id);

		if new_state == RoundState::Closed {
			if self.number == 0 { // round 0 closed
				if self.proposals.len() == 1 {
					game.set_state(GameState::Settled(SettleReason::NoChallenge));
				} else { // proposals > 1
					game.set_state(GameState::WaitingProofs);
				}
			} else { // round >= 1 closed
				if self.proposals.len() == 0 {
					game.set_state(GameState::Settled(SettleReason::AllAbstain));
				} else if self.proposals.len() == 1 {
					game.set_state(GameState::Settled(SettleReason::OnlyOneProof));
				} else {
					game.set_state(GameState::WaitingProofs);
				}
			}
		}
	}
}

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
		pub Games get(fn game_by_id)
			: map hasher(blake2_128_concat) GameId<TcBlockNumber<T, I>>
			=> Option<Game<T, I>>;

		pub ClosedRounds
			get(fn closed_rounds_at)
			: map hasher(blake2_128_concat) BlockNumber<T>
			=> Vec<Round>;

		/// All the proposals here per game
		pub Proposals
			get(fn proposals_of_game)
			: map hasher(blake2_128_concat) GameId<TcBlockNumber<T, I>>
			=> Vec<RelayProposalT<T, I>>;

		/// All the proposal relay headers(not brief) here per game
		pub Headers
			get(fn header_of_game_with_hash)
			: double_map
				hasher(blake2_128_concat) GameId<TcBlockNumber<T, I>>,
				hasher(blake2_128_concat) TcHeaderHash<T, I>
			=>  TcHeaderThing<T, I>;

		/// The last confirmed block number record of a game when it start
		// pub LastConfirmeds
		// 	get(fn last_confirmed_of_game)
		// 	: map hasher(blake2_128_concat) GameId<TcBlockNumber<T, I>>
		// 	=> TcBlockNumber<T, I>;

		/// The allow samples for each game
		// pub Samples
		// 	get(fn samples_of_game)
		// 	: map hasher(blake2_128_concat) TcBlockNumber<T, I>
		// 	=> Vec<Vec<TcBlockNumber<T, I>>>;

		/// The closed rounds which had passed the challenge time at this moment
		// pub ClosedRounds
		// 	get(fn closed_rounds_at)
		// 	: map hasher(blake2_128_concat) BlockNumber<T>
		// 	=> Vec<(GameId<TcBlockNumber<T, I>>, Round)>;

		/// All the bonds per relayer
		pub Bonds
			get(fn bonds_of_relayer)
			: map hasher(blake2_128_concat) AccountId<T>
			=> RingBalance<T, I>;


		// TODO: reject submit if the block number already on pending?
		/// Dawinia Relay Guard System
		pub PendingHeaders
			get(fn pending_headers)
			: Vec<(BlockNumber<T>, TcBlockNumber<T, I>, TcHeaderThing<T, I>)>;
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
			if let Ok(weight) = Self::system_approve_pending_headers(block_number) {
				weight
			} else {
				// TODO: handle error

				0
			}
		}

		// TODO: too many db operations and calc need to move to `offchain_worker`
		// 	https://github.com/darwinia-network/darwinia-common/issues/254
		// TODO: close the game that its id less than the best number
		// 	https://github.com/darwinia-network/darwinia-common/issues/253
		fn on_finalize(block_number: BlockNumber<T>) {
			let closed_rounds = <ClosedRounds<T, I>>::take(block_number);

			// `closed_rounds` MUST NOT be empty after this check; qed
			if closed_rounds.len() != 0 {
				// TODO: handle error
				for closed_round in closed_rounds {
					closed_round.set_state(RoundState::Closed);
				}
			}
		}

	}
}

impl<T: Trait<I>, I: Instance> Module<T, I> {
	pub fn ensure_can_bond(
		relayer: &AccountId<T>,
		proposal: &[TcHeaderThing<T, I>],
		extend_at: RoundNumber,
		other_proposals_len: usize,
	) -> Result<
		(
			RingBalance<T, I>,
			Vec<(RingBalance<T, I>, TcHeaderThing<T, I>)>,
		),
		DispatchError,
	> {
		let (bond, bonded_proposal) = extend_proposal(
			proposal,
			extend_at,
			other_proposals_len,
			T::RelayerGameAdjustor::estimate_bond,
		);

		ensure!(
			T::RingCurrency::usable_balance(&relayer) >= bond,
			<Error<T, I>>::InsufficientBond
		);

		Ok((bond, bonded_proposal))
	}

	pub fn update_bonds_with<F>(relayer: &AccountId<T>, calc_bonds: F)
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

	pub fn settle(
		now: BlockNumber<T>,
		closed_rounds: Vec<(GameId<TcBlockNumber<T, I>>, Round)>,
	) -> DispatchResult {
		info!(target: "relayer-game", "Found Closed Rounds at `{:?}`", now);
		info!(target: "relayer-game", "---");

		let mut pending_headers = vec![];

		for (game_id, last_round) in closed_rounds {
			info!(target: "relayer-game", ">  Trying to Settle Game `{:?}` at Round `{}`", game_id, last_round);

			let mut proposals = Self::proposals_of_game(game_id);

			match proposals.len() {
				0 => info!(target: "relayer-game", "   >  No Proposal Found"),
				1 => {
					info!(target: "relayer-game", "   >  No Challenge Found");

					let confirmed_proposal = proposals.pop().unwrap();

					Self::settle_without_challenge(&confirmed_proposal);

					// TODO: reward if no challenge

					pending_headers.push((
						game_id,
						Self::header_of_game_with_hash(
							game_id,
							confirmed_proposal.bonded_proposal[0].1.hash(),
						),
					));
				}
				_ => {
					// 从这个游戏的所有proposals中找出符合这轮长度的
					let last_round_proposals = proposals_filter_by_round(
						&mut proposals,
						last_round,
						T::RelayerGameAdjustor::round_of_samples_count,
					);

					match last_round_proposals.len() {
						0 => {
							info!(target: "relayer-game", "   >  All Relayers Abstain");

							// `last_round` MUST NOT be `0`; qed
							Self::settle_abandon(proposals_filter_by_round(
								&mut proposals,
								last_round - 1,
								T::RelayerGameAdjustor::round_of_samples_count,
							));
						}
						1 => {
							let confirmed_proposal = {
								let mut last_round_proposals = last_round_proposals;

								last_round_proposals.pop().unwrap()
							};

							Self::settle_with_challenge(
								last_round,
								proposals,
								&confirmed_proposal,
								vec![],
							);

							// TODO: reward if no challenge

							pending_headers.push((
								game_id,
								Self::header_of_game_with_hash(
									game_id,
									confirmed_proposal.bonded_proposal[0].1.hash(),
								),
							));
						}
						_ => {
							let last_round_proposals_chain_len =
								last_round_proposals[0].bonded_proposal.len();
							let full_chain_len = (game_id - Self::last_confirmed_of_game(game_id))
								.saturated_into() as u64;

							// 跑完了
							if last_round_proposals_chain_len as u64 == full_chain_len {
								info!(target: "relayer-game", "   >  On Chain Arbitrate");

								if let Some(hash) = Self::on_chain_arbitrate(
									last_round,
									proposals,
									last_round_proposals,
								) {
									pending_headers.push((
										game_id,
										Self::header_of_game_with_hash(game_id, hash),
									));
								}
							} else { // 还没有跑完，更新samples等信息，进入下一轮
								info!(target: "relayer-game", "   >  Update Samples");

								Self::update_samples(game_id);

								let round = last_round + 1;
								let closed_at = now + T::RelayerGameAdjustor::challenge_time(round);

								<ClosedRounds<T, I>>::append(closed_at, (game_id, round));

								continue;
							}
						}
					}
				}
			}

			Self::game_over(game_id);
		}

		Self::store_pending_headers(now, pending_headers)?;

		info!(target: "relayer-game", "---");

		Ok(())
	}

	pub fn settle_without_challenge(confirmed_proposal: &RelayProposalT<T, I>) {
		let bond = confirmed_proposal.bonded_proposal[0].0;

		Self::update_bonds_with(&confirmed_proposal.relayer, |bonds| {
			bonds.saturating_sub(bond)
		});
	}

	pub fn settle_with_challenge(
		round: Round,
		proposals: Vec<RelayProposalT<T, I>>,
		confirmed_proposal: &RelayProposalT<T, I>,
		rewards: Vec<(
			(AccountId<T>, RingBalance<T, I>),
			(AccountId<T>, RingBalance<T, I>),
		)>,
	) {
		let extend_from_header_hash = confirmed_proposal
			.extend_from_header_hash
			.as_ref()
			.unwrap()
			.to_owned();
		let (honesties_map, evils_map, missing) = build_reward_map(
			round,
			proposals,
			extend_from_header_hash,
			rewards,
			T::RelayerGameAdjustor::round_of_samples_count,
		);

		for (honesty, honesty_bonds) in honesties_map {
			Self::update_bonds_with(&honesty, |old_bonds| {
				old_bonds.saturating_sub(honesty_bonds)
			});
		}

		for (evil, (evil_bonds, honesties_map)) in evils_map {
			Self::update_bonds_with(&evil, |old_bonds| old_bonds.saturating_sub(evil_bonds));

			if honesties_map.is_empty() {
				Self::update_bonds_with(&evil, |old_bonds| old_bonds.saturating_sub(evil_bonds));

				let (imbalance, _) = T::RingCurrency::slash(&evil, evil_bonds);
				T::RingSlash::on_unbalanced(imbalance);
			} else {
				for (honesty, evil_bonds) in honesties_map {
					let _ = T::RingCurrency::transfer(
						&evil,
						&honesty,
						evil_bonds,
						ExistenceRequirement::KeepAlive,
					);
				}
			}
		}

		for evils in missing {
			for (evil, evil_bonds) in evils {
				Self::update_bonds_with(&evil, |old_bonds| old_bonds.saturating_sub(evil_bonds));

				let (imbalance, _) = T::RingCurrency::slash(&evil, evil_bonds);

				T::RingSlash::on_unbalanced(imbalance);
			}
		}
	}

	pub fn settle_abandon(proposals: Vec<RelayProposalT<T, I>>) {
		for proposal in proposals {
			let bond = proposal
				.bonded_proposal
				.iter()
				.fold(Zero::zero(), |proposal_bond, (round_bond, _)| {
					proposal_bond + *round_bond
				});

			Self::update_bonds_with(&proposal.relayer, |old_bonds| {
				old_bonds.saturating_sub(bond)
			});

			let (imbalance, _) = T::RingCurrency::slash(&proposal.relayer, bond);

			T::RingSlash::on_unbalanced(imbalance);
		}
	}

	pub fn on_chain_arbitrate(
		last_round: Round,
		proposals: Vec<RelayProposalT<T, I>>,
		last_round_proposals: Vec<RelayProposalT<T, I>>,
	) -> Option<TcHeaderHash<T, I>> {
		let mut maybe_confirmed_proposal: Option<RelayProposalT<T, I>> = None;
		let mut evils = vec![];

		for proposal in last_round_proposals.iter() {
			if T::TargetChain::on_chain_arbitrate(
				proposal
					.bonded_proposal
					.iter()
					.map(|(_, header_thing)| header_thing.clone())
					.collect(),
			)
			.is_ok()
			{
				if maybe_confirmed_proposal.is_none() {
					maybe_confirmed_proposal = Some(proposal.to_owned());
				} else {
					error!("Honest Relayer Count - MORE THAN 1 WITHIN A ROUND");
				}
			} else {
				evils.push((
					proposal.relayer.clone(),
					proposal.bonded_proposal.last().unwrap().0,
				));
			}
		}

		if let Some(confirmed_proposal) = maybe_confirmed_proposal {
			let rewards = evils
				.into_iter()
				.map(|evil| {
					(
						(
							confirmed_proposal.relayer.clone(),
							confirmed_proposal.bonded_proposal.last().unwrap().0,
						),
						evil,
					)
				})
				.collect();

			Self::settle_with_challenge(last_round, proposals, &confirmed_proposal, rewards);

			// TODO: reward if no challenge

			Some(confirmed_proposal.bonded_proposal[0].1.hash())
		} else {
			info!(target: "relayer-game", "   >  No Honest Relayer");

			Self::settle_abandon(last_round_proposals);

			None
		}
	}

	pub fn update_samples(game_id: GameId<TcBlockNumber<T, I>>) {
		<Samples<T, I>>::mutate(game_id, |samples| {
			T::RelayerGameAdjustor::update_samples(samples);

			if samples.len() < 2 {
				error!("Sample Points MISSING, Check Your Sample Strategy Implementation");

				return;
			}

			Self::deposit_event(RawEvent::NewRound(
				game_id,
				samples.concat(),
				samples[samples.len() - 1].clone(),
			));
		});
	}

	pub fn game_over(game_id: GameId<TcBlockNumber<T, I>>) {
		<Samples<T, I>>::take(game_id);
		<LastConfirmeds<T, I>>::take(game_id);
		<Headers<T, I>>::remove_prefix(game_id);
		<Proposals<T, I>>::take(game_id);

		Self::deposit_event(RawEvent::GameOver(game_id));
	}

	pub fn store_pending_headers(
		now: BlockNumber<T>,
		pending_headers: Vec<(TcBlockNumber<T, I>, TcHeaderThing<T, I>)>,
	) -> DispatchResult {
		let confirm_period = T::ConfirmPeriod::get();

		if confirm_period.is_zero() {
			for (_, pending_header) in pending_headers {
				T::TargetChain::store_header(pending_header)?;
			}
		} else {
			for (pending_block_number, pending_header) in pending_headers {
				<PendingHeaders<T, I>>::append((
					now + confirm_period,
					pending_block_number,
					pending_header,
				));
			}
		}

		Ok(())
	}

	pub fn update_pending_headers_with<F>(
		pending_block_number: TcBlockNumber<T, I>,
		f: F,
	) -> DispatchResult
	where
		F: FnOnce(TcHeaderThing<T, I>) -> DispatchResult,
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

	pub fn system_approve_pending_headers(now: BlockNumber<T>) -> Result<Weight, DispatchError> {
		<PendingHeaders<T, I>>::mutate(|pending_headers| {
			pending_headers.retain(|(confirm_at, pending_block_number, pending_header)| {
				if *confirm_at == now {
					// TODO: handle error
					let _ = T::TargetChain::store_header(pending_header.to_owned());

					Self::deposit_event(RawEvent::PendingHeaderApproved(
						*pending_block_number,
						b"Not Enough Technical Member Online, Approved By System".to_vec(),
					));

					false
				} else {
					true
				}
			})
		});

		Ok(0)
	}
}

impl<T: Trait<I>, I: Instance> RelayerGameProtocol for Module<T, I> {
	type Relayer = AccountId<T>;
	type Balance = RingBalance<T, I>;
	type HeaderThingWithProof = TcHeaderThingWithProof<T, I>;
	type HeaderThing = TcHeaderThing<T, I>;

	fn proposals_of_game(
		game_id: <Self::HeaderThing as HeaderThing>::Number,
	) -> Vec<
		RelayProposal<
			Self::Relayer,
			Self::Balance,
			Self::HeaderThing,
			<Self::HeaderThing as HeaderThing>::Hash,
		>,
	> {
		Self::proposals_of_game(game_id)
	}

	fn submit_extend_proposal(
		relayer: Self::Relayer,
		proposal: Vec<Self::HeaderThingWithProof>,
	) -> DispatchResult {
		let verified_proposal = T::TargetChain::basic_verify(proposal)?;
		let proposed_header = verified_proposal
			.get(0)
			.ok_or(<Error<T, I>>::ProposalI)?
			.to_owned();
		let game_id = proposed_header.number();

		match Self::game_by_id(game_id) {
			None => Ok(()), // TODO: add error
			Some(game) => {
				let round_number =
					T::RelayerGameAdjustor::round_of_samples_count(verified_proposal.len() as _);
				let prev_round_number = round_number.checked_sub(1).ok_or(<Error<T, I>>::RoundMis)?;

				let round = game.rounds[round_number];
				let previous_round = game.rounds[prev_round_number];

				ensure!(
					verified_proposal.len() == round.samples.len(),
					<Error<T, I>>::RoundMis
				);
				ensure!(
					verified_proposal
						.iter()
						.zip(round.samples.iter())
						.all(|(header_thing, sample_block_number)| header_thing.number()
							== *sample_block_number),
					<Error<T, I>>::RoundMis
				);

				let extend_at = previous_round.samples.len();
				let (bond, extend_proposal) = Self::ensure_can_bond(
					&relayer,
					&verified_proposal[extend_at..],
					prev_round_number,
					other_proposals_len,
				)?;

				let extend_from_proposal = previous_round.proposals.last();

				if let Some(RelayProposal {
								bonded_proposal: extend_from_proposal,
								..
							}) = extend_from_proposal
				{
					let extend_from_header = extend_from_proposal.last().unwrap().1.clone();
					let bonded_proposal = [extend_from_proposal, extend_proposal].concat();

					Self::update_bonds_with(&relayer, |bonds| bonds.saturating_add(bond));

					round.add_proposal(RelayProposal {
						relayer,
						bonded_proposal,
						// Each proposal MUST contains a NOT empty chain; qed
						extend_from_header_hash: Some(extend_from_header.hash()),
					});

					<Game<T, I>>::insert(game_id, game);
				} else {
					Err(<Error<T, I>>::RoundMis)?;
				}
			}
		}
	}

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
		proposal: Vec<Self::HeaderThingWithProof>,
	) -> DispatchResult {
		info!(
			target: "relayer-game",
			"Relayer `{:?}` Submit a Proposal:\n{:#?}",
			relayer,
			proposal
		);

		let verified_proposal = T::TargetChain::basic_verify(proposal)?;
		let proposed_header = verified_proposal
			.get(0)
			.ok_or(<Error<T, I>>::ProposalI)?
			.to_owned();
		let game_id = proposed_header.number();
		let proposed_header_hash = proposed_header.hash();

		let other_proposals = Self::proposals_of_game(game_id);
		let other_proposals_len = other_proposals.len();

		match Self::game_by_id(game_id) {
			// New Game
			None => {
				ensure!(verified_proposal.len() == 1, <Error<T, I>>::RoundMis);

				let best_block_number = T::TargetChain::best_block_number();

				ensure!(game_id > best_block_number, <Error<T, I>>::TargetHeaderAC);

				let (bond, bonded_proposal) =
					Self::ensure_can_bond(&relayer, &verified_proposal, 0, 0)?;

				Self::update_bonds_with(&relayer, |bonds| bonds.saturating_add(bond));

				<Headers<T, I>>::insert(game_id, proposed_header_hash, proposed_header);

				let game = Game::new(
					game_id,
					vec![game_id],
					vec![
						RelayProposal {
							relayer,
							bonded_proposal,
							extend_from_header_hash: None,
						},
					],
					best_block_number
				);
				<ClosedRounds<T, I>>::append(
					<frame_system::Module<T>>::block_number()
						+ T::RelayerGameAdjustor::challenge_time(0),
					&game.current_round,
				);
				<Game<T, I>>::insert(game_id, game);
			},
			// Challenge
			Some(game) => {
				let round = game.rounds[0];
				let (bond, bonded_proposal) =
					Self::ensure_can_bond(&relayer, &verified_proposal, 0, round.proposals.len())?;

				Self::update_bonds_with(&relayer, |bonds| bonds.saturating_add(bond));

				<Headers<T, I>>::insert(game.id, proposed_header_hash, proposed_header);

				round.add_proposal(RelayProposal {
					relayer,
					bonded_proposal,
					extend_from_header_hash: None,
				});
				<Game<T, I>>::insert(game_id, game);
			}
		}

		Ok(())
	}

	fn approve_pending_header(
		pending: <Self::HeaderThing as HeaderThing>::Number,
	) -> DispatchResult {
		Self::update_pending_headers_with(pending, |header| T::TargetChain::store_header(header))?;
		Self::deposit_event(RawEvent::PendingHeaderApproved(
			pending,
			b"Approved By Council".to_vec(),
		));

		Ok(())
	}

	fn reject_pending_header(
		pending: <Self::HeaderThing as HeaderThing>::Number,
	) -> DispatchResult {
		Self::update_pending_headers_with(pending, |_| Ok(()))?;
		Self::deposit_event(RawEvent::PendingHeaderRejected(pending));

		Ok(())
	}
}

// TODO: https://github.com/darwinia-network/darwinia-common/issues/209
pub trait WeightInfo {}
impl WeightInfo for () {}
