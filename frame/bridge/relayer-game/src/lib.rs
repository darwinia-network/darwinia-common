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
	pub type TcHeaderId<TcBlockNumber, TcHeaderHash> = (TcBlockNumber, TcHeaderHash);

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
	traits::{Convert, SaturatedConversion, Saturating, Zero},
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
	// TODO: MMR type
	type TargetChain: Relayable<TcHeaderMMR = ()>;
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

		/// Target Header - ALREADY CONFIRMED
		TargetHeaderAC,

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
				BondedTcHeader<
					RingBalance<T, I>,
					TcHeaderThing<TcBlockNumber<T, I>, TcHeaderHash<T, I>, ()>
				>,
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

		/// The finalize blocks' header's id which is recorded in darwinia
		///
		/// Use for cleaning the `TcHeaders` storage
		pub ConfirmedTcHeaderIds
			get(fn confirmed_tc_header_id)
			: Vec<TcHeaderId<TcBlockNumber<T, I>, TcHeaderHash<T, I>>>;

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

		// TODO: too many db operations and calc need to move to `offchain_worker`
		fn on_finalize(block_number: BlockNumber<T>) {
			let proposals = <ClosedRounds<T, I>>::take(block_number);

			// `proposals` MUST NOT be empty after this check; qed
			if proposals.len() == 0 {
				return;
			}

			for (game_id, round) in proposals {
				let proposals = Self::proposals_of_game(game_id)
					.into_iter()
					.filter(|proposal|
						T::RelayerGameAdjustor
							::round_from_chain_len(proposal.bonded_chain.len() as _)
								== round
					)
					.collect::<Vec<_>>();
				let proposals_len = proposals.len();

				if proposals_len == 0 {
					continue;
				}

				if proposals_len == 1 {
					let proposal = proposals[0].clone();
					let mut extend_from = proposal.extend_from.clone();

					while let Some((extend_from_block_number, extend_from_header_hash))
						= extend_from.clone()
					{
						// let mut relayer = None;
						let mut evils = vec![];

						for proposal in <Proposals<T, I>>::mutate(
							extend_from_block_number,
							|proposals| proposals
								.drain_filter(|proposal|
									T::RelayerGameAdjustor::round_from_chain_len(
										proposal.bonded_chain.len() as _
									) != round
								)
								.collect::<Vec<_>>()
						) {
							// if let Some(BondedTcHeader { id, bond }) = proposal.bonded_chain.last() {
			// 					let (_, header_hash) = id;

			// 					if let Some(mut header) = <TcHeaders<T, I>>::get(id) {
			// 						if header_hash == &extend_from_header_hash {
			// 							if relayer.is_none() {
			// 								relayer = Some(proposal.relayer);
			// 							} else {
			// 								error!("[relayer-game] \
			// 									Honest Relayer MORE THAN 1 Within a Round");
			// 							}

			// 							if header.status != TcHeaderStatus::Confirmed {
			// 								header.status = TcHeaderStatus::Confirmed;

			// 								<TcHeaders<T, I>>::insert(id, header);
			// 								<ConfirmedTcHeaderIds<T, I>>::append(id);
			// 							}

			// 							extend_from = proposal.extend_from.clone();
			// 						} else {
			// 							<TcHeaders<T, I>>::take(id);

			// 							evils.push((proposal.relayer, *bond));
			// 						}
			// 					} else {
			// 						evils.push((proposal.relayer, *bond));
			// 					}
							// } else {
							// 	error!("[relayer-game] Proposal Is EMPTY");
							// }
						}

			// 			if let Some(relayer) = relayer {
			// 				for (evil, bond) in evils {
			// 					let _ = T::RingCurrency::transfer(
			// 						&evil,
			// 						&relayer,
			// 						bond,
			// 						ExistenceRequirement::KeepAlive
			// 					);

			// 					<Bonds<T, I>>::mutate(evil, |bonds|
			// 						*bonds = bond.saturating_sub(bond));
			// 				}
			// 			} else {
			// 				// Should NEVER enter this condition
			// 				for (evil, bond) in evils {
			// 					let (imbalance, _) = T::RingCurrency
			// 						::slash(&proposal.relayer, bond);
			// 					T::RingSlash::on_unbalanced(imbalance);

			// 					<Bonds<T, I>>::mutate(evil, |bonds|
			// 						*bonds = bond.saturating_sub(bond));
			// 				}

			// 				error!("[relayer-game] NO Honest Relayer");
			// 			}
					}

					// TODO: reward if no challenge

					continue;
				}

			// 	{
			// 		let chain_len = proposals[0].chain.len() as u64;
			// 		let (last_confirmed_block_number, _) = proposals[0].chain[0].id;
			// 		let (relay_target_block_number, _) = proposals[0].chain[1].id;

			// 		if chain_len
			// 			== ((relay_target_block_number
			// 				- last_confirmed_block_number).saturated_into() as u64)
			// 		{

			// 		}
			// 	}

			// 	<Samples<T, I>>::mutate(proposals[0].chain[0].id.0, |samples| {
			// 		T::RelayerGameAdjustor::update_samples(
			// 			T::RelayerGameAdjustor
			// 				::round_from_chain_len(proposals[0].chain.len() as _),
			// 			T::TargetChain::last_confirmed(),
			// 			samples
			// 		);
			// 	});
			}
		}

		// TODO:
		//	The `header_thing_chain` could be very large,
		//	the bond should relate to the bytes fee
		//	that we slash the evil relayer(s) to reward the honest relayer(s)
		// TODO: compact params?
		#[weight = 0]
		fn submit_proposal(origin, raw_header_thing_chain: Vec<RawHeaderThing>) {
			let relayer = ensure_signed(origin)?;
			let game_id = T::TargetChain
				::verify_raw_header_thing(raw_header_thing_chain[0].clone())?[0].as_block_number();
			let last_confirmed = T::TargetChain::last_confirmed();

			ensure!(game_id > last_confirmed, <Error<T, I>>::TargetHeaderAC);

			let other_proposals = Self::proposals_of_game(game_id);
			let other_proposals_len = other_proposals.len();
			let extend_bonded_chain = |chain: &[_], extend_at_round| {
				let mut total_bonds = <RingBalance<T, I>>::zero();
				let extend_chain = chain
					.iter()
					.cloned()
					.enumerate()
					.map(|(round, header_brief)| {
						let bond = T::RelayerGameAdjustor::estimate_bond(
							round as Round + extend_at_round,
							other_proposals_len as _
						);

						total_bonds = total_bonds.saturating_add(bond);

						BondedTcHeader { header_brief, bond }
					})
					.collect::<Vec<_>>();

				(total_bonds, extend_chain)
			};

			match (other_proposals_len, raw_header_thing_chain.len()) {
				// New `Game`
				(0, raw_header_thing_chain_len) => {
					ensure!(raw_header_thing_chain_len == 1, <Error<T, I>>::RoundMis);
					ensure!(
						!T::TargetChain::header_existed(game_id),
						<Error<T, I>>::TargetHeaderAE
					);

					let chain = T::TargetChain
						::verify_raw_header_thing_chain(raw_header_thing_chain)?;
					let (bonds, bonded_chain) = extend_bonded_chain(&chain, 0);

					ensure!(
						T::RingCurrency::usable_balance(&relayer) >= bonds,
						<Error<T, I>>::InsufficientValue
					);

					<Bonds<T, I>>::mutate(&relayer, |old_bonds| {
						let new_bonds = old_bonds.saturating_add(bonds);

						T::RingCurrency::set_lock(
							RELAYER_GAME_ID,
							&relayer,
							LockFor::Common { amount: new_bonds },
							WithdrawReasons::all(),
						);

						*old_bonds = new_bonds;
					});
					<Proposals<T, I>>::append(game_id, Proposal {
						relayer,
						bonded_chain,
						extend_from: None
					});
					<ClosedRounds<T, I>>::append(
						<frame_system::Module<T>>::block_number()
							+ T::RelayerGameAdjustor::challenge_time(0),
						(game_id, 0)
					);
					// Each `Proposal`'s chain's len at least is 2; qed
					<Samples<T, I>>::insert(game_id, vec![last_confirmed, game_id]);
				}
				// // First round
				(_, 1) => {
					if other_proposals.iter().any(|proposal| proposal.bonded_chain.len() != 1) {
						Err(<Error<T, I>>::RoundMis)?;
					}

					let chain = T::TargetChain
						::verify_raw_header_thing_chain(raw_header_thing_chain)?;

					ensure!(
						!other_proposals
							.into_iter()
							.any(|proposal|
								(proposal.bonded_chain.len() == chain.len())
									&& (&proposal.bonded_chain.last().unwrap().header_brief
										== chain.last().unwrap())),
						<Error<T, I>>::ProposalAE
					);

					let (bonds, bonded_chain) = extend_bonded_chain(&chain, 0);

					ensure!(
						T::RingCurrency::usable_balance(&relayer) >= bonds,
						<Error<T, I>>::InsufficientValue
					);

					<Bonds<T, I>>::mutate(&relayer, |old_bonds| {
						let new_bonds = old_bonds.saturating_add(bonds);

						T::RingCurrency::set_lock(
							RELAYER_GAME_ID,
							&relayer,
							LockFor::Common { amount: new_bonds },
							WithdrawReasons::all(),
						);

						*old_bonds = new_bonds;
					});
					<Proposals<T, I>>::append(game_id, Proposal {
						relayer,
						bonded_chain,
						extend_from: None
					});

				}
				// // Extend
				(_, raw_header_thing_chain_len) => {
					let round = T::RelayerGameAdjustor
						::round_from_chain_len(raw_header_thing_chain_len as _);
					let prev_round = round.checked_sub(1).ok_or(<Error<T, I>>::RoundMis)?;
					let chain = T::TargetChain
						::verify_raw_header_thing_chain(raw_header_thing_chain)?;
					let chain_len = chain.len();
					let samples = {
						// Chain's len is ALWAYS great than 1 under this match pattern; qed
						//
						// `HeaderBrief`'s first item MUST be block number
						// which was defined in spec; qed
						let game_id = chain[0][0].as_block_number();

						Self::samples_of_game(game_id)
					};

					ensure!(chain_len == samples.len(), <Error<T, I>>::RoundMis);
					ensure!(
						chain
							.iter()
							.zip(samples.iter())
							.all(|(header_thing, sample_block_number)|
								header_thing[0].as_block_number() == *sample_block_number),
						<Error<T, I>>::RoundMis
					);

					let extend_at = T::RelayerGameAdjustor::chain_len_from_round(prev_round) as _;
					let (bonds, extend_chain) =
						extend_bonded_chain(&chain[extend_at..], prev_round);
					let mut extend_from_proposal = None;

					for proposal in other_proposals {
						let proposal_chain_len = proposal.bonded_chain.len();

						if proposal_chain_len == extend_at {
							if chain[..extend_at]
								.iter()
								.zip(proposal.bonded_chain.iter())
								.all(|(a, b)| a == &b.header_brief)
							{
								extend_from_proposal = Some(proposal);
							}
						} else if proposal_chain_len == chain_len {
							ensure!(
								extend_chain
									.iter()
									.zip(proposal.bonded_chain[extend_at..].iter())
									.all(|(a, b)| a.header_brief == b.header_brief),
								<Error<T, I>>::ProposalAE
							);
						}
					}

					if let Some(Proposal { bonded_chain: extend_from_chain, .. }) = extend_from_proposal {
						ensure!(
							T::RingCurrency::usable_balance(&relayer) >= bonds,
							<Error<T, I>>::InsufficientValue
						);

						let extend_from_header = extend_from_chain
							.last()
							.unwrap()
							.header_brief
							.clone();
						let mut extend_chain = extend_chain;
						let mut bonded_chain = extend_from_chain;
						bonded_chain.append(&mut extend_chain);

						<Bonds<T, I>>::mutate(&relayer, |old_bonds| {
							let new_bonds = old_bonds.saturating_add(bonds);

							T::RingCurrency::set_lock(
								RELAYER_GAME_ID,
								&relayer,
								LockFor::Common { amount: new_bonds },
								WithdrawReasons::all(),
							);

							*old_bonds = new_bonds;
						});
						<Proposals<T, I>>::append(
							game_id,
							Proposal {
								relayer,
								bonded_chain,
								// Each proposal MUST contains a NOT empty chain; qed
								extend_from: Some((
									extend_from_header[0].as_block_number(),
									extend_from_header[1].as_hash()
								))
							}
						);
						{
							let closed_at = <frame_system::Module<T>>::block_number()
								+ T::RelayerGameAdjustor::challenge_time(round);

							if !Self::closed_rounds_at(closed_at).contains(&(game_id, round)) {
								<ClosedRounds<T, I>>::append(closed_at, (game_id, round));
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

#[derive(Clone, Encode, Decode, RuntimeDebug)]
pub struct Proposal<AccountId, BondedTcHeader, ProposalId> {
	// TODO: Can this proposal submit by other relayers?
	/// The relayer of these series of headers
	/// The proposer of this proposal
	/// The person who support this proposal with some bonds
	relayer: AccountId,
	/// A series of target chain's header ids and the value that relayer had bonded for it
	bonded_chain: Vec<BondedTcHeader>,
	/// Parents (previous proposal's id)
	///
	/// If this field is `None` that means this proposal is the first proposal
	extend_from: Option<ProposalId>,
}

#[derive(Clone, Encode, Decode, RuntimeDebug)]
pub struct BondedTcHeader<Balance, TcHeaderThing> {
	header_brief: Vec<TcHeaderThing>,
	bond: Balance,
}
