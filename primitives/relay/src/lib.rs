//! Relayer Game Primitives

#![cfg_attr(not(feature = "std"), no_std)]
#![feature(drain_filter)]

// --- core ---
use core::fmt::Debug;
// --- crates ---
use codec::{Decode, Encode, FullCodec};
// --- substrate ---
use sp_runtime::DispatchResult;
#[cfg(not(feature = "std"))]
use sp_std::borrow::ToOwned;
use sp_std::prelude::*;

pub type Round = u64;

/// Implement this for target chain's relay module's
/// to expose some necessary APIs for relayer game
pub trait Relayable {
	type BlockId: Clone + PartialOrd + FullCodec;
	type RelayStuffs: Debug + PartialEq + PartialOrd + FullCodec;
	type Proofs;

	/// The latest finalize block's header's record id in darwinia
	fn best_block_id() -> Self::BlockId;

	fn verify_proofs(relay_stuffs: &Self::RelayStuffs, proofs: &Self::Proofs) -> DispatchResult;
}

// A regulator to adjust relay args for a specific chain
// Implement this in runtime's impls
pub trait AdjustableRelayerGame {
	type Moment;
	type Balance;
	type BlockId;

	// The maximum of active games
	//
	// This might relate to the validators count
	fn max_active_games() -> u8;

	fn challenge_time(round: Round) -> Self::Moment;

	fn update_samples(samples: &mut Vec<Vec<Self::BlockId>>);

	fn estimate_bond(round: Round, proposals_count: u8) -> Self::Balance;
}

pub trait RelayerGameProtocol {
	type Relayer;
	type GameId: Clone + PartialOrd;
	type RelayStuffs: Debug + PartialEq + PartialOrd + FullCodec;
	type Proofs;

	fn propose(
		relayer: Self::Relayer,
		game_id: Self::GameId,
		relay_stuffs: Self::RelayStuffs,
		proofs: Option<Self::Proofs>,
	) -> DispatchResult;
}

#[derive(Debug, Encode, Decode)]
pub struct RelayProposal<RelayStuffs, AccountId, Balance> {
	pub proposed: Vec<RelayStuffs>,
	pub bonds: Vec<(AccountId, Balance)>,
	pub extended_proposal_id: Option<()>,
	pub verified: bool,
}
impl<RelayStuffs, AccountId, Balance> RelayProposal<RelayStuffs, AccountId, Balance> {
	pub fn new() -> Self {
		Self {
			proposed: vec![],
			bonds: vec![],
			extended_proposal_id: None,
			verified: false,
		}
	}
}
