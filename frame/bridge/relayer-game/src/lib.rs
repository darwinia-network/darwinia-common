//! # Relayer Game Module
//!
//! ## Assumption
//! 1. At least **one** honest relayer
//! 2. Each proposal's header hash is unique at a certain block height
//!
//!
//! ## Flow
//! 1. Request the header in target chain's relay module,
//! weather the header is existed or not you should pay some fees
//! 2. If not, target chain's relay module will ask for a proposal here

#![cfg_attr(not(feature = "std"), no_std)]
#![feature(drain_filter)]

mod types {
	// --- darwinia ---
	use crate::*;

	pub type AccountId<T> = <T as frame_system::Trait>::AccountId;
	pub type BlockNumber<T> = <T as frame_system::Trait>::BlockNumber;

	pub type RingBalance<T, I> = <RingCurrency<T, I> as Currency<AccountId<T>>>::Balance;
	pub type RingNegativeImbalance<T, I> =
		<RingCurrency<T, I> as Currency<AccountId<T>>>::NegativeImbalance;

	pub type TcBlockNumber<T, I> = <Tc<T, I> as Relayable>::TcBlockNumber;
	pub type TcHeaderHash<T, I> = <Tc<T, I> as Relayable>::TcHeaderHash;

	pub type GameId<TcBlockNumber> = TcBlockNumber;
	pub type ProposalId<TcBlockNumber, TcHeaderHash> = (TcBlockNumber, TcHeaderHash);

	type RingCurrency<T, I> = <T as Trait<I>>::RingCurrency;

	type Tc<T, I> = <T as Trait<I>>::TargetChain;
}

// --- crates ---
use codec::{Decode, Encode};
// --- substrate ---
use frame_support::{
	debug::error,
	decl_error, decl_event, decl_module, decl_storage, ensure,
	traits::{Currency, ExistenceRequirement, OnUnbalanced},
};
use frame_system::{self as system, ensure_signed};
use sp_runtime::{
	traits::{Convert, Zero},
	DispatchResult, RuntimeDebug,
};
#[cfg(not(feature = "std"))]
use sp_std::borrow::ToOwned;
use sp_std::prelude::*;
// --- darwinia ---
use darwinia_support::{balance::lock::*, relay::*};
use types::*;

const RELAYER_GAME_ID: LockIdentifier = *b"da/rgame";

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
		TcBlockNumber = TcBlockNumber<Self, I>,
		Sampler = Self::Sampler,
	>;
	type Sampler: Convert<Round, Vec<TcBlockNumber<Self, I>>> + Convert<u32, Round>;

	/// The target chain's relay module's API
	type TargetChain: Relayable;
}

decl_event! {
	pub enum Event<T, I: Instance = DefaultInstance>
	where
		AccountId = AccountId<T>,
		BlockNumber = BlockNumber<T>,
	{
		/// TODO
		TODO(AccountId, BlockNumber),
	}
}

decl_error! {
	pub enum Error for Module<T: Trait<I>, I: Instance> {
		/// Challenge - NOT HAPPENED
		ChallengeNH,

		/// Proposal - ALREADY EXISTED
		ProposalAE,
		/// Target Header - ALREADY EXISTED
		TargetHeaderAE,

		/// Round - MISMATCHED
		RoundMis,

		/// Can not bond with value less than usable balance.
		InsufficientValue,
	}
}

decl_storage! {
	trait Store for Module<T: Trait<I>, I: Instance = DefaultInstance> as DarwiniaRelayerGame {
		/// Each target chain's header relay can open a game
		pub Proposals
			get(fn proposals_of_game)
			: map hasher(blake2_128_concat) GameId<TcBlockNumber<T, I>>
			=> Vec<Proposal<
				AccountId<T>,
				RingBalance<T, I>,
				TcHeaderId<TcBlockNumber<T, I>, TcHeaderHash<T, I>>,
				ProposalId<TcBlockNumber<T, I>, TcHeaderHash<T, I>>
			>>;

		/// The allow samples for each game
		pub Samples
			get(fn samples_of_game)
			: map hasher(blake2_128_concat) TcBlockNumber<T, I>
			=> Vec<TcBlockNumber<T, I>>;

		/// The closed rounds which had passed the challenge time at this moment
		pub ClosedRounds
			get(fn closed_rounds_at)
			: map hasher(blake2_128_concat) BlockNumber<T>
			=> Vec<(GameId<TcBlockNumber<T, I>>, Round)>;

		/// All the `TcHeader`s store here, **NON-DUPLICATIVE**
		pub TcHeaders
			get(fn tc_header)
			: map hasher(blake2_128_concat) TcHeaderId<TcBlockNumber<T, I>, TcHeaderHash<T, I>>
			=> RefTcHeader;

		/// The finalize blocks' header's id which is recorded in darwinia
		pub ConfirmedTcHeaderIds
			get(fn confirmed_tc_header_id)
			: TcHeaderId<TcBlockNumber<T, I>, TcHeaderHash<T, I>>;

		pub Bonds
			get(fn bond_of_relayer)
			: map hasher(blake2_128_concat) AccountId<T>
			=> RingBalance<T, I>;
	}
}

decl_module! {
	pub struct Module<T: Trait<I>, I: Instance = DefaultInstance> for enum Call
	where
		origin: T::Origin
	{
		type Error = Error<T, I>;

		fn deposit_event() = default;

		// TODO: too many db operations? move to `offchain_worker`?
		fn on_finalize(block_number: BlockNumber<T>) {
			let proposals = <ClosedRounds<T, I>>::take(block_number);

			if proposals.len() == 0 {
				return;
			}

			for (game_id, round) in proposals {
				let proposals = Self::proposals_of_game(game_id)
					.into_iter()
					.filter(|proposal|
						T::RelayerGameAdjustor::round_from_chain_len(proposal.chain.len() as _)
							== round
					)
					.collect::<Vec<_>>();
				let proposals_len = proposals.len();

				if proposals_len == 0 {
					return;
				}

				if proposals_len == 1 {
					// Chain's len is ALWAYS great than 1 under this match pattern; qed
					let proposal = proposals[0].clone();
					let mut extend_from = proposal.extend_from.clone();

					while let Some((extend_from_block_number, extend_from_header_hash))
						= extend_from.clone()
					{
						let mut relayer = None;
						let mut evils = vec![];

						for proposal in <Proposals<T, I>>::mutate(
							extend_from_block_number,
							|proposals| proposals
								.drain_filter(|proposal|
									T::RelayerGameAdjustor::round_from_chain_len(
										proposal.chain.len() as _
									) != round
								)
								.collect::<Vec<_>>()
						) {
							if let Some(BondedTcHeader { id, bond }) = proposal.chain.last() {
								let (_, header_hash) = id;
								let mut header = <TcHeaders<T, I>>::take(id);

								if header_hash == &extend_from_header_hash {
									if relayer.is_none() {
										relayer = Some(proposal.relayer);
									} else {
										error!("[relayer-game] \
											Honest Relayer MORE THAN 1 Within a Round");
									}

									extend_from = proposal.extend_from.clone();
									header.status = TcHeaderStatus::Confirmed;
								} else {
									if let Some(ref_count) = header.ref_count.checked_sub(1) {
										header.ref_count = ref_count;
									} else {
										error!("[relayer-game] `RefTcHeader.ref_count` BELOW 0");
									}

									evils.push((proposal.relayer, *bond));
									header.status = TcHeaderStatus::Invalid;
								}

								if header.ref_count != 1 {
									<TcHeaders<T, I>>::insert(id, header);
								}
							} else {
								error!("[relayer-game] Proposal Is EMPTY");
							}
						}

						// TODO: modify `Bonds`
						if let Some(relayer) = relayer {
							for (evil, bond) in evils {
								let _ = T::RingCurrency::transfer(
									&evil,
									&relayer,
									bond,
									ExistenceRequirement::KeepAlive
								);
							}
						} else {
							// Should NEVER enter this condition
							for (_, bond) in evils {
								let (imbalance, _) = T::RingCurrency
									::slash(&proposal.relayer, bond);
								T::RingSlash::on_unbalanced(imbalance);
							}

							error!("[relayer-game] NO Honest Relayer");
						}
					}

					// TODO: reward if no challenge
				} else {
					<Samples<T, I>>::mutate(proposals[0].chain[0].id.0, |samples| {
						T::RelayerGameAdjustor::update_samples(
							T::RelayerGameAdjustor
								::round_from_chain_len(proposals[0].chain.len() as _),
							T::TargetChain::highest_confirmed_at(),
							samples
						);
					});
				}
			}
		}

		// TODO:
		//	The `header_thing_chain` could be very large,
		//	the bond should relate to the bytes fee
		//	that we slash the evil relayer(s) to reward the honest relayer(s)
		#[weight = 0]
		fn submit_proposal(origin, raw_header_thing_chain: Vec<RawHeaderThing>) {
			let relayer = ensure_signed(origin)?;
			let (game_id, _) = T::TargetChain
				::verify_raw_header_thing(&raw_header_thing_chain[0])?;
			let other_proposals = Self::proposals_of_game(game_id);
			let other_proposals_len = other_proposals.len();
			let build_bonded_chain = |chain: Vec<_>| {
				chain
					.into_iter()
					.enumerate()
					.map(|(round, id)| {
						BondedTcHeader {
							id,
							bond: T::RelayerGameAdjustor::estimate_bond(
								round as _,
								other_proposals_len as _
							)
						}
					})
					.collect::<Vec<_>>()
			};
			// Always `add_bonded_header` first, this could cause an err
			let add_boned_headers = |
				bonded_headers: &[BondedTcHeader<_, _>],
				raw_header_thing_chain: Vec<RawHeaderThing>
			| -> DispatchResult {
				// TODO: modify `Bonds`
				let mut headers = vec![];
				let mut bond = Zero::zero();

				for (bonded_header, raw_header_thing) in bonded_headers
					.iter()
					.cloned()
					.zip(raw_header_thing_chain.into_iter())
				{
					let id = bonded_header.id;
					let mut header = Self::tc_header(&id);

					if header.ref_count == 0 {
						header = RefTcHeader {
							raw_header_thing: raw_header_thing.to_owned(),
							ref_count: 1,
							status: TcHeaderStatus::Unknown,
						};
					} else {
						header.ref_count = header
							.ref_count
							.checked_sub(1)
							.ok_or("`RefTcHeader.ref_count` Overflow \
								But I Think That's IMPOSSIABLE")?;
					}

					headers.push((id, header));
					bond += bonded_header.bond;
				}

				// TODO: estimate the bond at the beginning to save resources(calc)
				ensure!(
					T::RingCurrency::usable_balance(&relayer) >= bond,
					<Error<T, I>>::InsufficientValue
				);

				for (k, v) in headers {
					<TcHeaders<T, I>>::insert(k, v);
				}
				T::RingCurrency::set_lock(
					RELAYER_GAME_ID,
					&relayer,
					LockFor::Common { amount: bond },
					WithdrawReasons::all(),
				);

				Ok(())
			};

			match (other_proposals_len, raw_header_thing_chain.len()) {
				// New `Game`
				(0, raw_header_thing_chain_len) => {
					ensure!(raw_header_thing_chain_len == 1, <Error<T, I>>::RoundMis);
					ensure!(
						!T::TargetChain::header_existed(game_id),
						<Error<T, I>>::TargetHeaderAE
					);

					let id_chain = T::TargetChain
						::verify_raw_header_thing_chain(&raw_header_thing_chain)?;
					let chain = build_bonded_chain(id_chain);

					add_boned_headers(&chain, raw_header_thing_chain)?;
					<Proposals<T, I>>::insert(game_id, vec![Proposal {
						relayer,
						chain,
						extend_from: None
					}]);
					<ClosedRounds<T, I>>::append(
						<frame_system::Module<T>>::block_number()
							+ T::RelayerGameAdjustor::challenge_time(0),
						(game_id, 0)
					);
					<Samples<T, I>>::insert(game_id, vec![game_id]);
				}
				// First round
				(_, 1) => {
					if other_proposals.iter().any(|proposal| proposal.chain.len() != 1) {
						Err(<Error<T, I>>::RoundMis)?;
					}

					let id_chain = T::TargetChain
						::verify_raw_header_thing_chain(&raw_header_thing_chain)?;

					ensure!(
						!other_proposals
							.into_iter()
							.any(|proposal| &proposal.chain[0].id == &id_chain[0]),
						<Error<T, I>>::ProposalAE
					);

					let chain = build_bonded_chain(id_chain);

					add_boned_headers(&chain, raw_header_thing_chain)?;
					<Proposals<T, I>>::insert(game_id, vec![Proposal {
						relayer,
						chain,
						extend_from: None
					}]);

				}
				// Extend
				(_, raw_header_thing_chain_len) => {
					let round = T::RelayerGameAdjustor
						::round_from_chain_len(raw_header_thing_chain_len as _);
					let prev_round = round.checked_sub(1).ok_or(<Error<T, I>>::RoundMis)?;
					let id_chain = T::TargetChain
						::verify_raw_header_thing_chain(&raw_header_thing_chain)?;
					let samples = {
						// Chain's len is ALWAYS great than 1 under this match pattern; qed
						let (game_id, _) = id_chain[0];
						Self::samples_of_game(game_id)
					};

					ensure!(id_chain.len() == samples.len(), <Error<T, I>>::RoundMis);
					ensure!(
						id_chain
							.iter()
							.zip(samples.iter())
							.all(|((block_number, _), sample_block_number)| block_number
								== sample_block_number),
						<Error<T, I>>::RoundMis
					);

					let chain = build_bonded_chain(id_chain);
					let all_headers_equal = |
						a: &[BondedTcHeader<_, _>],
						b: &[BondedTcHeader<_, _>]
					| {
						a.iter().zip(b.iter()).all(|(a, b)| a.id == b.id)
					};
					let mut extend_from_proposal = None;
					// An optimize here, to skip the checking of extended headers
					// The shorter chain is ALWAYS at the head of `other_proposals`
					let mut extend_at = 0;

					for proposal in other_proposals {
						let proposal_round = T::RelayerGameAdjustor
							::round_from_chain_len(proposal.chain.len() as _);

						if proposal_round == prev_round {
							extend_at = proposal.chain.len();
							if all_headers_equal(&chain, &proposal.chain) {
								extend_from_proposal = Some(proposal);
							}
						} else if proposal_round == round {
							ensure!(
								all_headers_equal(
									// A chain MUST longer than the chain which it extend from; qed
									&chain[extend_at..],
									&proposal.chain[extend_at..]
								),
								<Error<T, I>>::ProposalAE
							);
						}
					}

					if let Some(Proposal { chain: extend_from_chain, ..}) = extend_from_proposal {
						// A chain MUST longer than the chain which it extend from; qed
						add_boned_headers(
							&chain[extend_at..],
							raw_header_thing_chain[extend_at..].to_vec()
						)?;
						<Proposals<T, I>>::append(
							game_id,
							Proposal {
								relayer,
								chain,
								// Each proposal MUST contains a NOT empty chain; qed
								extend_from: Some(extend_from_chain
									.last()
									.unwrap()
									.id
									.clone())
							}
						);
						{
							let closed_at = <frame_system::Module<T>>::block_number()
								+ T::RelayerGameAdjustor::challenge_time(round);
							let	mut closed_rounds = Self::closed_rounds_at(closed_at);

							if !closed_rounds.contains(&(game_id, round)) {
								closed_rounds.push((game_id, round));
								<ClosedRounds<T, I>>::insert(closed_at, closed_rounds);
							}
						}
					} else {
						Err(<Error<T, I>>::RoundMis)?;
					}
				}
			}
		}
	}
}

impl<T: Trait<I>, I: Instance> Module<T, I> {}

#[derive(Clone, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum TcHeaderStatus {
	/// The header has not been judged yet
	Unknown,
	/// The header had been confirmed by game
	Confirmed,
	/// The header had been confirmed by game but too old
	/// Means we might not use this header anymore so drop it to free the storage
	Outdated,
	/// The header is invalid
	Invalid,
}
impl Default for TcHeaderStatus {
	fn default() -> Self {
		Self::Unknown
	}
}

#[derive(Clone, Default, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct Proposal<AccountId, Balance, TcHeaderId, ProposalId> {
	// TODO: Can this proposal submit by other relayers?
	/// The relayer of these series of headers
	/// The proposer of this proposal
	/// The person who support this proposal with some bonds
	relayer: AccountId,
	/// A series of target chain's header ids and the value that relayer had bonded for it
	chain: Vec<BondedTcHeader<Balance, TcHeaderId>>,
	/// Parents (previous proposal's id)
	///
	/// If this field is `None` that means this proposal is the first proposal
	extend_from: Option<ProposalId>,
}

#[derive(Clone, Default, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct BondedTcHeader<Balance, TcHeaderId> {
	id: TcHeaderId,
	bond: Balance,
}

#[derive(Clone, Default, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct RefTcHeader {
	/// Codec style `Header` or `HeaderWithProofs` or ...
	/// That you defined in target chain's relay module use for verifying
	raw_header_thing: RawHeaderThing,
	/// Maybe two or more proposals are using the same `Header`
	/// Drop it while the `ref_count` is zero but **NOT** in `ConfirmedTcHeaders` list
	ref_count: u32,
	/// Help chain to end a round quickly
	status: TcHeaderStatus,
}
