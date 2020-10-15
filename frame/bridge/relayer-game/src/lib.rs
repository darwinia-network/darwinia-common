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
	// pub type BlockNumber<T> = <T as frame_system::Trait>::BlockNumber;

	pub type RingBalance<T, I> = <RingCurrency<T, I> as Currency<AccountId<T>>>::Balance;
	pub type RingNegativeImbalance<T, I> =
		<RingCurrency<T, I> as Currency<AccountId<T>>>::NegativeImbalance;

	pub type BlockId<T, I> = <Tc<T, I> as Relayable>::BlockId;
	pub type GameId<T, I> = BlockId<T, I>;
	pub type RelayStuffs<T, I> = <Tc<T, I> as Relayable>::RelayStuffs;
	pub type RelayProofs<T, I> = <Tc<T, I> as Relayable>::Proofs;

	pub type RelayProposalT<T, I> =
		RelayProposal<RelayStuffs<T, I>, AccountId<T>, RingBalance<T, I>>;

	type RingCurrency<T, I> = <T as Trait<I>>::RingCurrency;

	type Tc<T, I> = <T as Trait<I>>::TargetChain;
}

// --- substrate ---
use frame_support::{
	debug::info,
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
		BlockId = BlockId<Self, I>,
	>;

	/// The target chain's relay module's API
	type TargetChain: Relayable;

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
		/// Active Games - TOO MANY
		ActiveGamesTM,
		/// Target Header - ALREADY CONFIRMED
		TargetHeaderAC,
		/// Round - MISMATCHED
		RoundMis,
		/// Proposal - DUPLICATED
		ProposalDup,
		/// Usable *RING* for Bond - INSUFFICIENT
		BondI,
	}
}

decl_storage! {
	trait Store for Module<T: Trait<I>, I: Instance = DefaultInstance> as DarwiniaRelayerGame {
		/// The number of active games
		pub ActiveGames get(fn active_games): u8;

		pub Proposals
			get(fn proposals_of_game_at_round)
			: double_map
				hasher(identity) GameId<T, I>,
				hasher(identity) Round
			=> Vec<RelayProposalT<T, I>>;
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
		info!(
			target: "relayer-game",
			"Relayer `{:?}` propose:\n{:#?}",
			relayer,
			relay_stuffs
		);

		let active_games = Self::active_games();

		// Check if the chain can open more games
		ensure!(
			active_games < T::RelayerGameAdjustor::max_active_games(),
			<Error<T, I>>::ActiveGamesTM
		);
		// Check if the proposed header has already been relaied
		ensure!(
			game_id > T::TargetChain::best_block_id(),
			<Error<T, I>>::TargetHeaderAC
		);
		// Make sure this is a new game
		ensure!(
			Self::proposals_of_game_at_round(game_id.clone(), 1).is_empty(),
			<Error<T, I>>::RoundMis
		);

		let round = 0;
		let other_proposals = Self::proposals_of_game_at_round(game_id.clone(), round);

		// Check if others already make a same proposal
		ensure!(
			!other_proposals
				.iter()
				.any(|proposal| &proposal.proposed[0] == &relay_stuffs),
			<Error<T, I>>::ProposalDup
		);

		let bond = T::RelayerGameAdjustor::estimate_bond(round, other_proposals.len() as u8 + 1);

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
				T::TargetChain::verify_proofs(&relay_stuffs, &proofs)?;

				proposal.verified = true;
			}

			proposal.proposed = vec![relay_stuffs];
			proposal.bonds = vec![(relayer, bond)];

			proposal
		};

		<Proposals<T, I>>::insert(game_id, round, vec![proposal]);
		<ActiveGames<I>>::mutate(|count| *count += 1);

		Ok(())
	}
}
