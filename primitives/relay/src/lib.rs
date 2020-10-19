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
pub type RelayProposalId<GameId> = (GameId, u32, u32);

pub trait BlockInfo {
	type BlockId: Clone;

	fn block_id(&self) -> &Self::BlockId;
}

/// Implement this for target chain's relay module's
/// to expose some necessary APIs for relayer game
pub trait RelayableChain {
	type RelayBlockId: Clone + Debug + Default + PartialOrd + FullCodec;
	type RelayParcel: Clone
		+ Debug
		+ PartialEq
		+ PartialOrd
		+ FullCodec
		+ BlockInfo<BlockId = Self::RelayBlockId>;
	type Proofs;

	/// The latest finalize block's id which recorded in darwinia
	fn best_block_id() -> Self::RelayBlockId;

	fn verify_proofs(relay_parcel: &Self::RelayParcel, proofs: &Self::Proofs) -> DispatchResult;

	fn verify_continuous(
		parcels: &[Self::RelayParcel],
		extended_parcels: &[Self::RelayParcel],
	) -> DispatchResult;

	fn distance_between(
		relay_block_id: &Self::RelayBlockId,
		last_confirmed_relay_block_id: Self::RelayBlockId,
	) -> u32;

	fn store_relay_parcel(relay_parcel: Self::RelayParcel) -> DispatchResult;
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

	/// Update the samples
	///
	/// Push the new samples to the `samples`, the index of `samples` aka round index
	/// And return the new samples
	fn update_samples(samples: &mut Vec<Vec<Self::RelayBlockId>>);

	/// Give an estimate bond value for a specify round
	///
	/// Usally the bond value go expensive wihle the round and the proposals count increase
	fn estimate_bond(round: u32, proposals_count: u8) -> Self::Balance;
}

pub trait RelayerGameProtocol {
	type Relayer;
	type GameId: Clone + PartialOrd;
	type RelayParcel: Clone + Debug + PartialEq + PartialOrd + FullCodec + BlockInfo;
	type Proofs;

	/// Game's entry point, call only at the first round
	///
	/// Propose a new proposal or against a existed proposal
	fn propose(
		relayer: Self::Relayer,
		relay_parcel: Self::RelayParcel,
		proofs: Option<Self::Proofs>,
	) -> DispatchResult;

	/// Verify a specify proposal
	///
	/// Proofs is a `Vec` because the sampling function might give more than 1 sample points,
	/// so need to verify each sample point with its proofs
	fn complete_proofs(
		proposal_id: RelayProposalId<Self::GameId>,
		proofs: Vec<Self::Proofs>,
	) -> DispatchResult;

	/// Once there're different opinions in a game,
	/// chain will ask relayer to submit more samples
	/// to help the chain make a on chain arbitrate finally
	fn extend_proposal(
		relayer: Self::Relayer,
		samples: Vec<Self::RelayParcel>,
		extended_relay_proposal_id: RelayProposalId<Self::GameId>,
		proofses: Option<Vec<Self::Proofs>>,
	) -> DispatchResult;

	fn approve_pending_relay_parcel(pending_relay_block_id: Self::GameId) -> DispatchResult;

	fn reject_pending_relay_parcel(pending_relay_block_id: Self::GameId) -> DispatchResult;
}

#[derive(Clone, Encode, Decode, RuntimeDebug)]
pub struct RelayProposal<RelayParcel, Relayer, Balance, GameId> {
	pub relayer: Relayer,
	pub relay_parcels: Vec<RelayParcel>,
	pub bond: Balance,
	pub maybe_extended_proposal_id: Option<RelayProposalId<GameId>>,
	pub verified: bool,
}
impl<RelayParcel, Relayer, Balance, GameId> RelayProposal<RelayParcel, Relayer, Balance, GameId>
where
	Relayer: Default,
	Balance: Zero,
{
	pub fn new() -> Self {
		Self {
			relayer: Relayer::default(),
			relay_parcels: vec![],
			bond: Zero::zero(),
			maybe_extended_proposal_id: None,
			verified: false,
		}
	}
}
