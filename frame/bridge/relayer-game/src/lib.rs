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

	pub type RelayBlockId<T, I> = <RelayableChainT<T, I> as Relayable>::RelayBlockId;
	pub type RelayParcel<T, I> = <RelayableChainT<T, I> as Relayable>::RelayParcel;
	pub type RelayProofs<T, I> = <RelayableChainT<T, I> as Relayable>::Proofs;

	pub type RelayProposalT<T, I> =
		RelayProposal<RelayParcel<T, I>, AccountId<T>, RingBalance<T, I>, RelayBlockId<T, I>>;

	type RingCurrency<T, I> = <T as Trait<I>>::RingCurrency;

	type RelayableChainT<T, I> = <T as Trait<I>>::RelayableChain;
}

// --- substrate ---
use frame_support::{
	debug::*,
	decl_error, decl_event, decl_module, decl_storage, ensure,
	traits::{Currency, Get, OnUnbalanced},
	weights::Weight,
};
use frame_system::{
	ensure_none,
	offchain::{SendTransactionTypes, SubmitTransaction},
};
use sp_runtime::{
	traits::{Saturating, Zero},
	DispatchError, DispatchResult,
};
#[cfg(not(feature = "std"))]
use sp_std::borrow::ToOwned;
use sp_std::{collections::btree_map::BTreeMap, prelude::*};
// --- darwinia ---
use darwinia_relay_primitives::*;
use darwinia_support::balance::lock::*;
use types::*;

pub const RELAYER_GAME_ID: LockIdentifier = *b"da/rgame";

pub trait Trait<I: Instance = DefaultInstance>:
	frame_system::Trait + SendTransactionTypes<Call<Self, I>>
{
	type Call: From<Call<Self, I>>;

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

	/// A chain which implemented `Relayable` trait
	type RelayableChain: Relayable;

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
		AccountId = AccountId<T>,
		RelayBlockId = RelayBlockId<T, I>,
	{
		/// A new relay parcel proposed. [relay block id, round, index, relayer]
		RelayProposed(RelayBlockId, u32, u32, AccountId),
		/// A new round started. [relay block id, game sample points]
		NewRound(RelayBlockId, Vec<RelayBlockId>),
		/// A game has been settled. [relay block id]
		GameOver(RelayBlockId),
		/// Pending relay parcel approved. [relay block id, reason]
		PendingRelayParcelApproved(RelayBlockId, Vec<u8>),
		/// Pending relay parcel rejected. [relay block id]
		PendingRelayParcelRejected(RelayBlockId),
	}
}

decl_error! {
	pub enum Error for Module<T: Trait<I>, I: Instance> {
		/// Relay Stuffs - ALREADY RELAIED
		RelayStuffsAR,
		/// Round - MISMATCHED
		RoundMis,
		/// Active Games - TOO MANY
		ActiveGamesTM,
		/// Game - CLOSED
		GameC,
		/// Relay Proposal - DUPLICATED
		RelayProposalDup,
		/// Usable *RING* for Bond - INSUFFICIENT
		BondIns,
		/// Relay Proofs Quantity - INVALID
		RelayProofsQuantityInv,
		/// Relay Proposal - NOT EXISTED
		RelayProposalNE,
		/// Extended Relay Proposal - NOT EXISTED
		ExtendedRelayProposalNE,
		/// Previous Relay Proofs - INCOMPLETE
		PreviousRelayProofsInc,
		/// Pending Relay Parcel - NOT EXISTED
		PendingRelayParcelNE,
	}
}

decl_storage! {
	trait Store for Module<T: Trait<I>, I: Instance = DefaultInstance> as DarwiniaRelayerGame {
		/// Active games' relay block ids
		pub BlocksToRelay get(fn blocks_to_relay): Vec<RelayBlockId<T, I>>;

		/// All the active games' proposals here
		///
		/// The first key is relay block id, the second key is round index
		/// then you will get the proposals under that round in that game
		pub Proposals
			get(fn proposals_of_game_at)
			: double_map
				hasher(identity) RelayBlockId<T, I>,
				hasher(identity) u32
			=> Vec<RelayProposalT<T, I>>;

		/// The best relaied block id record of a game when it start
		pub BestRelaiedBlockId
			get(fn best_relaied_block_id_of)
			: map hasher(identity) RelayBlockId<T, I>
			=> RelayBlockId<T, I>;

		/// The total rounds of a game
		///
		/// `total rounds - 1 = last round index`
		pub RoundCounts
			get(fn round_count_of)
			: map hasher(identity) RelayBlockId<T, I>
			=> u32;

		/// All the closed games here
		///
		/// Game close at this moment, closed games won't accept any proposal
		pub ProposeEndTime
			get(fn propose_end_time_of)
			: map hasher(identity) RelayBlockId<T, I>
			=> BlockNumber<T>;

		/// All the closed rounds here
		///
		/// Record the closed rounds endpoint which use for settlling or updating
		/// Settle or update a game will be scheduled which will start at this moment
		pub GamesToUpdate
			get(fn games_to_update_at)
			: map hasher(identity) BlockNumber<T>
			=> Vec<RelayBlockId<T, I>>;

		/// All the bonds here
		pub Bonds
			get(fn bonds_of)
			: map hasher(blake2_128_concat) AccountId<T>
			=> RingBalance<T, I>;

		pub GameSamplePoints
			get(fn game_sample_points)
			:map hasher(identity) RelayBlockId<T, I>
			=> Vec<Vec<RelayBlockId<T, I>>>;

		pub PendingRelayParcels
			get(fn pending_relay_parcels)
			: Vec<(BlockNumber<T>, RelayBlockId<T, I>, RelayParcel<T, I>)>
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
			// TODO: handle error
			Self::system_approve_pending_relay_parcels(block_number).unwrap_or(0)
		}

		fn offchain_worker(now: BlockNumber<T>) {
			let game_ids = <GamesToUpdate<T, I>>::take(now);

			if !game_ids.is_empty() {
				if let Err(e) = Self::update_games_at(game_ids, now) {
					error!(target: "relayer-game", "{:?}", e);
				}
			}

			// Return while no closed rounds found
		}

		#[weight = 100_000_000]
		pub fn update_games_unsigned(origin, game_ids: Vec<RelayBlockId<T, I>>) {
			ensure_none(origin)?;

			let now = <frame_system::Module<T>>::block_number();
			let mut relay_parcels = vec![];

			for game_id in game_ids {
				trace!(
					target: "relayer-game",
					">  Trying to Settle Game `{:?}`", game_id
				);

				let round_count = Self::round_count_of(&game_id);
				let last_round = if let Some(last_round) = round_count.checked_sub(1) {
					last_round
				} else {
					// Should never enter this condition
					error!(target: "relayer-game", "   >  Rounds - EMPTY");

					continue;
				};
				let mut proposals = Self::proposals_of_game_at(&game_id, last_round);

				match (last_round, proposals.len()) {
					// Should never enter this condition
					(0, 0) => error!(target: "relayer-game", "   >  Proposals - EMPTY"),
					// At first round and only one proposal found
					(0, 1) => {
						trace!(target: "relayer-game", "   >  Challenge - NOT EXISTED");

						if let Some(relay_parcel) =
							Self::settle_without_challenge(proposals.pop().unwrap())
						{
							relay_parcels.push((game_id.to_owned(), relay_parcel));
						}
					}
					// No relayer response for the lastest round
					(_, 0) => {
						trace!(target: "relayer-game", "   >  All Relayers Abstain");

						Self::settle_abandon(proposals);
					},
					// No more challenge found at latest round, only one relayer win
					(_, 1) => {
						trace!(target: "relayer-game", "   >  No More Challenge");

						if let Some(relay_parcel) =
							Self::settle_with_challenge(&game_id, proposals.pop().unwrap())
						{
							relay_parcels.push((game_id.to_owned(), relay_parcel));
						}
					}
					(last_round, _) => {
						trace!(target: "relayer-game", "   >  Challenge Found");

						let distance = T::RelayableChain::distance_between(
							&game_id,
							Self::best_relaied_block_id_of(&game_id)
						);

						if distance == round_count {
							// A whole chain gave, start continuous verification
							if let Some(relay_parcel) = Self::on_chain_arbitrate(&game_id, last_round) {
								relay_parcels.push((game_id.to_owned(), relay_parcel));
							}
						} else {
							// Update game, start next round
							Self::update_game_at(&game_id, last_round, now);

							continue;
						}
					}
				}

				Self::game_over(game_id);
			}

			Self::store_relay_parcels(now, relay_parcels)?;

			trace!(target: "relayer-game", "---");
		}
	}
}

impl<T: Trait<I>, I: Instance> Module<T, I> {
	/// Check if time for proposing
	pub fn is_game_open_at(game_id: &RelayBlockId<T, I>, moment: BlockNumber<T>) -> bool {
		Self::propose_end_time_of(game_id) > moment
	}

	/// Check if others already make a same proposal
	pub fn is_unique_proposal(
		proposed_relay_parcels: &[RelayParcel<T, I>],
		existed_proposals: &[RelayProposalT<T, I>],
	) -> bool {
		!existed_proposals.iter().any(|existed_proposal| {
			existed_proposal.relay_parcels.as_slice() == proposed_relay_parcels
		})
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

	pub fn update_timer_of_game_at(
		game_id: &RelayBlockId<T, I>,
		round: u32,
		moment: BlockNumber<T>,
	) {
		let propose_time = moment + T::RelayerGameAdjustor::propose_time(round);
		let complete_proofs_time = T::RelayerGameAdjustor::complete_proofs_time(round);

		<ProposeEndTime<T, I>>::insert(game_id, propose_time);
		<GamesToUpdate<T, I>>::append(propose_time + complete_proofs_time, game_id);
	}

	pub fn update_games_at(
		game_ids: Vec<RelayBlockId<T, I>>,
		moment: BlockNumber<T>,
	) -> DispatchResult {
		trace!(target: "relayer-game", "Found Closed Rounds at `{:?}`", moment);
		trace!(target: "relayer-game", "---");

		let call = Call::update_games_unsigned(game_ids).into();

		<SubmitTransaction<T, Call<T, I>>>::submit_unsigned_transaction(call)
			.map_err(|_| "TODO".into())
	}

	pub fn update_bonds_with<F>(relayer: &AccountId<T>, calc_bonds: F)
	where
		F: FnOnce(RingBalance<T, I>) -> RingBalance<T, I>,
	{
		let bonds = calc_bonds(Self::bonds_of(relayer));

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

	pub fn slash_on(relayer: &AccountId<T>, bond: RingBalance<T, I>) {
		Self::update_bonds_with(relayer, |old_bonds| old_bonds.saturating_sub(bond));

		T::RingSlash::on_unbalanced(T::RingCurrency::slash(relayer, bond).0);
	}

	pub fn for_each_extended_proposal<F>(
		mut maybe_extended_proposal_id: Option<RelayProposalId<RelayBlockId<T, I>>>,
		mut f: F,
	) -> Option<RelayProposalT<T, I>>
	where
		F: FnMut(&RelayProposalT<T, I>, u32, u32),
	{
		if maybe_extended_proposal_id.is_none() {
			return None;
		}

		loop {
			let RelayProposalId {
				relay_block_id: game_id,
				round,
				index,
			} = maybe_extended_proposal_id.take().unwrap();

			if let Some(proposal) = Self::proposals_of_game_at(&game_id, round)
				.into_iter()
				.nth(index as _)
			{
				f(&proposal, round, index);

				if proposal.maybe_extended_proposal_id.is_none() {
					return Some(proposal);
				} else {
					maybe_extended_proposal_id = proposal.maybe_extended_proposal_id;
				}
			} else {
				// Should never enter this condition
				error!(target: "relayer-game", "   >  Proposal - NOT EXISTED");
			}
		}
	}

	// Build the honsties/evils(reward/slash) map follow a gave winning proposal
	pub fn build_honsties_evils_map(
		game_id: &RelayBlockId<T, I>,
		winning_proposal: RelayProposalT<T, I>,
	) -> (
		Option<RelayProposalT<T, I>>,
		BTreeMap<AccountId<T>, (RingBalance<T, I>, RingBalance<T, I>)>,
		BTreeMap<AccountId<T>, RingBalance<T, I>>,
	) {
		let RelayProposal {
			relayer,
			bond,
			maybe_extended_proposal_id,
			..
		} = winning_proposal;

		// BTreeMap<(relayer, unbond, reward)>
		let mut honesties = <BTreeMap<AccountId<T>, (RingBalance<T, I>, RingBalance<T, I>)>>::new();
		// BTreeMap<(relayer, slash)>
		let mut evils = <BTreeMap<AccountId<T>, RingBalance<T, I>>>::new();

		// TODO: reward on no challenge
		honesties.insert(relayer, (bond, Zero::zero()));

		let winning_proposal = Self::for_each_extended_proposal(
			maybe_extended_proposal_id,
			|RelayProposal {
			     relayer: honesty,
			     bond: honesty_bond,
			     ..
			 },
			 round,
			 index| {
				honesties
					.entry(honesty.to_owned())
					.and_modify(|(unbonds, _)| *unbonds = unbonds.saturating_add(*honesty_bond))
					.or_insert((*honesty_bond, Zero::zero()));

				for (i, RelayProposal { relayer, bond, .. }) in
					Self::proposals_of_game_at(game_id, round)
						.into_iter()
						.enumerate()
				{
					if i as u32 != index {
						honesties
							.entry(honesty.to_owned())
							.and_modify(|(_, rewards)| *rewards = rewards.saturating_add(bond));
						evils
							.entry(relayer)
							.and_modify(|slashs| *slashs = slashs.saturating_add(bond))
							.or_insert(Zero::zero());
					}
				}
			},
		);

		(winning_proposal, honesties, evils)
	}

	/// Try to find a winning proposal to build the honsties/evils(reward/slash) map
	pub fn try_build_honsties_evils_map(
		game_id: &RelayBlockId<T, I>,
		last_round: u32,
	) -> (
		Option<(
			RelayProposalT<T, I>,
			BTreeMap<AccountId<T>, (RingBalance<T, I>, RingBalance<T, I>)>,
		)>,
		BTreeMap<AccountId<T>, RingBalance<T, I>>,
	) {
		let mut round = last_round;
		// Vec<(id, relayer, unbond)>
		// Remove evil by this id each loop to find the honesties finally
		let mut honesties_candidates = vec![];
		// Vec<slashs>
		// Reward the round's winner with the round's total slash
		// But don't reward the round's winner if can't find a winning proposal finally
		let mut rounds_slashs = vec![];
		// BTreeMap<(relayer, slash)>
		let mut evils = <BTreeMap<AccountId<T>, RingBalance<T, I>>>::new();
		// Vec<(id, proposal)>
		// Remove evil proposal by this id each loop
		// to find the winning proposals(a winning proposal chain) finally
		let mut winning_proposal_candidates = Self::proposals_of_game_at(game_id, round)
			.into_iter()
			.filter(|proposal| proposal.verified)
			.enumerate()
			.collect::<Vec<_>>();
		// Verified proposals under current `round`
		let mut valid_proposals = winning_proposal_candidates.clone();
		let mut maybe_winning_proposal = None;

		loop {
			// A set of the previous round valid-proposal which the winning proposal candidate extend from
			let mut next_valid_proposals = vec![];

			if round == 0 {
				if next_valid_proposals.len() == 1 {
					maybe_winning_proposal = next_valid_proposals.pop();
				}

				break;
			}

			let previous_round = round - 1;
			let previous_round_proposals = Self::proposals_of_game_at(&game_id, previous_round);

			for (
				id,
				RelayProposal {
					ref relayer,
					ref relay_parcels,
					bond,
					ref maybe_extended_proposal_id,
					..
				},
			) in valid_proposals
			{
				if let Some(RelayProposalId { index, .. }) = maybe_extended_proposal_id {
					if let Some(extended_proposal) = previous_round_proposals.get(*index as usize) {
						// FIXME
						// if T::RelayableChain::verify_continuous(
						// 	relay_parcels,
						// 	&extended_proposal.relay_parcels,
						// )
						// .is_ok()
						if true {
							// Can't find the relayer cheating within this round
							// So add the relayer and his proposal into the candidate
							honesties_candidates.push((id, relayer.to_owned(), bond));
							next_valid_proposals.push((id, extended_proposal.to_owned()));
						} else {
							evils
								.entry(relayer.to_owned())
								.and_modify(|slashs| *slashs = slashs.saturating_add(bond))
								.or_insert(bond);
							rounds_slashs.push(bond);
						}
					} else {
						// Should never enter this condition
						error!(
							target: "relayer-game",
							"   >  During Finding Winning Proposal, Extended Proposal - NOT EXISTED"
						);
					}
				} else {
					// Should never enter this condition
					error!(
						target: "relayer-game",
						"   >  During Finding Winning Proposal, Proposal Extend From - NOTHING"
					);
				}
			}

			// Remove all the evils and record the slash
			honesties_candidates = honesties_candidates
				.into_iter()
				.filter(|(id, relayer, bond)| {
					if next_valid_proposals.iter().any(|(id_, _)| id == id_) {
						true
					} else {
						evils
							.entry(relayer.to_owned())
							.and_modify(|slashs| *slashs = slashs.saturating_add(*bond))
							.or_insert(*bond);
						rounds_slashs.push(*bond);

						false
					}
				})
				.collect();
			// Remove all the evil proposals
			winning_proposal_candidates = winning_proposal_candidates
				.into_iter()
				.filter(|(id, _)| next_valid_proposals.iter().any(|(id_, _)| id == id_))
				.collect();

			round = previous_round;
			valid_proposals = next_valid_proposals;
		}

		match winning_proposal_candidates.len() {
			0 => (None, evils),
			1 => {
				if honesties_candidates.len() as u32 + 1 == last_round
					&& rounds_slashs.len() as u32 + 1 == last_round
					&& maybe_winning_proposal.is_some()
				{
					// The first(head) proposal of a winning proposal chain
					let (_, winning_proposal_head) = maybe_winning_proposal.unwrap();
					// The last(tail) proposal of a winning proposal chain
					let (_, winning_proposal_tail) = winning_proposal_candidates.pop().unwrap();
					let mut honesties = BTreeMap::new();

					// TODO: reward on no challenge
					honesties.insert(
						winning_proposal_tail.relayer,
						(winning_proposal_tail.bond, Zero::zero()),
					);

					for ((_, round_winner, unbond), round_rewards) in honesties_candidates
						.into_iter()
						.zip(rounds_slashs.into_iter())
					{
						honesties.insert(round_winner, (unbond, round_rewards));
					}

					(Some((winning_proposal_head, honesties)), evils)
				} else {
					// TODO: handle return
					// Should never enter this condition
					error!(
						target: "relayer-game",
						"   >  Honesties Count - MISMATCHED"
					);

					(None, evils)
				}
			}
			_ => {
				// TODO: handle return
				// Should never enter this condition
				error!(
					target: "relayer-game",
					"   >  Wining Proposal - MORE THAN ONE"
				);

				(None, evils)
			}
		}
	}

	pub fn payout_honesties_and_slash_evils(
		honesties: BTreeMap<AccountId<T>, (RingBalance<T, I>, RingBalance<T, I>)>,
		evils: BTreeMap<AccountId<T>, RingBalance<T, I>>,
	) {
		for (relayer, (unbonds, rewards)) in honesties {
			// Unlock bonds for honesty
			Self::update_bonds_with(&relayer, |bonds| bonds.saturating_sub(unbonds));

			// Reward honesty
			T::RingCurrency::deposit_creating(&relayer, rewards);
		}

		for (relayer, slashs) in evils {
			// Punish evil
			T::RingCurrency::slash(&relayer, slashs);
		}
	}

	pub fn settle_without_challenge(
		mut winning_proposal: RelayProposalT<T, I>,
	) -> Option<RelayParcel<T, I>> {
		Self::update_bonds_with(&winning_proposal.relayer, |bonds| {
			bonds.saturating_sub(winning_proposal.bond)
		});

		// TODO: reward on no challenge

		if winning_proposal.relay_parcels.len() == 1 {
			Some(winning_proposal.relay_parcels.pop().unwrap())
		} else {
			// Should never enter this condition
			error!(target: "relayer-game", "   >  Relay Parcels Count - MISMATCHED");

			None
		}
	}

	pub fn settle_abandon(abandoned_proposals: Vec<RelayProposalT<T, I>>) {
		for RelayProposal {
			relayer,
			bond,
			maybe_extended_proposal_id,
			..
		} in abandoned_proposals
		{
			Self::slash_on(&relayer, bond);
			Self::for_each_extended_proposal(
				maybe_extended_proposal_id,
				|RelayProposal { relayer, bond, .. }, _, _| {
					Self::slash_on(relayer, *bond);
				},
			);
		}
	}

	pub fn settle_with_challenge(
		game_id: &RelayBlockId<T, I>,
		// The last(tail) proposal of a winning proposal chain
		winning_proposal: RelayProposalT<T, I>,
	) -> Option<RelayParcel<T, I>> {
		// The first(head) proposal of a winning proposal chain
		let (maybe_winning_proposal, honesties, evils) =
			Self::build_honsties_evils_map(game_id, winning_proposal);

		Self::payout_honesties_and_slash_evils(honesties, evils);

		if let Some(mut winning_proposal) = maybe_winning_proposal {
			if winning_proposal.relay_parcels.len() == 1 {
				Some(winning_proposal.relay_parcels.pop().unwrap())
			} else {
				// Should never enter this condition
				error!(target: "relayer-game", "   >  Relay Parcels Count - MISMATCHED");

				None
			}
		} else {
			// Should never enter this condition
			error!(target: "relayer-game", "   >  Winning Proposal - NOT EXISTED");

			None
		}
	}

	pub fn on_chain_arbitrate(
		game_id: &RelayBlockId<T, I>,
		last_round: u32,
	) -> Option<RelayParcel<T, I>> {
		let (maybe_honesties, evils) = Self::try_build_honsties_evils_map(game_id, last_round);

		if let Some((mut winning_proposal, honesties)) = maybe_honesties {
			Self::payout_honesties_and_slash_evils(honesties, evils);

			if winning_proposal.relay_parcels.len() == 1 {
				Some(winning_proposal.relay_parcels.pop().unwrap())
			} else {
				// Should never enter this condition
				error!(target: "relayer-game", "   >  Relay Parcels Count - MISMATCHED");

				None
			}
		} else {
			Self::payout_honesties_and_slash_evils(BTreeMap::new(), evils);

			None
		}
	}

	pub fn update_game_at(game_id: &RelayBlockId<T, I>, last_round: u32, moment: BlockNumber<T>) {
		Self::update_timer_of_game_at(game_id, last_round + 1, moment);

		<GameSamplePoints<T, I>>::mutate(game_id, |game_sample_points| {
			T::RelayerGameAdjustor::update_sample_points(game_sample_points);

			Self::deposit_event(RawEvent::NewRound(
				game_id.to_owned(),
				game_sample_points
					.last()
					.map(ToOwned::to_owned)
					.unwrap_or_default(),
			));
		});
	}

	pub fn store_relay_parcels(
		now: BlockNumber<T>,
		pending_relay_parcels: Vec<(RelayBlockId<T, I>, RelayParcel<T, I>)>,
	) -> DispatchResult {
		let confirm_period = T::ConfirmPeriod::get();

		if confirm_period.is_zero() {
			for (_, pendingrelay_parcel) in pending_relay_parcels {
				T::RelayableChain::store_relay_parcel(pendingrelay_parcel)?;
			}
		} else {
			for (pending_relay_block_id, pendingrelay_parcel) in pending_relay_parcels {
				<PendingRelayParcels<T, I>>::append((
					now + confirm_period,
					pending_relay_block_id,
					pendingrelay_parcel,
				));
			}
		}

		Ok(())
	}

	pub fn game_over(game_id: RelayBlockId<T, I>) {
		// TODO: error trace
		let _ = <BlocksToRelay<T, I>>::try_mutate(|blocks_to_relay| {
			if let Some(i) = blocks_to_relay
				.iter()
				.position(|block_id| block_id == &game_id)
			{
				blocks_to_relay.remove(i);

				Ok(())
			} else {
				Err(())
			}
		});
		<Proposals<T, I>>::remove_prefix(&game_id);
		<BestRelaiedBlockId<T, I>>::take(&game_id);
		<RoundCounts<T, I>>::take(&game_id);
		<ProposeEndTime<T, I>>::take(&game_id);
		<GameSamplePoints<T, I>>::take(&game_id);

		Self::deposit_event(RawEvent::GameOver(game_id));
	}

	pub fn update_pending_relay_parcels_with<F>(
		pending_relay_block_id: &RelayBlockId<T, I>,
		f: F,
	) -> DispatchResult
	where
		F: FnOnce(RelayParcel<T, I>) -> DispatchResult,
	{
		<PendingRelayParcels<T, I>>::mutate(|pending_relay_parcels| {
			if let Some(i) = pending_relay_parcels
				.iter()
				.position(|(_, relay_block_id, _)| relay_block_id == pending_relay_block_id)
			{
				let (_, _, relay_parcel) = pending_relay_parcels.remove(i);

				f(relay_parcel)
			} else {
				Err(<Error<T, I>>::PendingRelayParcelNE)?
			}
		})
	}

	pub fn system_approve_pending_relay_parcels(
		now: BlockNumber<T>,
	) -> Result<Weight, DispatchError> {
		<PendingRelayParcels<T, I>>::mutate(|pending_relay_parcels| {
			pending_relay_parcels.retain(
				|(confirm_at, pending_relay_block_id, pendingrelay_parcel)| {
					if *confirm_at == now {
						// TODO: handle error
						let _ =
							T::RelayableChain::store_relay_parcel(pendingrelay_parcel.to_owned());

						Self::deposit_event(RawEvent::PendingRelayParcelApproved(
							pending_relay_block_id.to_owned(),
							b"Not Enough Technical Member Online, Approved By System".to_vec(),
						));

						false
					} else {
						true
					}
				},
			)
		});

		// TODO weight
		Ok(0)
	}
}

impl<T: Trait<I>, I: Instance> RelayerGameProtocol for Module<T, I> {
	type Relayer = AccountId<T>;
	type RelayBlockId = RelayBlockId<T, I>;
	type RelayParcel = RelayParcel<T, I>;
	type Proofs = RelayProofs<T, I>;

	fn get_proposed_relay_parcels(
		proposal_id: RelayProposalId<Self::RelayBlockId>,
	) -> Option<Vec<Self::RelayParcel>> {
		let RelayProposalId {
			relay_block_id: game_id,
			round,
			index,
		} = proposal_id;

		Self::proposals_of_game_at(&game_id, round)
			.into_iter()
			.nth(index as usize)
			.map(|proposal| proposal.relay_parcels)
	}

	fn propose(
		relayer: Self::Relayer,
		relay_parcel: Self::RelayParcel,
		optional_relay_proofs: Option<Self::Proofs>,
	) -> DispatchResult {
		trace!(
			target: "relayer-game",
			"Relayer `{:?}` propose:\n{:#?}",
			relayer,
			relay_parcel
		);

		let best_relaied_block_id = T::RelayableChain::best_relaied_block_id();
		let game_id = relay_parcel.block_id();

		// Check if the proposed header has already been relaied
		ensure!(
			game_id > best_relaied_block_id,
			<Error<T, I>>::RelayStuffsAR
		);
		// Make sure the game is at first round
		ensure!(
			<Proposals<T, I>>::decode_len(&game_id, 1).unwrap_or(0) == 0,
			<Error<T, I>>::RoundMis
		);

		let now = <frame_system::Module<T>>::block_number();
		let existed_proposals = Self::proposals_of_game_at(&game_id, 0);
		let proposed_relay_parcels = vec![relay_parcel];

		if existed_proposals.is_empty() {
			// A new game might open

			// Check if it is ok to open more games
			ensure!(
				<BlocksToRelay<T, I>>::decode_len()
					.map(|length| length as u8)
					.unwrap_or(0) < T::RelayerGameAdjustor::max_active_games(),
				<Error<T, I>>::ActiveGamesTM
			);
		} else {
			// An against proposal might add

			ensure!(Self::is_game_open_at(&game_id, now), <Error<T, I>>::GameC);
			// Currently not allow to vote for(relay) the same parcel
			ensure!(
				Self::is_unique_proposal(&proposed_relay_parcels, &existed_proposals),
				<Error<T, I>>::RelayProposalDup
			);
		}

		let bond = Self::ensure_can_bond(&relayer, 0, existed_proposals.len() as u8 + 1)?;

		Self::update_bonds_with(&relayer, |old_bonds| old_bonds.saturating_add(bond));

		let proposal = {
			let mut proposal = RelayProposal::new();

			proposal.relayer = relayer.clone();
			proposal.relay_parcels = proposed_relay_parcels;
			proposal.bond = bond;

			// Allow propose without relay proofs
			// The relay proofs can be completed later through `complete_proofs`
			if let Some(relay_proofs) = optional_relay_proofs {
				T::RelayableChain::verify_proofs(
					&game_id,
					&proposal.relay_parcels[0],
					&relay_proofs,
					Some(&best_relaied_block_id),
				)?;

				proposal.verified = true;
			}

			proposal
		};
		let round = 0;

		<Proposals<T, I>>::append(&game_id, round, proposal);
		<BestRelaiedBlockId<T, I>>::insert(&game_id, best_relaied_block_id);
		<RoundCounts<T, I>>::insert(&game_id, round);
		<BlocksToRelay<T, I>>::mutate(|blocks_to_relay| blocks_to_relay.push(game_id.clone()));

		Self::update_timer_of_game_at(&game_id, round, now);
		Self::deposit_event(RawEvent::RelayProposed(game_id, round, 0, relayer));

		Ok(())
	}

	fn complete_relay_proofs(
		proposal_id: RelayProposalId<Self::RelayBlockId>,
		relay_proofs: Vec<Self::Proofs>,
	) -> DispatchResult {
		let RelayProposalId {
			relay_block_id: game_id,
			round,
			index,
		} = proposal_id;

		<Proposals<T, I>>::try_mutate(&game_id, round, |proposals| {
			if let Some(proposal) = proposals.get_mut(index as usize) {
				for (relay_parcel, relay_proofs) in
					proposal.relay_parcels.iter().zip(relay_proofs.into_iter())
				{
					if round == 0 {
						T::RelayableChain::verify_proofs(
							&game_id,
							relay_parcel,
							&relay_proofs,
							Some(&Self::best_relaied_block_id_of(&game_id)),
						)?;
					} else {
						T::RelayableChain::verify_proofs(
							&game_id,
							relay_parcel,
							&relay_proofs,
							None,
						)?;
					}
				}

				proposal.verified = true;

				Ok(())
			} else {
				Err(<Error<T, I>>::RelayProposalNE.into())
			}
		})
	}

	fn extend_proposal(
		relayer: Self::Relayer,
		game_sample_points: Vec<Self::RelayParcel>,
		extended_relay_proposal_id: RelayProposalId<Self::RelayBlockId>,
		optional_relay_proofs: Option<Vec<Self::Proofs>>,
	) -> DispatchResult {
		let RelayProposalId {
			relay_block_id: game_id,
			round: previous_round,
			index: previous_index,
		} = extended_relay_proposal_id.clone();

		ensure!(
			Self::is_game_open_at(&game_id, <frame_system::Module<T>>::block_number()),
			<Error<T, I>>::GameC
		);

		if let Some(ref relay_proofs) = &optional_relay_proofs {
			ensure!(
				relay_proofs.len() == game_sample_points.len(),
				<Error<T, I>>::RelayProofsQuantityInv
			);
		}

		let round = previous_round + 1;
		let existed_proposals = Self::proposals_of_game_at(&game_id, round);

		ensure!(
			Self::is_unique_proposal(&game_sample_points, &existed_proposals),
			<Error<T, I>>::RelayProposalDup
		);

		let extended_proposal = existed_proposals
			.get(previous_index as usize)
			.ok_or(<Error<T, I>>::ExtendedRelayProposalNE)?;

		// Currently only accept extending from a completed proposal
		ensure!(
			extended_proposal.verified,
			<Error<T, I>>::PreviousRelayProofsInc
		);

		let bond = Self::ensure_can_bond(&relayer, round, existed_proposals.len() as u8 + 1)?;

		Self::update_bonds_with(&relayer, |old_bonds| old_bonds.saturating_add(bond));

		let proposal = {
			let mut proposal = RelayProposal::new();

			proposal.relayer = relayer.clone();
			proposal.relay_parcels = game_sample_points;
			proposal.bond = bond;
			proposal.maybe_extended_proposal_id = Some(extended_relay_proposal_id);

			// Allow propose without relay proofs
			// The relay proofs can be completed later through `complete_proofs`
			if let Some(relay_proofs) = optional_relay_proofs {
				for (relay_parcel, relay_proofs) in
					proposal.relay_parcels.iter().zip(relay_proofs.into_iter())
				{
					T::RelayableChain::verify_proofs(&game_id, relay_parcel, &relay_proofs, None)?;
				}

				proposal.verified = true;
			}

			proposal
		};

		<Proposals<T, I>>::append(&game_id, round, proposal);

		let index = <Proposals<T, I>>::decode_len(&game_id, round)
			.map(|length| length as u32)
			.unwrap_or(0);

		Self::deposit_event(RawEvent::RelayProposed(game_id, round, index, relayer));

		Ok(())
	}

	fn approve_pending_relay_parcel(pending_relay_block_id: Self::RelayBlockId) -> DispatchResult {
		Self::update_pending_relay_parcels_with(&pending_relay_block_id, |header| {
			T::RelayableChain::store_relay_parcel(header)
		})?;
		Self::deposit_event(RawEvent::PendingRelayParcelApproved(
			pending_relay_block_id,
			b"Approved By Tech.Comm".to_vec(),
		));

		Ok(())
	}

	fn reject_pending_relay_parcel(pending_relay_block_id: Self::RelayBlockId) -> DispatchResult {
		Self::update_pending_relay_parcels_with(&pending_relay_block_id, |_| Ok(()))?;
		Self::deposit_event(RawEvent::PendingRelayParcelRejected(pending_relay_block_id));

		Ok(())
	}
}
