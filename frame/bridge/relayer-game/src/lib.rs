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
	debug::error, decl_error, decl_event, decl_module, decl_storage, ensure, traits::Currency,
};
use frame_system::{self as system, ensure_signed};
use sp_runtime::{traits::Convert, DispatchError, RuntimeDebug};
#[cfg(not(feature = "std"))]
use sp_std::borrow::ToOwned;
use sp_std::prelude::*;
// --- darwinia ---
use darwinia_support::{balance::lock::*, relay::*};
use types::*;

pub trait Trait<I: Instance = DefaultInstance>: frame_system::Trait {
	type Event: From<Event<Self, I>> + Into<<Self as frame_system::Trait>::Event>;

	/// The currency use for bond
	type RingCurrency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

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
		/// Proposal - ALREADY EXISTED
		ProposalAE,
		/// Target Header - ALREADY EXISTED
		TargetHeaderAE,

		/// Round - MISMATCHED
		RoundMis,

		/// Challenge - NOT HAPPENED
		ChallengeNH,
	}
}

decl_storage! {
	trait Store for Module<T: Trait<I>, I: Instance = DefaultInstance> as DarwiniaRelayerGame {
		/// Each target chain's header relay can open a game
		pub Games
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
	}
}

decl_module! {
	pub struct Module<T: Trait<I>, I: Instance = DefaultInstance> for enum Call
	where
		origin: T::Origin
	{
		type Error = Error<T, I>;

		fn deposit_event() = default;

		fn on_finalize(block_number: BlockNumber<T>) {
			let proposals = <ClosedRounds<T, I>>::take(block_number);
			match proposals.len() {
				0 => (),
				_ => {
					for (game_id, round) in proposals {
						let proposals = Self::proposals_of_game(game_id)
							.into_iter()
							.filter(|proposal|
								T::RelayerGameAdjustor
									::round_from_chain_len(proposal.chain.len() as _)
										== round
							)
							.collect::<Vec<_>>();

						match proposals.len() {
							0 => (),
							1 => {
								// chain's len is always great than 1 under this match pattern; qed
								let proposal = proposals[0].clone();
								let mut extend_from = proposal.extend_from.clone();
								while let
									Some((extend_from_block_number, extend_from_header_hash))
										= extend_from.clone()
								{
									let mut reward = 0;

									for proposal in <Games<T, I>>::mutate(
										extend_from_block_number,
										|proposals|
											proposals
												.drain_filter(|proposal|
													T::RelayerGameAdjustor::round_from_chain_len(
														proposal.chain.len() as _
													) != round
												)
												.collect::<Vec<_>>()
									) {
										if let Some(BondedTcHeader { id: (_, header_hash), .. })
											= proposal.chain.last()
										{
											if header_hash == &extend_from_header_hash {
												reward += 1;
												extend_from = proposal.extend_from.clone();
												// TODO: reward
											} else {
												// TODO: punish
											}
										} else {
											error!("[relayer-game] Proposal Is EMPTY");
										}
									}

									match reward {
										0 => error!("[relayer-game] NO Honest Relayer"),
										1 => (),
										_ => error!("[relayer-game] Honest Relayer MORE THAN 1 \
											Within a Round"),
									}
								}
								// TODO: reward
							}
							_ => {
								<Samples<T, I>>::mutate(proposals[0].chain[0].id.0, |samples| {
									T::RelayerGameAdjustor::update_samples(
										T::RelayerGameAdjustor::round_from_chain_len(
											proposals[0]
												.chain
												.len() as _
										),
										T::TargetChain::highest_confirmed_at(),
										samples
									);
								});
							}
						}
					}
				}
			}
		}

		// TODO:
		//	the `header_thing_chain` could be very large,
		//	the bond should relate to the bytes fee
		//	that we slash the evil relayer(s) to reward the honest relayer(s)
		#[weight = 0]
		fn submit_proposal(
			origin,
			target_block_number: TcBlockNumber<T, I>,
			raw_header_thing_chain: Vec<Vec<u8>>
		) {
			let relayer = ensure_signed(origin)?;
			let game_id = target_block_number;
			let other_proposals = Self::proposals_of_game(game_id);
			let other_proposals_len = other_proposals.len();
			let build_chain = || -> Result<Vec<_>, DispatchError> {
				Ok(T::TargetChain::verify_raw_header_thing_chain(&raw_header_thing_chain)?
					.into_iter()
					.enumerate()
					.map(|(round, header_id)| BondedTcHeader {
						id: header_id,
						bond: T::RelayerGameAdjustor::estimate_bond(
							round as _,
							other_proposals_len as _
						)
					})
					.collect())
			};
			let add_ref_tc_header = |header_id, raw_header_thing: &[_]| {
				<TcHeaders<T, I>>::mutate(header_id, |header|
					match header.ref_count {
						0 => *header = RefTcHeader {
							raw_header_thing: raw_header_thing.to_owned(),
							ref_count: 1,
							status: TcHeaderStatus::Unknown,
						},
						_ => header.ref_count += 1,
					}
				)
			};

			match (other_proposals_len, raw_header_thing_chain.len()) {
				(0, raw_header_thing_chain_len) => {
					ensure!(raw_header_thing_chain_len == 1, <Error<T, I>>::RoundMis);
					ensure!(
						!T::TargetChain::header_existed(game_id),
						<Error<T, I>>::TargetHeaderAE
					);

					let chain = build_chain()?;

					add_ref_tc_header(&chain[0].id, &raw_header_thing_chain[0]);
					<Games<T, I>>::insert(game_id, vec![Proposal {
						relayer,
						chain,
						extend_from: None
					}]);
					<ClosedRounds<T, I>>::mutate(
						<frame_system::Module<T>>::block_number()
							+ T::RelayerGameAdjustor::challenge_time(0),
						|closed_rounds| closed_rounds.push((game_id, 0))
					);
					<Samples<T, I>>::insert(game_id, vec![game_id]);
				}
				(_, 1) => {
					if other_proposals.iter().any(|proposal| proposal.chain.len() != 1) {
						Err(<Error<T, I>>::RoundMis)?;
					}

					let chain = build_chain()?;

					if other_proposals
						.into_iter()
						.any(|proposal| &proposal.chain[0].id == &chain[0].id)
					{
						Err(<Error<T, I>>::ProposalAE)?;
					}

					add_ref_tc_header(&chain[0].id, &raw_header_thing_chain[0]);
					<Games<T, I>>::insert(game_id, vec![Proposal {
						relayer,
						chain,
						extend_from: None
					}]);

				}
				(_, raw_header_thing_chain_len) => {
					let round = T::RelayerGameAdjustor
						::round_from_chain_len(raw_header_thing_chain_len as _);
					let prev_round = round.checked_sub(1).ok_or(<Error<T, I>>::RoundMis)?;
					let chain = build_chain()?;
					let samples = {
						// chain's len is always great than 1 under this match pattern; qed
						let BondedTcHeader { id: (game_id, _), .. } = chain[0];
						Self::samples_of_game(game_id)
					};

					ensure!(chain.len() == samples.len(), <Error<T, I>>::RoundMis);
					ensure!(
						chain
							.iter()
							.zip(samples.iter())
							.all(|(BondedTcHeader { id: (block_number, _), .. },
								sample_block_number)|
									block_number == sample_block_number
							),
						<Error<T, I>>::RoundMis
					);

					let all_headers_equal = |a: &[_], b: &[_]| {
						a.iter().zip(b.iter()).all(|(a, b)| a == b)
					};
					let mut extend_from_proposal = None;
					let mut extend_at = 0;

					for proposal in other_proposals {
						match T::RelayerGameAdjustor
							::round_from_chain_len(proposal.chain.len() as _)
						{
							proposal_round if proposal_round == prev_round => {
								extend_at = proposal.chain.len();
								if all_headers_equal(&chain, &proposal.chain) {
									extend_from_proposal = Some(proposal);
								}
							}
							proposal_round if proposal_round == round => {
								if all_headers_equal(
									// a chain must longer than the chain which it extend from; qed
									&chain[extend_at..],
									&proposal.chain[extend_at..]
								) {
									Err(<Error<T, I>>::ProposalAE)?;
								}
							}
							_ => ()
						}
					}

					if let Some(Proposal { chain: extend_from_chain, ..}) = extend_from_proposal {
						// a chain must longer than the chain which it extend from; qed
						for i in extend_at..chain.len() {
							add_ref_tc_header(&chain[i].id, &raw_header_thing_chain[i]);
						}
						<Games<T, I>>::mutate(
							game_id,
							|proposals|
								proposals.push(Proposal {
									relayer,
									chain,
									// each proposal must contains a NOT empty chain; qed
									extend_from: Some(extend_from_chain[0].id.clone())
								})
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
	raw_header_thing: Vec<u8>,
	/// Maybe two or more proposals are using the same `Header`
	/// Drop it while the `ref_count` is zero but **NOT** in `ConfirmedTcHeaders` list
	ref_count: u32,
	/// Help chain to end a round quickly
	status: TcHeaderStatus,
}
