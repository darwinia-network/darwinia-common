//! Relayer Game Primitives

#![cfg_attr(not(feature = "std"), no_std)]

// --- core ---
use core::fmt::Debug;
// --- crates ---
use codec::{Decode, Encode, FullCodec};
// --- substrate ---
use sp_runtime::{traits::AtLeast32BitUnsigned, DispatchError, DispatchResult, RuntimeDebug};
use sp_std::prelude::*;

pub type Round = u64;

/// Implement this for target chain's relay module's
/// to expose some necessary APIs for relayer game
pub trait Relayable {
	type HeaderThing: Clone + Debug + Default + PartialEq;
	type HeaderBrief: Clone
		+ Debug
		+ Default
		+ PartialEq
		+ FullCodec
		+ HeaderBrief<Hash = Self::HeaderHash, BlockNumber = Self::BlockNumber>;
	type BlockNumber: Clone + Copy + Debug + Default + AtLeast32BitUnsigned + FullCodec;
	type HeaderHash: Clone + Debug + Default + PartialEq + FullCodec;

	fn basic_verify(
		proposal: Vec<Self::HeaderThing>,
	) -> Result<Vec<Self::HeaderBrief>, DispatchError>;

	/// The latest finalize block's header's record id in darwinia
	fn best_block_number() -> Self::BlockNumber;

	/// On chain arbitrate, to confirmed the header with 100% sure
	fn on_chain_arbitrate(header_brief_chain: Vec<Self::HeaderBrief>) -> DispatchResult;

	/// Store the header confirmed in relayer game
	fn store_header(header_thing: Self::HeaderBrief) -> DispatchResult;
}
pub trait HeaderBrief {
	type Hash;
	type BlockNumber;

	fn hash(&self) -> Self::Hash;

	fn block_number(&self) -> Self::BlockNumber;
}

// A regulator to adjust relay args for a specific chain
// Implement this in runtime's impls
pub trait AdjustableRelayerGame {
	type Moment;
	type Balance;
	type TcBlockNumber;

	fn challenge_time(round: Round) -> Self::Moment;

	fn round_from_chain_len(chain_len: u64) -> Round;

	fn chain_len_from_round(round: Round) -> u64;

	fn update_samples(samples: &mut Vec<Vec<Self::TcBlockNumber>>);

	fn estimate_bond(round: Round, proposals_count: u64) -> Self::Balance;
}

pub trait RelayerGameProtocol {
	type Relayer;
	type HeaderThing;
	type BlockNumber;

	fn submit_proposal(relayer: Self::Relayer, proposal: Vec<Self::HeaderThing>) -> DispatchResult;

	fn approve_pending_header(pending: Self::BlockNumber) -> DispatchResult;

	fn reject_pending_header(pending: Self::BlockNumber) -> DispatchResult;
}

#[derive(Clone, Encode, Decode, RuntimeDebug)]
pub struct RelayProposal<AccountId, BondedHeader, HeaderHash> {
	// TODO: Can this proposal submit by other relayers?
	/// The relayer of these series of headers
	/// The proposer of this proposal
	/// The person who support this proposal with some bonds
	pub relayer: AccountId,
	/// A series of target chain's header ids and the value that relayer had bonded for it
	pub bonded_proposal: Vec<BondedHeader>,
	/// Parents (previous header hash)
	///
	/// If this field is `None` that means this proposal is the first proposal
	pub extend_from_header_hash: Option<HeaderHash>,
}

#[derive(Clone, Encode, Decode, RuntimeDebug)]
pub struct BondedHeaderBrief<Balance, HeaderBrief> {
	pub header_brief: HeaderBrief,
	pub bond: Balance,
}

pub fn extend_proposal<Balance, HeaderBrief, F>(
	proposal: &[HeaderBrief],
	extend_at: Round,
	other_proposals_len: usize,
	estimate_bond: F,
) -> (Balance, Vec<BondedHeaderBrief<Balance, HeaderBrief>>)
where
	Balance: Copy + AtLeast32BitUnsigned,
	HeaderBrief: Clone,
	F: Fn(Round, u64) -> Balance,
{
	let mut bonds = Balance::zero();

	(
		bonds,
		proposal
			.iter()
			.cloned()
			.enumerate()
			.map(|(round_offset, header_brief)| {
				let bond =
					estimate_bond(extend_at + round_offset as Round, other_proposals_len as _);

				bonds = bonds.saturating_add(bond);

				BondedHeaderBrief { header_brief, bond }
			})
			.collect(),
	)
}
