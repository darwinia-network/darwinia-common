//! Relayer Game Primitives

#![cfg_attr(not(feature = "std"), no_std)]
#![feature(drain_filter)]

// --- core ---
use core::fmt::Debug;
// --- crates ---
use codec::{Decode, Encode, FullCodec};
// --- substrate ---
use sp_runtime::{traits::Zero, DispatchResult, RuntimeDebug};
#[cfg(not(feature = "std"))]
use sp_std::borrow::ToOwned;
use sp_std::prelude::*;

/// Game id, round and the index under the round point to a unique proposal AKA proposal id
pub type ProposalId<GameId> = (GameId, u32, u32);

/// Implement this for target chain's relay module's
/// to expose some necessary APIs for relayer game
pub trait RelayableChain {
	type RelayBlockId: Clone + Debug + Default + PartialOrd + FullCodec;
	type Parcel: Clone + Debug + PartialEq + PartialOrd + FullCodec;
	type Proofs;

	/// The latest finalize block's id which recorded in darwinia
	fn best_block_id() -> Self::RelayBlockId;

	fn verify_proofs(parcel: &Self::Parcel, proofs: &Self::Proofs) -> DispatchResult;

	fn verify_continuous(samples: &[Self::Parcel], extended: &[Self::Parcel]) -> DispatchResult;

	fn distance_between(
		game_id: &Self::RelayBlockId,
		last_confirmed_block_id_of: Self::RelayBlockId,
	) -> u32;
}

/// A regulator to adjust relay args for a specific chain
/// Implement this in runtime's `impls.rs`
pub trait AdjustableRelayerGame {
	type Moment;
	type Balance;
	type RelayBlockId;

	/// The maximum number of active games
	///
	/// This might relate to the validators count
	fn max_active_games() -> u8;

	fn propose_time(round: u32) -> Self::Moment;

	fn complete_proofs_time(round: u32) -> Self::Moment;

	fn update_samples(samples: &mut Vec<Vec<Self::RelayBlockId>>);

	/// Give an estimate bond value for a specify round
	///
	/// Usally the bond value go expensive wihle the round and the proposals count increase
	fn estimate_bond(round: u32, proposals_count: u8) -> Self::Balance;
}

pub trait RelayerGameProtocol {
	type Relayer;
	type GameId: Clone + PartialOrd;
	type Parcel: Clone + Debug + PartialEq + PartialOrd + FullCodec;
	type Proofs;

	/// Game's entry point, call only at the first round
	///
	/// Propose a new proposal or against a existed proposal
	fn propose(
		relayer: Self::Relayer,
		game_id: Self::GameId,
		parcel: Self::Parcel,
		proofs: Option<Self::Proofs>,
	) -> DispatchResult;

	/// Verify a specify proposal
	///
	/// Proofs is a `Vec` because the sampling function might give more than 1 sample points,
	/// so need to verify each sample point with its proofs
	fn complete_proofs(
		proposal_id: ProposalId<Self::GameId>,
		proofs: Vec<Self::Proofs>,
	) -> DispatchResult;

	/// Once there're different opinions in a game,
	/// chain will ask relayer to submit more samples
	/// to help the chain make a on chain arbitrate finally
	fn extend_proposal(
		relayer: Self::Relayer,
		samples: Vec<Self::Parcel>,
		extended_proposal_id: ProposalId<Self::GameId>,
		proofses: Option<Vec<Self::Proofs>>,
	) -> DispatchResult;
}

#[derive(Clone, Encode, Decode, RuntimeDebug)]
pub struct RelayProposal<Parcel, Relayer, Balance, GameId> {
	pub relayer: Relayer,
	pub content: Vec<Parcel>,
	pub bond: Balance,
	pub maybe_extended_proposal_id: Option<ProposalId<GameId>>,
	pub verified: bool,
}
impl<Parcel, Relayer, Balance, GameId> RelayProposal<Parcel, Relayer, Balance, GameId>
where
	Relayer: Default,
	Balance: Zero,
{
	pub fn new() -> Self {
		Self {
			relayer: Relayer::default(),
			content: vec![],
			bond: Zero::zero(),
			maybe_extended_proposal_id: None,
			verified: false,
		}
	}
}

// #[derive(Encode, Decode, RuntimeDebug)]
// pub enum GameStatus<Moment> {
// 	/// Relayer can propose before `Moment`
// 	Open(Moment),
// 	/// First parameter means there are some different opinions in this game
// 	/// if true, the proofs can be ignored
// 	///
// 	/// Second parameter means relayer can complete proofs before this time
// 	Closed((bool, Moment)),
// }
// impl<Moment> Default for GameStatus<Moment>
// where
// 	Moment: Zero,
// {
// 	fn default() -> Self {
// 		Self::Closed((false, Zero::zero()))
// 	}
// }
