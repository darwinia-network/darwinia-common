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

	pub type RelayBlockId<T, I> = <RelayableChainT<T, I> as RelayableChain>::RelayBlockId;
	pub type GameId<T, I> = RelayBlockId<T, I>;
	pub type RelayStuffs<T, I> = <RelayableChainT<T, I> as RelayableChain>::RelayStuffs;
	pub type RelayProofs<T, I> = <RelayableChainT<T, I> as RelayableChain>::Proofs;

	pub type RelayProposalT<T, I> =
		RelayProposal<RelayStuffs<T, I>, AccountId<T>, RingBalance<T, I>, GameId<T, I>>;

	type RingCurrency<T, I> = <T as Trait<I>>::RingCurrency;

	type RelayableChainT<T, I> = <T as Trait<I>>::RelayableChain;
}

// --- substrate ---
use frame_support::{
	debug::*,
	decl_error, decl_event, decl_module, decl_storage, ensure,
	traits::{Currency, Get, OnUnbalanced},
};
use sp_runtime::{DispatchError, DispatchResult};
#[cfg(not(feature = "std"))]
use sp_std::borrow::ToOwned;
use sp_std::prelude::*;
// --- darwinia ---
use darwinia_relay_primitives::*;
use darwinia_support::balance::lock::*;
use types::*;

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
		RelayBlockId = RelayBlockId<Self, I>,
	>;

	/// A chain which implemented `RelayableChain` trait
	type RelayableChain: RelayableChain;

	/// The comfirm period for guard
	///
	/// Tech.Comm. can vote for the pending header within this period
	/// If not enough Tech.Comm. votes for the pending header it will be confirmed
	/// automatically after this period
	type ConfirmPeriod: Get<Self::BlockNumber>;

	/// Weight information for extrinsics in this pallet.
	type WeightInfo: WeightInfo;
}

// TODO: https://github.com/darwinia-network/darwinia-common/issues/209
pub trait WeightInfo {}
impl WeightInfo for () {}

decl_event! {
	pub enum Event<T, I: Instance = DefaultInstance>
	where
		GameId = GameId<T, I>,
	{
		TODO(GameId),
	}
}

decl_error! {
	pub enum Error for Module<T: Trait<I>, I: Instance> {
		/// Relay Stuffs - ALREADY CONFIRMED
		RelayStuffsAC,
		/// Round - MISMATCHED
		RoundMis,
		/// Active Games - TOO MANY
		ActiveGamesTM,
		/// Game - CLOSED
		GameC,
		/// Proposal - DUPLICATED
		ProposalDup,
		/// Usable *RING* for Bond - INSUFFICIENT
		BondIns,
		/// Proofses - NOT ENOUGH
		ProofsesNE,
		/// Proposal - NOT EXISTED
		ProposalNE,
		/// Extended Proposal - NOT EXISTED
		ExtendedProposalNE,
		/// Previous Proofs - INCOMPLETE
		PreviousProofsInc,
	}
}

decl_storage! {
	trait Store for Module<T: Trait<I>, I: Instance = DefaultInstance> as DarwiniaRelayerGame {
		/// The number of active games
		pub ActiveGames get(fn active_games): u8;

		/// All the active games' proposals here
		///
		/// The first key is game id, the second key is round index
		/// then you will get the proposals under that round in that game
		pub Proposals
			get(fn proposals_of_game_at)
			: double_map
				hasher(identity) GameId<T, I>,
				hasher(identity) u32
			=> Vec<RelayProposalT<T, I>>;

		/// All the closed games here
		///
		/// Closed games won't accept any proposal
		pub ClosedGames
			get(fn game_closed_at_of)
			: map hasher(identity) GameId<T, I>
			=> BlockNumber<T>;

		/// All the closed rounds here
		///
		/// Record the closed rounds endpoint which use for settlling or updating
		pub ClosedRounds
			get(fn closed_rounds_at)
			: map hasher(identity) BlockNumber<T>
			=> Vec<GameId<T, I>>;
	}
}

decl_module! {
	pub struct Module<T: Trait<I>, I: Instance = DefaultInstance> for enum Call
	where
		origin: T::Origin
	{
		type Error = Error<T, I>;

		fn deposit_event() = default;

		fn offchain_worker(now: BlockNumber<T>) {
			let closed_rounds = Self::closed_rounds_at(now);

			if !closed_rounds.is_empty() {
				Self::update_games_at(closed_rounds, now);
			}
		}
	}
}

impl<T: Trait<I>, I: Instance> Module<T, I> {
	/// Check if time for proposing
	pub fn is_game_open_at(game_id: &GameId<T, I>, moment: BlockNumber<T>) -> bool {
		Self::game_closed_at_of(game_id) > moment
	}

	/// Check if others already make a same proposal
	pub fn is_unique_proposal(
		proposal_content: &[RelayStuffs<T, I>],
		existed_proposals: &[RelayProposalT<T, I>],
	) -> bool {
		!existed_proposals
			.iter()
			.any(|existed_proposal| existed_proposal.content.as_slice() == proposal_content)
	}

	/// Check if relayer can afford the bond
	///
	/// Make sure relayer have enough balance,
	/// this won't let the account's free balance drop below existential deposit
	pub fn ensure_can_bond(
		relayer: &AccountId<T>,
		round: u32,
		proposals_count: u8,
	) -> Result<RingBalance<T, I>, DispatchError> {
		let bond = T::RelayerGameAdjustor::estimate_bond(round, proposals_count);

		ensure!(
			T::RingCurrency::usable_balance(relayer) >= bond,
			<Error<T, I>>::BondIns
		);

		Ok(bond)
	}

	// /// Build a proposal
	// ///
	// /// Allow propose without proofs
	// /// The proofs can be completed later through `complete_proofs`
	// pub fn build_proposal(
	// 	relayer: AccountId<T>,
	// 	relay_stuffses: Vec<RelayStuffs<T, I>>,
	// 	proofses: Option<Vec<RelayProofs<T, I>>>,
	// 	bond: RingBalance<T, I>,
	// ) -> Result<RelayProposalT<T, I>, DispatchError> {
	// 	let mut proposal = RelayProposal::new();

	// 	if let Some(proofses) = proofses {
	// 		for (relay_stuffs, proofs) in relay_stuffses.iter().zip(proofses.into_iter()) {
	// 			T::RelayableChain::verify_proofs(relay_stuffs, &proofs)?;
	// 		}

	// 		proposal.verified = true;
	// 	}

	// 	proposal.relayer = relayer;
	// 	proposal.content = relay_stuffses;
	// 	proposal.bond = bond;

	// 	Ok(proposal)
	// }

	pub fn update_timer_of_game_at(game_id: &GameId<T, I>, round: u32) {
		let propose_time = T::RelayerGameAdjustor::propose_time(round);
		let complete_proofs_time = T::RelayerGameAdjustor::complete_proofs_time(round);

		<ClosedGames<T, I>>::insert(game_id, propose_time);
		<ClosedRounds<T, I>>::append(propose_time + complete_proofs_time, game_id);
	}

	pub fn update_games_at(game_ids: Vec<GameId<T, I>>, moment: BlockNumber<T>) {
		info!(target: "relayer-game", "Found Closed Rounds at `{:?}`", moment);
		info!(target: "relayer-game", "---");

		for game_id in game_ids {}
	}
}

impl<T: Trait<I>, I: Instance> RelayerGameProtocol for Module<T, I> {
	type Relayer = AccountId<T>;
	type GameId = GameId<T, I>;
	type RelayStuffs = RelayStuffs<T, I>;
	type Proofs = RelayProofs<T, I>;

	fn propose(
		relayer: Self::Relayer,
		game_id: Self::GameId,
		relay_stuffs: Self::RelayStuffs,
		proofs: Option<Self::Proofs>,
	) -> DispatchResult {
		trace!(
			target: "relayer-game",
			"Relayer `{:?}` propose:\n{:#?}",
			relayer,
			relay_stuffs
		);

		let active_games = Self::active_games();

		// Check if the proposed header has already been relaied
		ensure!(
			game_id > T::RelayableChain::best_block_id(),
			<Error<T, I>>::RelayStuffsAC
		);
		// Make sure the game is at first round
		ensure!(
			Self::proposals_of_game_at(&game_id, 1).is_empty(),
			<Error<T, I>>::RoundMis
		);

		let existed_proposals = Self::proposals_of_game_at(&game_id, 0);
		let proposal_content = vec![relay_stuffs];

		if existed_proposals.is_empty() {
			// A new game might open

			// Check if it is ok to open more games
			ensure!(
				active_games < T::RelayerGameAdjustor::max_active_games(),
				<Error<T, I>>::ActiveGamesTM
			);
		} else {
			// An against proposal might add

			ensure!(
				Self::is_game_open_at(&game_id, <frame_system::Module<T>>::block_number()),
				<Error<T, I>>::GameC
			);
			ensure!(
				Self::is_unique_proposal(&proposal_content, &existed_proposals),
				<Error<T, I>>::ProposalDup
			);
		}

		let bond = Self::ensure_can_bond(&relayer, 0, existed_proposals.len() as u8 + 1)?;
		// let proposal = Self::build_proposal(relayer, samples, proofses, bond)?;
		let proposal = {
			let mut proposal = RelayProposal::new();

			// Allow propose without proofs
			// The proofs can be completed later through `complete_proofs`
			if let Some(proofs) = proofs {
				T::RelayableChain::verify_proofs(&proposal_content[0], &proofs)?;

				proposal.verified = true;
			}

			proposal.relayer = relayer;
			proposal.content = proposal_content;
			proposal.bond = bond;

			proposal
		};

		<ActiveGames<I>>::mutate(|count| *count += 1);
		<Proposals<T, I>>::append(&game_id, 0, proposal);

		Self::update_timer_of_game_at(&game_id, 0);

		Ok(())
	}

	fn complete_proofs(
		proposal_id: ProposalId<Self::GameId>,
		proofses: Vec<Self::Proofs>,
	) -> DispatchResult {
		let (game_id, round, round_index) = proposal_id;

		<Proposals<T, I>>::try_mutate(&game_id, round, |proposals| {
			if let Some(proposal) = proposals.get_mut(round_index as usize) {
				for (relay_stuffs, proofs) in proposal.content.iter().zip(proofses.into_iter()) {
					T::RelayableChain::verify_proofs(relay_stuffs, &proofs)?;
				}

				proposal.verified = true;

				Ok(())
			} else {
				Err(<Error<T, I>>::ProposalNE)?
			}
		})
	}

	fn extend_proposal(
		relayer: Self::Relayer,
		samples: Vec<Self::RelayStuffs>,
		extended_proposal_id: ProposalId<Self::GameId>,
		proofses: Option<Vec<Self::Proofs>>,
	) -> DispatchResult {
		let (game_id, previous_round, previous_round_index) = &extended_proposal_id;

		ensure!(
			Self::is_game_open_at(game_id, <frame_system::Module<T>>::block_number()),
			<Error<T, I>>::GameC
		);

		if let Some(ref proofses) = &proofses {
			ensure!(proofses.len() == samples.len(), <Error<T, I>>::ProofsesNE);
		}

		let round = *previous_round + 1;
		let existed_proposals = Self::proposals_of_game_at(&game_id, round);

		ensure!(
			Self::is_unique_proposal(&samples, &existed_proposals),
			<Error<T, I>>::ProposalDup
		);

		let extended_proposal = existed_proposals
			.get(*previous_round_index as usize)
			.ok_or(<Error<T, I>>::ExtendedProposalNE)?;

		// Currently only accept extending from a completed proposal
		ensure!(extended_proposal.verified, <Error<T, I>>::PreviousProofsInc);

		T::RelayableChain::verify_continuous(&samples, &extended_proposal.content)?;

		let bond = Self::ensure_can_bond(&relayer, round, existed_proposals.len() as u8 + 1)?;
		// let proposal = Self::build_proposal(relayer, samples, proofses, bond)?;
		let proposal = {
			let mut proposal = RelayProposal::new();

			// Allow propose without proofs
			// The proofs can be completed later through `complete_proofs`
			if let Some(proofses) = proofses {
				for (sample, proofs) in samples.iter().zip(proofses.into_iter()) {
					T::RelayableChain::verify_proofs(sample, &proofs)?;
				}

				proposal.verified = true;
			}

			proposal.relayer = relayer;
			proposal.content = samples;
			proposal.bond = bond;

			proposal
		};

		<Proposals<T, I>>::append(&game_id, round, proposal);

		Ok(())
	}
}
