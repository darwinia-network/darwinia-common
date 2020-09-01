//! Relayer Game Primitives

#![cfg_attr(not(feature = "std"), no_std)]
#![feature(drain_filter)]

// --- core ---
use core::fmt::Debug;
// --- crates ---
use codec::{Decode, Encode, FullCodec};
// --- substrate ---
use frame_support::debug::error;
use sp_runtime::{traits::AtLeast32BitUnsigned, DispatchError, DispatchResult, RuntimeDebug};
#[cfg(not(feature = "std"))]
use sp_std::borrow::ToOwned;
use sp_std::{collections::btree_map::BTreeMap, prelude::*};

pub type Round = u64;

/// Implement this for target chain's relay module's
/// to expose some necessary APIs for relayer game
pub trait Relayable {
	type HeaderThingWithProof: Debug;
	type HeaderThing: HeaderThing;
	// type BlockNumber: Clone + Copy + Debug + Default + AtLeast32BitUnsigned + FullCodec;
	// type HeaderHash: Clone + Debug + Default + PartialEq + FullCodec;

	fn basic_verify(
		proposal_with_proof: Vec<Self::HeaderThingWithProof>,
	) -> Result<Vec<Self::HeaderThing>, DispatchError>;

	/// The latest finalize block's header's record id in darwinia
	fn best_block_number() -> <Self::HeaderThing as HeaderThing>::Number;

	/// On chain arbitrate, to confirmed the header with 100% sure
	fn on_chain_arbitrate(proposal: Vec<Self::HeaderThing>) -> DispatchResult;

	/// Store the header confirmed in relayer game
	fn store_header(header_thing: Self::HeaderThing) -> DispatchResult;
}
pub trait HeaderThing: Clone + Debug + Default + PartialEq + FullCodec {
	type Number: Clone + Copy + Debug + Default + AtLeast32BitUnsigned + FullCodec;
	type Hash: Clone + Debug + Default + PartialEq + FullCodec;

	fn number(&self) -> Self::Number;

	fn hash(&self) -> Self::Hash;
}

// A regulator to adjust relay args for a specific chain
// Implement this in runtime's impls
pub trait AdjustableRelayerGame {
	type Moment;
	type Balance;
	type TcBlockNumber;

	fn challenge_time(round: Round) -> Self::Moment;

	fn round_of_samples_count(samples_count: u64) -> Round;

	fn samples_count_of_round(round: Round) -> u64;

	fn update_samples(samples: &mut Vec<Vec<Self::TcBlockNumber>>);

	fn estimate_bond(round: Round, proposals_count: u64) -> Self::Balance;
}

pub trait RelayerGameProtocol {
	type Relayer;
	type Balance;
	type HeaderThingWithProof;
	type HeaderThing: HeaderThing;

	fn proposals_of_game(
		game_id: <Self::HeaderThing as HeaderThing>::Number,
	) -> Vec<
		RelayProposal<
			Self::Relayer,
			Self::Balance,
			Self::HeaderThing,
			<Self::HeaderThing as HeaderThing>::Hash,
		>,
	>;

	fn submit_proposal(
		relayer: Self::Relayer,
		proposal: Vec<Self::HeaderThingWithProof>,
	) -> DispatchResult;

	fn approve_pending_header(
		pending: <Self::HeaderThing as HeaderThing>::Number,
	) -> DispatchResult;

	fn reject_pending_header(pending: <Self::HeaderThing as HeaderThing>::Number)
		-> DispatchResult;
}

#[derive(Clone, Encode, Decode, RuntimeDebug)]
pub struct RelayProposal<Relayer, Balance, HeaderThing, HeaderHash> {
	// TODO: Can this proposal submit by other relayers?
	/// The relayer of these series of headers
	/// The proposer of this proposal
	/// The person who support this proposal with some bonds
	pub relayer: Relayer,
	/// A series of target chain's header brief and the value that relayer had bonded for it
	pub bonded_proposal: Vec<(Balance, HeaderThing)>,
	/// Parents (previous header hash)
	///
	/// If this field is `None` that means this proposal is the first proposal
	pub extend_from_header_hash: Option<HeaderHash>,
}

pub fn extend_proposal<Balance, HeaderThing, F>(
	proposal: &[HeaderThing],
	round: Round,
	other_proposals_len: usize,
	estimate_bond: F,
) -> (Balance, Vec<(Balance, HeaderThing)>)
where
	Balance: Copy + AtLeast32BitUnsigned,
	HeaderThing: Clone,
	F: Fn(Round, u64) -> Balance,
{
	let mut bonds = Balance::zero();
	let bonded_proposal = proposal
		.iter()
		.cloned()
		.enumerate()
		.map(|(round_offset, header_thing)| {
			let bond = estimate_bond(round + round_offset as Round, other_proposals_len as _);

			bonds = bonds.saturating_add(bond);

			(bond, header_thing)
		})
		.collect();

	(bonds, bonded_proposal)
}

pub fn build_reward_map<Relayer, Balance, HeaderThing, HeaderHash, F>(
	mut round: Round,
	mut proposals: Vec<RelayProposal<Relayer, Balance, HeaderThing, HeaderHash>>,
	mut extend_from_header_hash: HeaderHash,
	mut rewards: Vec<((Relayer, Balance), (Relayer, Balance))>,
	round_of_samples_count: F,
) -> (
	BTreeMap<Relayer, Balance>,
	BTreeMap<Relayer, (Balance, BTreeMap<Relayer, Balance>)>,
	Vec<Vec<(Relayer, Balance)>>,
)
where
	Relayer: Clone + Ord,
	Balance: Copy + AtLeast32BitUnsigned,
	HeaderThing: crate::HeaderThing<Hash = HeaderHash>,
	HeaderHash: Clone + Debug + Default + PartialEq + FullCodec,
	F: Fn(u64) -> Round,
{
	let mut missing = vec![];

	// If there's no extended at first round,
	// that means this proposal MUST be the first proposal
	// Else,
	// it MUST extend from some; qed
	// TODO: while let Some()? to remove the round
	while round > 0 {
		round -= 1;

		let mut maybe_honesty = None;
		let mut evils = vec![];

		for proposal in proposals_filter_by_round(&mut proposals, round, &round_of_samples_count) {
			let (bond, header_thing) = proposal.bonded_proposal.last().unwrap();
			let header_hash = header_thing.hash();

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

			missing.push(evils);

			error!("Honest Relayer - NOT FOUND");
		}
	}

	// Use for updating relayers' bonds and locks with just 2 DB writes
	let mut honesties_map = BTreeMap::new();
	// Use for updating evils' bonds, locks and reward relayers
	let mut evils_map = BTreeMap::new();

	for ((honesty, honesty_bonds), (evil, evil_bond)) in rewards {
		*honesties_map
			.entry(honesty.clone())
			.or_insert(honesty_bonds) += honesty_bonds;

		let evil_map_ptr = evils_map.entry(evil).or_insert({
			let mut slash_map = BTreeMap::new();

			slash_map.insert(honesty.clone(), evil_bond);

			// The first item means total bonds
			// which use for updating bonds and locks with just 2 DB writes
			//
			// The second item use for rewarding relayers
			(evil_bond, slash_map)
		});

		evil_map_ptr.0 += evil_bond;
		*evil_map_ptr.1.entry(honesty).or_insert(evil_bond) += evil_bond;
	}

	(honesties_map, evils_map, missing)
}

pub fn proposals_filter_by_round<R, B, HB, HH, F>(
	proposals: &mut Vec<RelayProposal<R, B, HB, HH>>,
	round: Round,
	round_of_samples_count: F,
) -> Vec<RelayProposal<R, B, HB, HH>>
where
	F: Fn(u64) -> Round,
{
	proposals
		.drain_filter(|proposal| {
			round_of_samples_count(proposal.bonded_proposal.len() as _) == round
		})
		.collect()
}
