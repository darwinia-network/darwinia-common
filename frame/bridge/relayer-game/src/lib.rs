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
use sp_runtime::DispatchResult;
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
		BondI,
		/// Proposal - NOT FOUND
		ProposalNF,
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
			get(fn proposals_of_game_at_round)
			: double_map
				hasher(identity) GameId<T, I>,
				hasher(identity) u32
			=> Vec<RelayProposalT<T, I>>;

		/// All the games' status here
		///
		/// Use this to manage the challenge time
		pub GameStatuses
			get(fn game_status)
			: map hasher(identity) GameId<T, I>
			=> GameStatus<BlockNumber<T>>;
	}
}

decl_module! {
	pub struct Module<T: Trait<I>, I: Instance = DefaultInstance> for enum Call
	where
		origin: T::Origin
	{
		type Error = Error<T, I>;

		fn deposit_event() = default;
	}
}

impl<T: Trait<I>, I: Instance> Module<T, I> {
	/// Check if is time for proposing
	pub fn is_open_game(game_id: &GameId<T, I>) -> bool {
		matches!(Self::game_status(&game_id), GameStatus::Open(_))
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
			Self::proposals_of_game_at_round(&game_id, 1).is_empty(),
			<Error<T, I>>::RoundMis
		);

		let existed_proposals = Self::proposals_of_game_at_round(&game_id, 0);
		let proposal_content = vec![relay_stuffs];

		if existed_proposals.is_empty() {
			// A new game might open

			// Check is ok to open more games
			ensure!(
				active_games < T::RelayerGameAdjustor::max_active_games(),
				<Error<T, I>>::ActiveGamesTM
			);
		} else {
			// An against proposal might add

			ensure!(Self::is_open_game(&game_id), <Error<T, I>>::GameC);
			ensure!(
				Self::is_unique_proposal(&proposal_content, &existed_proposals),
				<Error<T, I>>::ProposalDup
			);
		}

		let bond = T::RelayerGameAdjustor::estimate_bond(0, existed_proposals.len() as u8 + 1);

		// Make sure relayer have enough balance,
		// this won't let the account's free balance drop below existential deposit
		ensure!(
			T::RingCurrency::usable_balance(&relayer) >= bond,
			<Error<T, I>>::BondI
		);

		let proposal = {
			let mut proposal = RelayProposal::new();

			// Allow propose without proofs
			// The proofs can be completed later through `complete_proofs`
			if let Some(proofs) = proofs {
				T::RelayableChain::verify_proofs(&proposal_content[0], &proofs)?;

				proposal.verified = true;
			}

			proposal.content = proposal_content;
			proposal.bonds = vec![(relayer, bond)];

			proposal
		};

		<Proposals<T, I>>::append(&game_id, 0, proposal);
		<GameStatuses<T, I>>::insert(
			&game_id,
			GameStatus::Open(T::RelayerGameAdjustor::propose_time(0)),
		);
		<ActiveGames<I>>::mutate(|count| *count += 1);

		Ok(())
	}

	fn complete_proofs(
		proposal_id: ProposalId<Self::GameId>,
		proofs: Vec<Self::Proofs>,
	) -> DispatchResult {
		let (game_id, round, round_index) = proposal_id;

		<Proposals<T, I>>::try_mutate(&game_id, round, |proposals| {
			if let Some(proposal) = proposals.get_mut(round_index as usize) {
				for (relay_stuffs, proofs) in proposal.content.iter().zip(proofs.into_iter()) {
					T::RelayableChain::verify_proofs(relay_stuffs, &proofs)?;
				}

				proposal.verified = true;

				Ok(())
			} else {
				Err(<Error<T, I>>::ProposalNF)?
			}
		})
	}

	fn extend_proposal(
		samples: Vec<Self::RelayStuffs>,
		extended_proposal_id: ProposalId<Self::GameId>,
		proofs: Option<Vec<Self::Proofs>>,
	) -> DispatchResult {
		let (game_id, previous_round, previous_round_index) = &extended_proposal_id;
		let round = *previous_round + 1;
		let existed_proposals = Self::proposals_of_game_at_round(&game_id, round);

		ensure!(Self::is_open_game(game_id), <Error<T, I>>::GameC);

		Ok(())
	}
}
