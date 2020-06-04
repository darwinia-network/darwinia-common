//! # Relayer Game Module
//!
//! ## Assume
//! 1. At least **one** honest relayer
//! 2. Each proposal's header hash is unique at a certain block height
//!
//!
//! ## Flow
//! 1. Request the header in *TargetChain* Module
//!    Weather the header is existed or not you should pay some fees
//! 2. If not header doesn't exist, *TargetChain* Module  will ask for a proposal here

#![cfg_attr(not(feature = "std"), no_std)]

mod types {
	// --- darwinia ---
	use crate::*;

	pub type AccountId<T> = <T as frame_system::Trait>::AccountId;
	pub type BlockNumber<T> = <T as frame_system::Trait>::BlockNumber;
	pub type RingBalance<T, I> = <RingCurrency<T, I> as Currency<AccountId<T>>>::Balance;

	pub type TCBlockNumber<T, I> = <TC<T, I> as Relayable>::BlockNumber;
	pub type TCHeaderHash<T, I> = <TC<T, I> as Relayable>::HeaderHash;

	// pub type ProposalId<HeaderNumber, HeaderHash> = (HeaderNumber, HeaderHash);
	pub type TCHeaderId<HeaderNumber, HeaderHash> = (HeaderNumber, HeaderHash);

	type RingCurrency<T, I> = <T as Trait<I>>::RingCurrency;

	type TC<T, I> = <T as Trait<I>>::TargetChain;
}

// --- crates ---
use codec::{Decode, Encode};
// --- substrate ---
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage, traits::Currency, traits::Get,
};
use frame_system as system;
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;
// --- darwinia ---
use darwinia_support::{balance::lock::*, relay::*};
use types::*;

pub trait Trait<I: Instance = DefaultInstance>: frame_system::Trait {
	type Event: From<Event<Self, I>> + Into<<Self as frame_system::Trait>::Event>;

	type RingCurrency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

	type ChallengeTime: Get<Self::BlockNumber>;

	type RelayRegulator: RelayerGameRegulator;

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
		pub Proposals
			get(fn proposal)
			: double_map
				hasher(blake2_128_concat) TCBlockNumber<T, I>,
				hasher(identity) TCHeaderHash<T, I>
			=> Proposal<
				AccountId<T>,
				BlockNumber<T>,
				RingBalance<T, I>,
				TCBlockNumber<T, I>,
				TCHeaderHash<T, I>
			>;

		pub TCHeaderPool
			get(fn tc_header)
			: double_map
				hasher(blake2_128_concat) TCBlockNumber<T, I>,
				hasher(identity) TCHeaderHash<T, I>
			=> RefTCHeader;

		pub LastConfirmed
			get(fn last_confirmed)
			: TCHeaderId<TCBlockNumber<T, I>, TCHeaderHash<T, I>>;
	}
}

decl_module! {
	pub struct Module<T: Trait<I>, I: Instance = DefaultInstance> for enum Call
	where
		origin: T::Origin
	{
		type Error = Error<T, I>;

		fn deposit_event() = default;
	}
}

#[derive(Clone, PartialEq, Encode, Decode, RuntimeDebug)]
pub enum RelayStatus {
	Confirmed,
	Unconfirmed,
}
impl Default for RelayStatus {
	fn default() -> Self {
		Self::Unconfirmed
	}
}

#[derive(Clone, Default, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct Proposal<AccountId, BlockNumber, Balance, TCBlockNumber, TCHeaderHash> {
	id: TCHeaderId<TCBlockNumber, TCHeaderHash>,
	// Will be confirmed automatically at this moment
	confirm_at: BlockNumber,
	// The person who support this proposal with some bonds
	voters: Vec<(AccountId, Balance)>,

	// If `challenge_at` is not `None`
	// That means we are in a sub-proposal or you can call this a round
	//
	// This filed could be
	// 	1. Same `TCBlockNumber` but with different `TCHeaderHash`
	// 	2. Parents or previous proposal
	challenge_at: Option<TCHeaderId<TCBlockNumber, TCHeaderHash>>,
	// This filed could be
	// 	1. Parents or previous proposal
	take_over_from: Option<TCHeaderId<TCBlockNumber, TCHeaderHash>>,
}

/// Maybe two or more proposals are using the same `Header`
/// Drop it while the `ref_count` is zero
#[derive(Clone, Default, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct RefTCHeader {
	// Codec style `Header` or `HeaderWithProofs` or ...
	// That you defined in *TargetChain* Module
	header_thing: Vec<u8>,
	ref_count: u32,
	// If this field is `Confirmed`, we can end this round immediately for all proposals
	status: RelayStatus,
}
