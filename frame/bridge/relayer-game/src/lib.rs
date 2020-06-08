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

	pub type TcBlockNumber<T, I> = <Tc<T, I> as Relayable>::BlockNumber;
	pub type TcHeaderHash<T, I> = <Tc<T, I> as Relayable>::HeaderHash;

	pub type Round = u32;

	type RingCurrency<T, I> = <T as Trait<I>>::RingCurrency;

	type Tc<T, I> = <T as Trait<I>>::TargetChain;
}

// --- crates ---
use codec::{Decode, Encode};
// --- substrate ---
use frame_support::{decl_error, decl_event, decl_module, decl_storage, traits::Currency};
use frame_system::{self as system, ensure_signed};
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;
// --- darwinia ---
use darwinia_support::{balance::lock::*, relay::*};
use types::*;

pub trait Trait<I: Instance = DefaultInstance>: frame_system::Trait {
	type Event: From<Event<Self, I>> + Into<<Self as frame_system::Trait>::Event>;

	// The currency use for bond
	type RingCurrency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

	// A regulator to adjust relay args for a specific chain
	type RelayerGameAdjustment: RelayerGameAdjustable;

	// The target chain's relay module's API
	type TargetChain: Relayable;
}

decl_event! {
	pub enum Event<T, I: Instance = DefaultInstance>
	where
		AccountId = AccountId<T>,
	{
		/// TODO
		TODO(AccountId),
	}
}

decl_error! {
	pub enum Error for Module<T: Trait<I>, I: Instance> {
	}
}

decl_storage! {
	trait Store for Module<T: Trait<I>, I: Instance = DefaultInstance> as DarwiniaRelayerGame {
		// Each `TcHeaderId` is a `Proposal`
		// A `Proposal` can spawn many sub-proposals
		pub Proposals
			get(fn proposal)
			: double_map
				hasher(blake2_128_concat) TcBlockNumber<T, I>,
				hasher(identity) TcHeaderHash<T, I>
			=> Proposal<
				AccountId<T>,
				BlockNumber<T>,
				RingBalance<T, I>,
				TcBlockNumber<T, I>,
				TcHeaderHash<T, I>
			>;

		// All the `TcHeader`s store here, **NON-DUPLICATIVE**
		pub TcHeaders
			get(fn tc_header)
			: double_map
				hasher(blake2_128_concat) TcBlockNumber<T, I>,
				hasher(identity) TcHeaderHash<T, I>
			=> RefTcHeader;

		pub ChallengeTimes
			get(fn challenge_time)
			: double_map
				hasher(blake2_128_concat) Round,
				hasher(blake2_128_concat) TcBlockNumber<T, I>
			=> T::BlockNumber;

		// The finalize blocks' header's record id in darwinia
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

		#[weight = 0]
		fn submit(origin, tc_header_thing: Vec<u8>) {
			let relayer = ensure_signed(origin)?;
			let tc_header_id = T::TargetChain::verify(&tc_header_thing)?;

			// TcHeaders::insert(tc_header_id.0, tc_header_id.1, tc_header_thing);
		}
	}
}

#[derive(Clone, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum TcHeaderStatus {
	// The header had been confirmed by game
	Confirmed,
	// The header had been confirmed by game but too old
	// Means we might not use this header anymore so drop it to free the storage
	Outdated,
	// The header has not been judged yet
	Unknown,
	// The header is invalid
	Invalid,
}
impl Default for TcHeaderStatus {
	fn default() -> Self {
		Self::Unknown
	}
}

#[derive(Clone, Default, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct Proposal<AccountId, BlockNumber, Balance, TcBlockNumber, TcHeaderHash> {
	// Current target chain's header id
	id: TcHeaderId<TcBlockNumber, TcHeaderHash>,
	// Will be confirmed automatically at this moment
	confirm_at: BlockNumber,
	// The person who support this proposal with some bonds
	nominators: Vec<(AccountId, Balance)>,
	// Parents (previous proposal)
	//
	// If this field is `None` that means this proposal is the main proposal
	// which is the head of a proposal link list
	extend_from: Option<TcHeaderId<TcBlockNumber, TcHeaderHash>>,
}

#[derive(Clone, Default, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct RefTcHeader {
	// Codec style `Header` or `HeaderWithProofs` or ...
	// That you defined in target chain's relay module use for verifying
	header_thing: Vec<u8>,
	// Maybe two or more proposals are using the same `Header`
	// Drop it while the `ref_count` is zero but **NOT** in `ConfirmedTcHeaders` list
	ref_count: u32,
	// Help chain to end a round quickly
	status: TcHeaderStatus,
}
