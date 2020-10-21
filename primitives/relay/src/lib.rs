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

pub trait RelayHeaderParcelInfo {
	type HeaderId: Clone;

	fn header_id(&self) -> Self::HeaderId;
}

/// Implement this for target chain's relay module's
/// to expose some necessary APIs for relayer game
pub trait Relayable {
	type RelayHeaderId: Clone + Debug + Default + PartialOrd + FullCodec;
	type RelayHeaderParcel: Clone
		+ Debug
		+ PartialEq
		+ FullCodec
		+ RelayHeaderParcelInfo<HeaderId = Self::RelayHeaderId>;
	type RelayProofs;

	/// The latest finalize block's id which recorded in darwinia
	fn best_confirmed_block_id() -> Self::RelayHeaderId;

	// TODO: optimize this
	fn verify_relay_proofs(
		relay_header_id: &Self::RelayHeaderId,
		relay_parcel: &Self::RelayHeaderParcel,
		relay_proofs: &Self::RelayProofs,
		optional_best_confirmed_block_id: Option<&Self::RelayHeaderId>,
	) -> DispatchResult;

	fn verify_continuous(
		relay_header_parcel: &Self::RelayHeaderParcel,
		extended_relay_parcel: &Self::RelayHeaderParcel,
	) -> DispatchResult;

	fn verify_relay_chain(relay_chain: Vec<&Self::RelayHeaderParcel>) -> DispatchResult;

	fn distance_between(
		relay_header_id: &Self::RelayHeaderId,
		best_confirmed_block_id: Self::RelayHeaderId,
	) -> u32;

	fn store_relay_parcel(relay_parcel: Self::RelayHeaderParcel) -> DispatchResult;
}

/// A regulator to adjust relay args for a specific chain
/// Implement this in runtime's `impls.rs`
pub trait AdjustableRelayerGame {
	type Moment;
	type Balance;
	type RelayHeaderId;

	/// The maximum number of active games
	///
	/// This might relate to the validators count
	fn max_active_games() -> u8;

	fn propose_time(round: u32) -> Self::Moment;

	fn complete_proofs_time(round: u32) -> Self::Moment;

	/// Update the game's sample points
	///
	/// Push the new samples to the `sample_points`, the index of `sample_points` aka round index
	/// And return the new samples
	fn update_sample_points(sample_points: &mut Vec<Vec<Self::RelayHeaderId>>);

	/// Give an estimate bond value for a specify round
	///
	/// Usally the bond value go expensive wihle the round and the affirmations count increase
	fn estimate_bond(round: u32, proposals_count: u8) -> Self::Balance;
}

pub trait RelayerGameProtocol {
	type Relayer;
	type RelayHeaderId: Clone + PartialOrd;
	type RelayHeaderParcel: Clone
		+ Debug
		+ PartialEq
		+ FullCodec
		+ RelayHeaderParcelInfo<HeaderId = Self::RelayHeaderId>;
	type RelayProofs;

	fn get_proposed_relay_parcels(
		proposal_id: RelayAffirmationId<Self::RelayHeaderId>,
	) -> Option<Vec<Self::RelayHeaderParcel>>;

	/// Game's entry point, call only at the first round
	///
	/// Arrirm a new affirmation or against a existed affirmation
	fn affirm(
		relayer: Self::Relayer,
		relay_parcel: Self::RelayHeaderParcel,
		optional_relay_proofs: Option<Self::RelayProofs>,
	) -> DispatchResult;

	/// Verify a specify affirmation
	///
	/// Proofs is a `Vec` because the sampling function might give more than 1 sample points,
	/// so need to verify each sample point with its proofs
	fn complete_relay_proofs(
		proposal_id: RelayAffirmationId<Self::RelayHeaderId>,
		relay_proofs: Vec<Self::RelayProofs>,
	) -> DispatchResult;

	/// Once there're different opinions in a game,
	/// chain will ask relayer to submit more samples
	/// to help the chain make a on chain arbitrate finally
	fn extend_affirmation(
		relayer: Self::Relayer,
		game_sample_points: Vec<Self::RelayHeaderParcel>,
		extended_relay_affirmation_id: RelayAffirmationId<Self::RelayHeaderId>,
		optional_relay_proofs: Option<Vec<Self::RelayProofs>>,
	) -> DispatchResult;

	fn approve_pending_relay_header_parcel(
		pending_relay_block_id: Self::RelayHeaderId,
	) -> DispatchResult;

	fn reject_pending_relay_header_parcel(
		pending_relay_block_id: Self::RelayHeaderId,
	) -> DispatchResult;
}

/// Game id, round and the index under the round point to a unique affirmation AKA affirmation id
#[derive(Clone, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct RelayAffirmationId<RelayHeaderId> {
	/// Relay header id aka game id
	pub relay_header_id: RelayHeaderId,
	/// Round index
	pub round: u32,
	/// Index of a affirmation list which under a round
	pub index: u32,
}

#[derive(Clone, Encode, Decode, RuntimeDebug)]
pub struct RelayAffirmation<RelayHeaderParcel, Relayer, Balance, RelayHeaderId> {
	pub relayer: Relayer,
	pub relay_header_parcels: Vec<RelayHeaderParcel>,
	pub bond: Balance,
	pub maybe_extended_relay_affirmation_id: Option<RelayAffirmationId<RelayHeaderId>>,
	pub verified: bool,
}
impl<RelayHeaderParcel, Relayer, Balance, RelayHeaderId>
	RelayAffirmation<RelayHeaderParcel, Relayer, Balance, RelayHeaderId>
where
	Relayer: Default,
	Balance: Zero,
{
	pub fn new() -> Self {
		Self {
			relayer: Relayer::default(),
			relay_header_parcels: vec![],
			bond: Zero::zero(),
			maybe_extended_relay_affirmation_id: None,
			verified: false,
		}
	}
}
