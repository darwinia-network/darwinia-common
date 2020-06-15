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

mod types {
	// --- darwinia ---
	use crate::*;

	pub type AccountId<T> = <T as frame_system::Trait>::AccountId;
	pub type BlockNumber<T> = <T as frame_system::Trait>::BlockNumber;
	pub type RingBalance<T, I> = <RingCurrency<T, I> as Currency<AccountId<T>>>::Balance;

	pub type TcBlockNumber<T, I> = <Tc<T, I> as Relayable>::TcBlockNumber;
	pub type TcHeaderHash<T, I> = <Tc<T, I> as Relayable>::TcHeaderHash;

	pub type GameId<TcBlockNumber> = TcBlockNumber;
	pub type RoundIndex = u32;

	type RingCurrency<T, I> = <T as Trait<I>>::RingCurrency;

	type Tc<T, I> = <T as Trait<I>>::TargetChain;
}

// --- crates ---
use codec::{Decode, Encode};
// --- substrate ---
use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure, traits::Currency};
use frame_system::{self as system, ensure_signed};
use sp_runtime::{traits::Convert, DispatchError, RuntimeDebug};
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
				TcBlockNumber<T, I>,
				TcHeaderHash<T, I>
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
						let mut proposals = Self::proposals_of_game(game_id)
							.into_iter()
							.filter(|proposal| Self::round_of_chain(proposal.chain.len() as _)
								== round)
							.collect::<Vec<_>>();

						match proposals.len() {
							0 => (),
							1 => {
								// chain's len is always great than 1 under this match pattern; qed
								let mut proposal = proposals.pop().unwrap();
								while let Some(extend_from) = proposal.extend_from {
									<Games<T, I>>::mutate(extend_from, |proposals| {

									});
								}
							}
							_ => {}
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
			let build_from_raw_header_chain = || -> Result<Vec<_>, DispatchError> {
				Ok(T::TargetChain::verify_header_chain(&raw_header_thing_chain)?
					.into_iter()
					.enumerate()
					.map(|(round, header_id)| TcHeaderIdWithBond {
						id: header_id,
						bond: T::RelayerGameAdjustor::estimate_bond(
							round as _,
							other_proposals_len as _
						)
					})
					.collect())
			};
			let add_ref_tc_header = |tc_header_id, raw_header_thing| {
				<TcHeaders<T, I>>::mutate(tc_header_id, |ref_tc_header|
					match ref_tc_header.ref_count {
						0 => *ref_tc_header = RefTcHeader {
							raw_header_thing,
							ref_count: 1,
							status: TcHeaderStatus::Unknown,
						},
						_ => ref_tc_header.ref_count += 1,
					}
				)
			};

			match other_proposals_len {
				0 => {
					ensure!(raw_header_thing_chain.len() == 1, <Error<T, I>>::RoundMis);
					ensure!(
						!T::TargetChain::header_existed(game_id),
						<Error<T, I>>::TargetHeaderAE
					);

					let chain = build_from_raw_header_chain()?;

					for (tc_header_id_with_bond, raw_header_thing) in chain
						.iter()
						.cloned()
						.zip(raw_header_thing_chain.into_iter())
					{
						add_ref_tc_header(tc_header_id_with_bond.id, raw_header_thing);
					}
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
				}
				_ => {
					let round = Self::round_of_chain(raw_header_thing_chain.len() as _)
						.checked_sub(1)
						.ok_or(<Error<T, I>>::RoundMis)?;
					let chain = build_from_raw_header_chain()?;
					let samples = {
						// chain's len is always great than 1 under this match pattern; qed
						let TcHeaderIdWithBond { id: (game_id, _), .. } = chain[0];
						Self::samples_of_game(game_id)
					};

					ensure!(
						raw_header_thing_chain.len() == samples.len(),
						<Error<T, I>>::RoundMis
					);
					ensure!(
						chain
							.iter()
							.zip(samples.iter())
							.all(|(TcHeaderIdWithBond { id: (block_number, _), .. },
								sample_block_number)| block_number == sample_block_number),
						<Error<T, I>>::RoundMis
					);

					let extend_from = other_proposals
						.into_iter()
						.find(|proposal|
							(Self::round_of_chain(proposal.chain.len() as _) == round) && chain
								.iter()
								.zip(proposal.chain.iter())
								.all(|(a, b)| a == b))
						// each proposal must contains a NOT empty chain; qed
						.map(|proposal| (proposal.chain[0].id.0));
					if extend_from.is_some() {
						{
							// chain's len is always great than 1 under this match pattern; qed
							let TcHeaderIdWithBond { id: tc_header_id, .. } = chain
								.last()
								.unwrap()
								.clone();
							let raw_header_thing = raw_header_thing_chain
								.last()
								.unwrap()
								.clone();
							add_ref_tc_header(tc_header_id, raw_header_thing);
						}
						<Games<T, I>>::mutate(game_id, |proposals| proposals.push(Proposal {
							relayer,
							chain,
							extend_from,
						}));
						{
							let next_round = round + 1;
							<ClosedRounds<T, I>>::mutate(
								<frame_system::Module<T>>::block_number()
									+ T::RelayerGameAdjustor::challenge_time(next_round),
								|closed_rounds| closed_rounds.push((game_id, next_round))
							);
						}
					} else {
						Err(<Error<T, I>>::RoundMis)?;
					}
				}
			}
		}
	}
}

impl<T: Trait<I>, I: Instance> Module<T, I> {
	fn round_of_chain(chain_len: u32) -> Round {
		T::RelayerGameAdjustor::round_from_chain_len(chain_len)
	}
}

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
pub struct Proposal<AccountId, Balance, TcBlockNumber, TcHeaderHash> {
	// TODO: Can this proposal submit by other relayers?
	/// The relayer of these series of headers
	/// The proposer of this proposal
	/// The person who support this proposal with some bonds
	relayer: AccountId,
	/// A series of target chain's header ids and the value that relayer had bonded for it
	chain: Vec<TcHeaderIdWithBond<Balance, TcBlockNumber, TcHeaderHash>>,
	/// Parents (previous block number)
	///
	/// If this field is `None` that means this proposal is the first proposal
	extend_from: Option<TcBlockNumber>,
}

#[derive(Clone, Default, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct TcHeaderIdWithBond<Balance, TcBlockNumber, TcHeaderHash> {
	id: TcHeaderId<TcBlockNumber, TcHeaderHash>,
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
