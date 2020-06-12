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
		pub Games
			get(fn proposals_of_game)
			: map hasher(blake2_128_concat) GameId<TcBlockNumber<T, I>>
			=> Vec<Proposal<
				T::AccountId,
				RingBalance<T, I>,
				TcBlockNumber<T, I>,
				TcHeaderHash<T, I>
			>>;

		/// The closed rounds which had passed the challenge time at this moment
		pub ClosedRounds
			get(fn closed_rounds_at)
			: map hasher(blake2_128_concat) T::BlockNumber
			=>  Vec<(GameId<TcBlockNumber<T, I>>, Round)>;

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
					.map(|(round, header_id)| (header_id, T::RelayerGameAdjustor::estimate_bond(
						round as _,
						other_proposals_len as _
					)))
					.collect())
			};

			if other_proposals_len == 0 {
				ensure!(
					!T::TargetChain::header_existed(game_id),
					<Error<T, I>>::ProposalAE
				);
				ensure!(raw_header_thing_chain.len() == 1, <Error<T, I>>::RoundMis);

				let chain = build_from_raw_header_chain()?;

				for ((tc_header_id, _), raw_header_thing) in chain
					.iter()
					.cloned()
					.zip(raw_header_thing_chain.into_iter())
				{
					<TcHeaders<T, I>>::mutate(tc_header_id, |ref_tc_header| {
						if ref_tc_header.ref_count == 0 {
							*ref_tc_header = RefTcHeader {
								raw_header_thing,
								ref_count: 1,
								status: TcHeaderStatus::Unknown,
							};
						} else {
							ref_tc_header.ref_count += 1;
						}
					});
				}
				<Games<T, I>>::insert(game_id, vec![Proposal {
					relayer,
					chain,
					extend_from: None
				}]);
				<ClosedRounds<T, I>>::mutate(
					<frame_system::Module<T>>::block_number(),
					|closed_rounds| closed_rounds.push((game_id, 0))
				);
			} else {
				let next_round =
					T::RelayerGameAdjustor::round_from_chain_len(raw_header_thing_chain.len() as _);
				let extend_from = other_proposals
					.into_iter()
					.position(|proposal| {
						T::RelayerGameAdjustor::round_from_chain_len(proposal.chain.len() as _) + 1
							== next_round
					})
					.map(|i| i as _);
				if extend_from.is_some() {
					let chain = build_from_raw_header_chain()?;

					<Games<T, I>>::mutate(game_id, |proposals| proposals.push(Proposal {
						relayer,
						chain,
						extend_from,
					}));
					<ClosedRounds<T, I>>::mutate(
						<frame_system::Module<T>>::block_number(),
						|closed_rounds| closed_rounds.push((game_id, 0))
					);
				} else {
					Err(<Error<T, I>>::RoundMis)?;
				}
			}

			// 	<Proposals<T, I>>::insert(game_id, proposal_id, Proposal {
			// 		relayer:,
			// 		chain:,
			// 		extend_from:,
			// 	});
			// 	ensure!(rounds_proposals_count > 1, <Error<T, I>>::ChallengeNH);
		}
	}
}

impl<T: Trait<I>, I: Instance> Module<T, I> {
	// /// Whether the submission window is open
	// fn proposal_is_open(at: BlockNumber<T>) -> bool {}
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
	chain: Vec<(TcHeaderId<TcBlockNumber, TcHeaderHash>, Balance)>,
	/// Parents (previous proposal)
	///
	/// If this field is `None` that means this proposal is the first proposal
	extend_from: Option<RoundIndex>,
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
