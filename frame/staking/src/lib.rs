// This file is part of Darwinia.
//
// Copyright (C) 2018-2021 Darwinia Network
// SPDX-License-Identifier: GPL-3.0
//
// Darwinia is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Darwinia is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

//! # Staking Pallet
//!
//! The Staking pallet is used to manage funds at stake by network maintainers.
//!
//! - [`Config`]
//! - [`Call`]
//! - [`Pallet`]
//!
//! ## Overview
//!
//! The Staking pallet is the means by which a set of network maintainers (known as _authorities_ in
//! some contexts and _validators_ in others) are chosen based upon those who voluntarily place
//! funds under deposit. Under deposit, those funds are rewarded under normal operation but are held
//! at pain of _slash_ (expropriation) should the staked maintainer be found not to be discharging
//! its duties properly.
//!
//! ### Terminology
//! <!-- Original author of paragraph: @gavofyork -->
//!
//! - Staking: The process of locking up funds for some time, placing them at risk of slashing
//!   (loss) in order to become a rewarded maintainer of the network.
//! - Validating: The process of running a node to actively maintain the network, either by
//!   producing blocks or guaranteeing finality of the chain.
//! - Nominating: The process of placing staked funds behind one or more validators in order to
//!   share in any reward, and punishment, they take.
//! - Stash account: The account holding an owner's funds used for staking.
//! - Controller account: The account that controls an owner's funds for staking.
//! - Era: A (whole) number of sessions, which is the period that the validator set (and each
//!   validator's active nominator set) is recalculated and where rewards are paid out.
//! - Slash: The punishment of a staker by reducing its funds.
//!
//! ### Goals
//! <!-- Original author of paragraph: @gavofyork -->
//!
//! The staking system in Darwinia NPoS is designed to make the following possible:
//!
//! - Stake funds that are controlled by a cold wallet.
//! - Withdraw some, or deposit more, funds without interrupting the role of an entity.
//! - Switch between roles (nominator, validator, idle) with minimal overhead.
//!
//! ### Scenarios
//!
//! #### Staking
//!
//! Almost any interaction with the Staking pallet requires a process of _**bonding**_ (also known
//! as being a _staker_). To become *bonded*, a fund-holding account known as the _stash account_,
//! which holds some or all of the funds that become frozen in place as part of the staking process,
//! is paired with an active **controller** account, which issues instructions on how they shall be
//! used.
//!
//! An account pair can become bonded using the [`bond`](Call::bond) call.
//!
//! Stash accounts can change their associated controller using the
//! [`set_controller`](Call::set_controller) call.
//!
//! There are three possible roles that any staked account pair can be in: `Validator`, `Nominator`
//! and `Idle` (defined in [`StakerStatus`]). There are three
//! corresponding instructions to change between roles, namely:
//! [`validate`](Call::validate),
//! [`nominate`](Call::nominate), and [`chill`](Call::chill).
//!
//! #### Validating
//!
//! A **validator** takes the role of either validating blocks or ensuring their finality,
//! maintaining the veracity of the network. A validator should avoid both any sort of malicious
//! misbehavior and going offline. Bonded accounts that state interest in being a validator do NOT
//! get immediately chosen as a validator. Instead, they are declared as a _candidate_ and they
//! _might_ get elected at the _next era_ as a validator. The result of the election is determined
//! by nominators and their votes.
//!
//! An account can become a validator candidate via the
//! [`validate`](Call::validate) call.
//!
//! #### Nomination
//!
//! A **nominator** does not take any _direct_ role in maintaining the network, instead, it votes on
//! a set of validators  to be elected. Once interest in nomination is stated by an account, it
//! takes effect at the next election round. The funds in the nominator's stash account indicate the
//! _weight_ of its vote. Both the rewards and any punishment that a validator earns are shared
//! between the validator and its nominators. This rule incentivizes the nominators to NOT vote for
//! the misbehaving/offline validators as much as possible, simply because the nominators will also
//! lose funds if they vote poorly.
//!
//! An account can become a nominator via the [`nominate`](Call::nominate) call.
//!
//! #### Rewards and Slash
//!
//! The **reward and slashing** procedure is the core of the Staking pallet, attempting to _embrace
//! valid behavior_ while _punishing any misbehavior or lack of availability_.
//!
//! `payout_stakers` call. Any account can call `payout_stakers`, which pays the reward to the
//! validator as well as its nominators. Only the [`Config::MaxNominatorRewardedPerValidator`]
//! biggest stakers can claim their reward. This is to limit the i/o cost to mutate storage for each
//! nominator's account.
//!
//! Slashing can occur at any point in time, once misbehavior is reported. Once slashing is
//! determined, a value is deducted from the balance of the validator and all the nominators who
//! voted for this validator (values are deducted from the _stash_ account of the slashed entity).
//!
//! Slashing logic is further described in the documentation of the `slashing` pallet.
//!
//! Similar to slashing, rewards are also shared among a validator and its associated nominators.
//! Yet, the reward funds are not always transferred to the stash account and can be configured. See
//! [Reward Calculation](#reward-calculation) for more details.
//!
//! #### Chilling
//!
//! Finally, any of the roles above can choose to step back temporarily and just chill for a while.
//! This means that if they are a nominator, they will not be considered as voters anymore and if
//! they are validators, they will no longer be a candidate for the next election.
//!
//! An account can step back via the [`chill`](Call::chill) call.
//!
//! ### Session managing
//!
//! The pallet implement the trait `SessionManager`. Which is the only API to query new validator
//! set and allowing these validator set to be rewarded once their era is ended.
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! The dispatchable functions of the Staking pallet enable the steps needed for entities to accept
//! and change their role, alongside some helper functions to get/set the metadata of the pallet.
//!
//! ### Public Functions
//!
//! The Staking pallet contains many public storage items and (im)mutable functions.
//!
//! ## Usage
//!
//! ### Example: Rewarding a validator by id.
//!
//! ```
//! use frame_support::{decl_module, dispatch};
//! use frame_system::ensure_signed;
//! use darwinia_staking as staking;
//!
//! pub trait Config: staking::Config {}
//!
//! decl_module! {
//! 	pub struct Module<T: Config> for enum Call where origin: T::Origin {
//! 		/// Reward a validator.
//! 		#[weight = 0]
//! 		pub fn reward_myself(origin) -> dispatch::DispatchResult {
//! 			let reported = ensure_signed(origin)?;
//! 			<staking::Pallet<T>>::reward_by_ids(vec![(reported, 10)]);
//! 			Ok(())
//! 		}
//! 	}
//! }
//! # fn main() {}
//! ```
//!
//! ## Implementation Details
//!
//! ### Era payout
//!
//! The era payout is computed using yearly inflation curve defined at
//! [`Config::EraPayout`] as such:
//!
//! ```nocompile
//! staker_payout = yearly_inflation(npos_token_staked / total_tokens) * total_tokens / era_per_year
//! ```
//! This payout is used to reward stakers as defined in next section
//!
//! ```nocompile
//! remaining_payout = max_yearly_inflation * total_tokens / era_per_year - staker_payout
//! ```
//! The remaining reward is send to the configurable end-point
//! [`Config::RewardRemainder`].
//!
//! ### Reward Calculation
//!
//! Validators and nominators are rewarded at the end of each era. The total reward of an era is
//! calculated using the era duration and the staking rate (the total amount of tokens staked by
//! nominators and validators, divided by the total token supply). It aims to incentivize toward a
//! defined staking rate. The full specification can be found
//! [here](https://research.web3.foundation/en/latest/polkadot/Token%20Economics.html#inflation-model).
//!
//! Total reward is split among validators and their nominators depending on the number of points
//! they received during the era. Points are added to a validator using
//! [`reward_by_ids`](Pallet::reward_by_ids).
//!
//! [`Pallet`] implements
//! [`pallet_authorship::EventHandler`] to add reward
//! points to block producer and block producer of referenced uncles.
//!
//! The validator and its nominator split their reward as following:
//!
//! The validator can declare an amount, named
//! [`commission`](ValidatorPrefs::commission), that does not get shared
//! with the nominators at each reward payout through its
//! [`ValidatorPrefs`]. This value gets deducted from the total reward
//! that is paid to the validator and its nominators. The remaining portion is split among the
//! validator and all of the nominators that nominated the validator, proportional to the value
//! staked behind this validator (_i.e._ dividing the
//! [`own`](Exposure::own) or
//! [`others`](Exposure::others) by
//! [`total`](Exposure::total) in [`Exposure`]).
//!
//! All entities who receive a reward have the option to choose their reward destination through the

//! [`Payee`] storage item (see
//! [`set_payee`](Call::set_payee)), to be one of the following:
//!
//! - Controller account, (obviously) not increasing the staked value.
//! - Stash account, not increasing the staked value.
//! - Stash account, also increasing the staked value.
//!
//! ### Additional Fund Management Operations
//!
//! Any funds already placed into stash can be the target of the following operations:
//!
//! The controller account can free a portion (or all) of the funds using the
//! [`unbond`](Call::unbond) call. Note that the funds are not immediately
//! accessible. Instead, a duration denoted by
//! [`Config::BondingDuration`] (in number of eras) must
//! pass until the funds can actually be removed.
//!
//! Note that there is a limitation to the number of fund-chunks that can be scheduled to be
//! unlocked in the future via [`unbond`](Call::unbond). In case this maximum
//! (`MAX_UNLOCKING_CHUNKS`) is reached, the bonded account _must_ first wait until a successful
//! call to `withdraw_unbonded` to remove some of the chunks.
//!
//! ### Election Algorithm
//!
//! The current election algorithm is implemented based on Phragmén. The reference implementation
//! can be found [here](https://github.com/w3f/consensus/tree/master/NPoS).
//!
//! The election algorithm, aside from electing the validators with the most stake value and votes,
//! tries to divide the nominator votes among candidates in an equal manner. To further assure this,
//! an optional post-processing can be applied that iteratively normalizes the nominator staked
//! values until the total difference among votes of a particular nominator are less than a
//! threshold.
//!
//! ## GenesisConfig
//!
//! The Staking pallet depends on the [`GenesisConfig`]. The
//! `GenesisConfig` is optional and allow to set some initial stakers.
//!
//! ## Related Modules
//!
//! - [Balances](../darwinia_balances/index.html): Used to manage values at stake.
//! - [Session](../pallet_session/index.html): Used to manage sessions. Also, a list of new
//!   validators is stored in the Session pallet's `Validators` at the end of each era.

#![cfg_attr(not(feature = "std"), no_std)]
#![feature(drain_filter)]

// syntactic sugar for logging.
#[macro_export]
macro_rules! log {
	($level:tt, $patter:expr $(, $values:expr)* $(,)?) => {
		frame_support::log::$level!(
			target: crate::LOG_TARGET,
			concat!("[{:?}] 💸 ", $patter), <frame_system::Pallet<T>>::block_number() $(, $values)*
		)
	};
}

#[cfg(test)]
mod darwinia_tests;
#[cfg(test)]
mod inflation_tests;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod substrate_tests;
#[cfg(test)]
mod testing_utils;

pub mod weights;
pub use weights::WeightInfo;

pub mod inflation;
pub mod slashing;

pub mod constants {
	// --- paritytech ---
	use frame_support::traits::LockIdentifier;
	// --- darwinia-network ---
	use crate::*;

	pub const LOG_TARGET: &'static str = "runtime::staking";

	pub const STAKING_ID: LockIdentifier = *b"da/staki";

	// TODO: Limited in frame/support/src/lib.rs `StakingLock`
	pub const MAX_UNLOCKING_CHUNKS: usize = 32;

	pub const MONTH_IN_MINUTES: TsInMs = 30 * 24 * 60;
	pub const MONTH_IN_MILLISECONDS: TsInMs = MONTH_IN_MINUTES * 60 * 1000;
}
pub use constants::*;

pub mod types {
	// --- paritytech ---
	use frame_support::traits::Currency;
	use frame_system::pallet_prelude::*;
	// --- darwinia-network ---
	use crate::*;

	/// Counter for the number of eras that have passed.
	pub type EraIndex = u32;
	/// Counter for the number of "reward" points earned by a given validator.
	pub type RewardPoint = u32;

	/// Balance of an account.
	pub type Balance = u128;
	/// Power of an account.
	pub type Power = u32;
	/// A timestamp: milliseconds since the unix epoch.
	/// `u64` is enough to represent a duration of half a billion years, when the
	/// time scale is milliseconds.
	pub type TsInMs = u64;

	pub type StakingLedgerT<T> =
		StakingLedger<AccountId<T>, RingBalance<T>, KtonBalance<T>, BlockNumberFor<T>>;
	pub type StakingBalanceT<T> = StakingBalance<RingBalance<T>, KtonBalance<T>>;
	pub type ExposureT<T> = Exposure<AccountId<T>, RingBalance<T>, KtonBalance<T>>;

	pub type AccountId<T> = <T as frame_system::Config>::AccountId;

	pub type RingBalance<T> = <RingCurrency<T> as Currency<AccountId<T>>>::Balance;
	pub type RingPositiveImbalance<T> =
		<RingCurrency<T> as Currency<AccountId<T>>>::PositiveImbalance;
	pub type RingNegativeImbalance<T> =
		<RingCurrency<T> as Currency<AccountId<T>>>::NegativeImbalance;

	pub type KtonBalance<T> = <KtonCurrency<T> as Currency<AccountId<T>>>::Balance;
	pub type KtonPositiveImbalance<T> =
		<KtonCurrency<T> as Currency<AccountId<T>>>::PositiveImbalance;
	pub type KtonNegativeImbalance<T> =
		<KtonCurrency<T> as Currency<AccountId<T>>>::NegativeImbalance;

	type RingCurrency<T> = <T as Config>::RingCurrency;
	type KtonCurrency<T> = <T as Config>::KtonCurrency;
}
pub use types::*;

#[frame_support::pallet]
pub mod pallet {
	// --- core ---
	use core::{convert::TryInto, mem};
	// --- crates.io ---
	use codec::{Decode, Encode, HasCompact};
	#[cfg(feature = "std")]
	use serde::{Deserialize, Serialize};
	// --- paritytech ---
	use frame_election_provider_support::{
		data_provider, ElectionDataProvider, ElectionProvider, Supports, VoteWeight,
	};
	use frame_support::{
		dispatch::WithPostDispatchInfo,
		pallet_prelude::*,
		traits::{
			Currency, EstimateNextNewSession, ExistenceRequirement::KeepAlive, Imbalance,
			OnUnbalanced, UnixTime, WithdrawReasons,
		},
		weights::constants::{WEIGHT_PER_MICROS, WEIGHT_PER_NANOS},
		PalletId, WeakBoundedVec,
	};
	use frame_system::{offchain::SendTransactionTypes, pallet_prelude::*};
	use sp_runtime::{
		helpers_128bit,
		traits::{
			AccountIdConversion, AtLeast32BitUnsigned, Bounded, CheckedSub, Convert, Saturating,
			StaticLookup, Zero,
		},
		Perbill, Percent, Perquintill, SaturatedConversion,
	};
	use sp_staking::{
		offence::{Offence, OffenceDetails, OffenceError, OnOffenceHandler, ReportOffence},
		SessionIndex,
	};
	use sp_std::{borrow::ToOwned, collections::btree_map::BTreeMap, prelude::*};
	// --- darwinia-network ---
	use crate::*;
	use darwinia_staking_rpc_runtime_api::RuntimeDispatchInfo;
	use darwinia_support::{balance::*, traits::OnDepositRedeem};

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + SendTransactionTypes<Call<Self>> {
		/// Maximum number of nominations per nominator.
		const MAX_NOMINATIONS: u32;

		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type PalletId: Get<PalletId>;

		/// Time used for computing era duration.
		///
		/// It is guaranteed to start being called from the first `on_finalize`. Thus value at genesis
		/// is not used.
		type UnixTime: UnixTime;

		/// Something that provides the election functionality.
		type ElectionProvider: ElectionProvider<
			Self::AccountId,
			Self::BlockNumber,
			// we only accept an election provider that has staking as data provider.
			DataProvider = Pallet<Self>,
		>;

		/// Something that provides the election functionality at genesis.
		type GenesisElectionProvider: ElectionProvider<
			Self::AccountId,
			Self::BlockNumber,
			DataProvider = Pallet<Self>,
		>;

		/// Number of sessions per era.
		#[pallet::constant]
		type SessionsPerEra: Get<SessionIndex>;
		/// Interface for interacting with a session pallet.
		type SessionInterface: self::SessionInterface<Self::AccountId>;
		/// Something that can estimate the next session change, accurately or as a best effort guess.
		type NextNewSession: EstimateNextNewSession<Self::BlockNumber>;

		/// Number of eras that slashes are deferred by, after computation.
		///
		/// This should be less than the bonding duration. Set to 0 if slashes
		/// should be applied immediately, without opportunity for intervention.
		#[pallet::constant]
		type SlashDeferDuration: Get<EraIndex>;
		/// The origin which can cancel a deferred slash. Root can always do this.
		type SlashCancelOrigin: EnsureOrigin<Self::Origin>;

		/// The maximum number of nominators rewarded for each validator.
		///
		/// For each validator only the `$MaxNominatorRewardedPerValidator` biggest stakers can claim
		/// their reward. This used to limit the i/o cost for the nominator payout.
		#[pallet::constant]
		type MaxNominatorRewardedPerValidator: Get<u32>;

		/// Number of eras that staked funds must remain bonded for.
		#[pallet::constant]
		type BondingDurationInEra: Get<EraIndex>;
		/// Number of eras that staked funds must remain bonded for.
		#[pallet::constant]
		type BondingDurationInBlockNumber: Get<Self::BlockNumber>;

		/// The *RING* currency.
		type RingCurrency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;
		/// Tokens have been minted and are unused for validator-reward.
		/// See [Era payout](./index.html#era-payout).
		type RingRewardRemainder: OnUnbalanced<RingNegativeImbalance<Self>>;
		/// Handler for the unbalanced *RING* reduction when slashing a staker.
		type RingSlash: OnUnbalanced<RingNegativeImbalance<Self>>;
		/// Handler for the unbalanced *RING* increment when rewarding a staker.
		type RingReward: OnUnbalanced<RingPositiveImbalance<Self>>;

		/// The *KTON* currency.
		type KtonCurrency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;
		/// Handler for the unbalanced *KTON* reduction when slashing a staker.
		type KtonSlash: OnUnbalanced<KtonNegativeImbalance<Self>>;
		/// Handler for the unbalanced *KTON* increment when rewarding a staker.
		type KtonReward: OnUnbalanced<KtonPositiveImbalance<Self>>;

		/// Darwinia's hard cap default `10_000_000_000 * 10^9`
		#[pallet::constant]
		type Cap: Get<RingBalance<Self>>;
		/// Darwinia's staking vote default `1_000_000_000`
		#[pallet::constant]
		type TotalPower: Get<Power>;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	#[pallet::metadata(
		AccountId<T> = "AccountId",
		BlockNumberFor<T> = "BlockNumber",
		RingBalance<T> = "RingBalance",
		KtonBalance<T> = "KtonBalance",
	)]
	pub enum Event<T: Config> {
		/// The era payout has been set; the first balance is the validator-payout; the second is
		/// the remainder from the maximum amount of reward.
		/// \[era_index, validator_payout, remainder\]
		EraPayout(EraIndex, RingBalance<T>, RingBalance<T>),

		/// The staker has been rewarded by this amount. \[stash, amount\]
		Reward(AccountId<T>, RingBalance<T>),

		/// One validator (and its nominators) has been slashed by the given amount.
		/// \[validator, amount, amount\]
		Slash(AccountId<T>, RingBalance<T>, KtonBalance<T>),
		/// An old slashing report from a prior era was discarded because it could
		/// not be processed. \[session_index\]
		OldSlashingReportDiscarded(SessionIndex),

		/// A new set of stakers was elected.
		StakingElection,

		/// An account has bonded this amount. \[amount, start, end\]
		///
		/// NOTE: This event is only emitted when funds are bonded via a dispatchable. Notably,
		/// it will not be emitted for staking rewards when they are added to stake.
		BondRing(RingBalance<T>, TsInMs, TsInMs),
		/// An account has bonded this amount. \[amount, start, end\]
		///
		/// NOTE: This event is only emitted when funds are bonded via a dispatchable. Notably,
		/// it will not be emitted for staking rewards when they are added to stake.
		BondKton(KtonBalance<T>),

		/// An account has unbonded this amount. \[amount, now\]
		UnbondRing(RingBalance<T>, BlockNumberFor<T>),
		/// An account has unbonded this amount. [amount, now\]
		UnbondKton(KtonBalance<T>, BlockNumberFor<T>),

		/// A nominator has been kicked from a validator. \[nominator, stash\]
		Kicked(AccountId<T>, AccountId<T>),

		/// The election failed. No new era is planned.
		StakingElectionFailed,

		/// An account has stopped participating as either a validator or nominator.
		/// \[stash\]
		Chilled(T::AccountId),

		/// Someone claimed his deposits. \[stash\]
		DepositsClaimed(AccountId<T>),
		/// Someone claimed his deposits with some *KTON*s punishment. \[stash, forfeit\]
		DepositsClaimedWithPunish(AccountId<T>, KtonBalance<T>),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Not a controller account.
		NotController,
		/// Not a stash account.
		NotStash,
		/// Stash is already bonded.
		AlreadyBonded,
		/// Controller is already paired.
		AlreadyPaired,
		/// Targets cannot be empty.
		EmptyTargets,
		/// Duplicate index.
		DuplicateIndex,
		/// Slash record index out of bounds.
		InvalidSlashIndex,
		/// Can not bond with value less than minimum required.
		InsufficientBond,
		/// Can not schedule more unlock chunks.
		NoMoreChunks,
		/// Can not rebond without unlocking chunks.
		NoUnlockChunk,
		/// Attempting to target a stash that still has funds.
		FundedTarget,
		/// Invalid era to reward.
		InvalidEraToReward,
		/// Invalid number of nominations.
		InvalidNumberOfNominations,
		/// Items are not sorted and unique.
		NotSortedAndUnique,
		/// Rewards for this era have already been claimed for this validator.
		AlreadyClaimed,
		/// Incorrect previous history depth input provided.
		IncorrectHistoryDepth,
		/// Incorrect number of slashing spans provided.
		IncorrectSlashingSpans,
		/// Internal state has become somehow corrupted and the operation cannot continue.
		BadState,
		/// Too many nomination targets supplied.
		TooManyTargets,
		/// A nomination target was supplied that was blocked or otherwise not a validator.
		BadTarget,
		/// The user has enough bond and thus cannot be chilled forcefully by an external person.
		CannotChillOther,
		/// There are too many nominators in the system. Governance needs to adjust the staking settings
		/// to keep things safe for the runtime.
		TooManyNominators,
		/// There are too many validators in the system. Governance needs to adjust the staking settings
		/// to keep things safe for the runtime.
		TooManyValidators,
		/// Payout - INSUFFICIENT
		PayoutIns,
	}

	#[pallet::extra_constants]
	impl<T: Config> Pallet<T> {
		//TODO: rename to snake case after https://github.com/paritytech/substrate/issues/8826 fixed.
		#[allow(non_snake_case)]
		fn MaxNominations() -> u32 {
			T::MAX_NOMINATIONS
		}
	}

	/// Number of eras to keep in history.
	///
	/// Information is kept for eras in `[current_era - history_depth; current_era]`.
	///
	/// Must be more than the number of eras delayed by session otherwise. I.e. active era must
	/// always be in history. I.e. `active_era > current_era - history_depth` must be
	/// guaranteed.
	#[pallet::storage]
	#[pallet::getter(fn history_depth)]
	pub type HistoryDepth<T> = StorageValue<_, u32, ValueQuery, HistoryDepthOnEmpty>;
	#[pallet::type_value]
	pub fn HistoryDepthOnEmpty() -> u32 {
		336
	}

	/// The ideal number of staking participants.
	#[pallet::storage]
	#[pallet::getter(fn validator_count)]
	pub type ValidatorCount<T> = StorageValue<_, u32, ValueQuery>;

	/// Minimum number of staking participants before emergency conditions are imposed.
	#[pallet::storage]
	#[pallet::getter(fn minimum_validator_count)]
	pub type MinimumValidatorCount<T> = StorageValue<_, u32, ValueQuery>;

	/// Any validators that may never be slashed or forcibly kicked. It's a Vec since they're
	/// easy to initialize and the performance hit is minimal (we expect no more than four
	/// invulnerables) and restricted to testnets.
	#[pallet::storage]
	#[pallet::getter(fn invulnerables)]
	pub type Invulnerables<T: Config> = StorageValue<_, Vec<AccountId<T>>, ValueQuery>;

	/// Map from all locked "stash" accounts to the controller account.
	#[pallet::storage]
	#[pallet::getter(fn bonded)]
	pub type Bonded<T: Config> = StorageMap<_, Twox64Concat, AccountId<T>, AccountId<T>>;

	/// The minimum active bond to become and maintain the role of a nominator.
	#[pallet::storage]
	pub type MinNominatorBond<T: Config> = StorageValue<_, RingBalance<T>, ValueQuery>;

	/// The minimum active bond to become and maintain the role of a validator.
	#[pallet::storage]
	pub type MinValidatorBond<T: Config> = StorageValue<_, RingBalance<T>, ValueQuery>;

	/// Map from all (unlocked) "controller" accounts to the info regarding the staking.
	#[pallet::storage]
	#[pallet::getter(fn ledger)]
	pub type Ledger<T: Config> = StorageMap<_, Blake2_128Concat, AccountId<T>, StakingLedgerT<T>>;

	/// Where the reward payment should be made. Keyed by stash.
	#[pallet::storage]
	#[pallet::getter(fn payee)]
	pub type Payee<T: Config> =
		StorageMap<_, Twox64Concat, AccountId<T>, RewardDestination<AccountId<T>>, ValueQuery>;

	/// The map from (wannabe) validator stash key to the preferences of that validator.
	///
	/// When updating this storage item, you must also update the `CounterForValidators`.
	#[pallet::storage]
	#[pallet::getter(fn validators)]
	pub type Validators<T: Config> =
		StorageMap<_, Twox64Concat, AccountId<T>, ValidatorPrefs, ValueQuery>;

	/// A tracker to keep count of the number of items in the `Validators` map.
	#[pallet::storage]
	pub type CounterForValidators<T> = StorageValue<_, u32, ValueQuery>;

	/// The maximum validator count before we stop allowing new validators to join.
	///
	/// When this value is not set, no limits are enforced.
	#[pallet::storage]
	pub type MaxValidatorsCount<T> = StorageValue<_, u32, OptionQuery>;

	/// The map from nominator stash key to the set of stash keys of all validators to nominate.
	///
	/// When updating this storage item, you must also update the `CounterForNominators`.
	#[pallet::storage]
	#[pallet::getter(fn nominators)]
	pub type Nominators<T: Config> =
		StorageMap<_, Twox64Concat, AccountId<T>, Nominations<AccountId<T>>>;

	/// A tracker to keep count of the number of items in the `Nominators` map.
	#[pallet::storage]
	pub type CounterForNominators<T> = StorageValue<_, u32, ValueQuery>;

	/// The maximum nominator count before we stop allowing new validators to join.
	///
	/// When this value is not set, no limits are enforced.
	#[pallet::storage]
	pub type MaxNominatorsCount<T> = StorageValue<_, u32, OptionQuery>;

	/// The current era index.
	///
	/// This is the latest planned era, depending on how the Session pallet queues the validator
	/// set, it might be active or not.
	#[pallet::storage]
	#[pallet::getter(fn current_era)]
	pub type CurrentEra<T> = StorageValue<_, EraIndex>;

	/// The active era information, it holds index and start.
	///
	/// The active era is the era being currently rewarded. Validator set of this era must be
	/// equal to [`SessionInterface::validators`].
	#[pallet::storage]
	#[pallet::getter(fn active_era)]
	pub type ActiveEra<T> = StorageValue<_, ActiveEraInfo>;

	/// The session index at which the era start for the last `HISTORY_DEPTH` eras.
	///
	/// Note: This tracks the starting session (i.e. session index when era start being active)
	/// for the eras in `[CurrentEra - HISTORY_DEPTH, CurrentEra]`.
	#[pallet::storage]
	#[pallet::getter(fn eras_start_session_index)]
	pub type ErasStartSessionIndex<T> = StorageMap<_, Twox64Concat, EraIndex, SessionIndex>;

	/// Exposure of validator at era.
	///
	/// This is keyed first by the era index to allow bulk deletion and then the stash account.
	///
	/// Is it removed after `HISTORY_DEPTH` eras.
	/// If stakers hasn't been set or has been removed then empty exposure is returned.
	#[pallet::storage]
	#[pallet::getter(fn eras_stakers)]
	pub type ErasStakers<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		EraIndex,
		Twox64Concat,
		AccountId<T>,
		ExposureT<T>,
		ValueQuery,
	>;

	/// Clipped Exposure of validator at era.
	///
	/// This is similar to [`ErasStakers`] but number of nominators exposed is reduced to the
	/// `T::MaxNominatorRewardedPerValidator` biggest stakers.
	/// (Note: the field `total` and `own` of the exposure remains unchanged).
	/// This is used to limit the i/o cost for the nominator payout.
	///
	/// This is keyed fist by the era index to allow bulk deletion and then the stash account.
	///
	/// Is it removed after `HISTORY_DEPTH` eras.
	/// If stakers hasn't been set or has been removed then empty exposure is returned.
	#[pallet::storage]
	#[pallet::getter(fn eras_stakers_clipped)]
	pub type ErasStakersClipped<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		EraIndex,
		Twox64Concat,
		AccountId<T>,
		ExposureT<T>,
		ValueQuery,
	>;

	/// Similar to `ErasStakers`, this holds the preferences of validators.
	///
	/// This is keyed first by the era index to allow bulk deletion and then the stash account.
	///
	/// Is it removed after `HISTORY_DEPTH` eras.
	// If prefs hasn't been set or has been removed then 0 commission is returned.
	#[pallet::storage]
	#[pallet::getter(fn eras_validator_prefs)]
	pub type ErasValidatorPrefs<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		EraIndex,
		Twox64Concat,
		AccountId<T>,
		ValidatorPrefs,
		ValueQuery,
	>;

	/// The total validator era payout for the last `HISTORY_DEPTH` eras.
	///
	/// Eras that haven't finished yet or has been removed doesn't have reward.
	#[pallet::storage]
	#[pallet::getter(fn eras_validator_reward)]
	pub type ErasValidatorReward<T: Config> = StorageMap<_, Twox64Concat, EraIndex, RingBalance<T>>;

	/// Rewards for the last `HISTORY_DEPTH` eras.
	/// If reward hasn't been set or has been removed then 0 reward is returned.
	#[pallet::storage]
	#[pallet::getter(fn eras_reward_points)]
	pub type ErasRewardPoints<T: Config> =
		StorageMap<_, Twox64Concat, EraIndex, EraRewardPoints<AccountId<T>>, ValueQuery>;

	/// The total amount staked for the last `HISTORY_DEPTH` eras.
	/// If total hasn't been set or has been removed then 0 stake is returned.
	#[pallet::storage]
	#[pallet::getter(fn eras_total_stake)]
	pub type ErasTotalStake<T: Config> = StorageMap<_, Twox64Concat, EraIndex, Power, ValueQuery>;

	/// Mode of era forcing.
	#[pallet::storage]
	#[pallet::getter(fn force_era)]
	pub type ForceEra<T> = StorageValue<_, Forcing, ValueQuery>;

	/// The percentage of the slash that is distributed to reporters.
	///
	/// The rest of the slashed value is handled by the `Slash`.
	#[pallet::storage]
	#[pallet::getter(fn slash_reward_fraction)]
	pub type SlashRewardFraction<T> = StorageValue<_, Perbill, ValueQuery>;

	/// The amount of currency given to reporters of a slash event which was
	/// canceled by extraordinary circumstances (e.g. governance).
	#[pallet::storage]
	#[pallet::getter(fn canceled_payout)]
	pub type CanceledSlashPayout<T: Config> = StorageValue<_, Power, ValueQuery>;

	/// All unapplied slashes that are queued for later.
	#[pallet::storage]
	pub type UnappliedSlashes<T: Config> = StorageMap<
		_,
		Twox64Concat,
		EraIndex,
		Vec<UnappliedSlash<AccountId<T>, RingBalance<T>, KtonBalance<T>>>,
		ValueQuery,
	>;

	/// A mapping from still-bonded eras to the first session index of that era.
	///
	/// Must contains information for eras for the range:
	/// `[active_era - bounding_duration; active_era]`
	#[pallet::storage]
	pub type BondedEras<T: Config> = StorageValue<_, Vec<(EraIndex, SessionIndex)>, ValueQuery>;

	/// All slashing events on validators, mapped by era to the highest slash proportion
	/// and slash value of the era.
	#[pallet::storage]
	pub type ValidatorSlashInEra<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		EraIndex,
		Twox64Concat,
		AccountId<T>,
		(Perbill, slashing::RKT<T>),
	>;

	/// All slashing events on nominators, mapped by era to the highest slash value of the era.
	#[pallet::storage]
	pub type NominatorSlashInEra<T: Config> =
		StorageDoubleMap<_, Twox64Concat, EraIndex, Twox64Concat, AccountId<T>, slashing::RKT<T>>;

	/// Slashing spans for stash accounts.
	#[pallet::storage]
	pub type SlashingSpans<T: Config> =
		StorageMap<_, Twox64Concat, AccountId<T>, slashing::SlashingSpans>;

	/// Records information about the maximum slash of a stash within a slashing span,
	/// as well as how much reward has been paid out.
	#[pallet::storage]
	pub type SpanSlash<T: Config> = StorageMap<
		_,
		Twox64Concat,
		(AccountId<T>, slashing::SpanIndex),
		slashing::SpanRecord<RingBalance<T>, KtonBalance<T>>,
		ValueQuery,
	>;

	/// The earliest era for which we have a pending, unapplied slash.
	#[pallet::storage]
	pub type EarliestUnappliedSlash<T> = StorageValue<_, EraIndex>;

	/// The last planned session scheduled by the session pallet.
	///
	/// This is basically in sync with the call to [`SessionManager::new_session`].
	#[pallet::storage]
	#[pallet::getter(fn current_planned_session)]
	pub type CurrentPlannedSession<T> = StorageValue<_, SessionIndex, ValueQuery>;

	/// True if network has been upgraded to this version.
	/// Storage version of the pallet.
	///
	/// This is set to v7.0.0 for new networks.
	#[pallet::storage]
	pub type StorageVersion<T: Config> = StorageValue<_, Releases, ValueQuery>;

	/// The threshold for when users can start calling `chill_other` for other validators / nominators.
	/// The threshold is compared to the actual number of validators / nominators (`CountFor*`) in
	/// the system compared to the configured max (`Max*Count`).
	#[pallet::storage]
	pub type ChillThreshold<T: Config> = StorageValue<_, Percent, OptionQuery>;

	/// The chain's running time form genesis in milliseconds,
	/// use for calculate darwinia era payout
	#[pallet::storage]
	#[pallet::getter(fn living_time)]
	pub type LivingTime<T> = StorageValue<_, TsInMs, ValueQuery>;

	/// The percentage of the total payout that is distributed to validators and nominators
	///
	/// The reset might go to Treasury or something else.
	#[pallet::storage]
	#[pallet::getter(fn payout_fraction)]
	pub type PayoutFraction<T> = StorageValue<_, Perbill, ValueQuery>;

	/// Total *RING* in pool.
	#[pallet::storage]
	#[pallet::getter(fn ring_pool)]
	pub type RingPool<T> = StorageValue<_, RingBalance<T>, ValueQuery>;
	/// Total *KTON* in pool.
	#[pallet::storage]
	#[pallet::getter(fn kton_pool)]
	pub type KtonPool<T> = StorageValue<_, KtonBalance<T>, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub history_depth: u32,
		pub validator_count: u32,
		pub minimum_validator_count: u32,
		pub invulnerables: Vec<AccountId<T>>,
		pub force_era: Forcing,
		pub slash_reward_fraction: Perbill,
		pub canceled_payout: Power,
		pub stakers: Vec<(
			AccountId<T>,
			AccountId<T>,
			RingBalance<T>,
			StakerStatus<AccountId<T>>,
		)>,
		pub min_nominator_bond: RingBalance<T>,
		pub min_validator_bond: RingBalance<T>,
		pub payout_fraction: Perbill,
	}
	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig {
				history_depth: 336u32,
				validator_count: Default::default(),
				minimum_validator_count: Default::default(),
				invulnerables: Default::default(),
				force_era: Default::default(),
				slash_reward_fraction: Default::default(),
				canceled_payout: Default::default(),
				stakers: Default::default(),
				min_nominator_bond: Default::default(),
				min_validator_bond: Default::default(),
				payout_fraction: Default::default(),
			}
		}
	}
	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			<HistoryDepth<T>>::put(self.history_depth);
			<ValidatorCount<T>>::put(self.validator_count);
			<MinimumValidatorCount<T>>::put(self.minimum_validator_count);
			<Invulnerables<T>>::put(&self.invulnerables);
			<ForceEra<T>>::put(self.force_era);
			<CanceledSlashPayout<T>>::put(self.canceled_payout);
			<SlashRewardFraction<T>>::put(self.slash_reward_fraction);
			<StorageVersion<T>>::put(Releases::V7_0_0);
			<MinNominatorBond<T>>::put(self.min_nominator_bond);
			<MinValidatorBond<T>>::put(self.min_validator_bond);
			<PayoutFraction<T>>::put(self.payout_fraction);

			for (stash, controller, ring_to_be_bonded, status) in &self.stakers {
				assert!(
					T::RingCurrency::free_balance(&stash) >= *ring_to_be_bonded,
					"Stash does not have enough balance to bond.",
				);
				let _ = <Pallet<T>>::bond(
					T::Origin::from(Some(stash.to_owned()).into()),
					T::Lookup::unlookup(controller.to_owned()),
					StakingBalance::RingBalance(*ring_to_be_bonded),
					RewardDestination::Staked,
					0,
				);
				let _ = match status {
					StakerStatus::Validator => <Pallet<T>>::validate(
						T::Origin::from(Some(controller.to_owned()).into()),
						Default::default(),
					),
					StakerStatus::Nominator(votes) => <Pallet<T>>::nominate(
						T::Origin::from(Some(controller.to_owned()).into()),
						votes
							.iter()
							.map(|l| T::Lookup::unlookup(l.to_owned()))
							.collect(),
					),
					_ => Ok(()),
				};
				let _ = T::RingCurrency::make_free_balance_be(
					&<Pallet<T>>::account_id(),
					T::RingCurrency::minimum_balance(),
				);
			}
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_runtime_upgrade() -> Weight {
			if <StorageVersion<T>>::get() == Releases::V6_0_0 {
				migration::migrate::<T>()
			} else {
				T::DbWeight::get().reads(1)
			}
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<(), &'static str> {
			if <StorageVersion<T>>::get() == Releases::V6_0_0 {
				migration::pre_migrate::<T>()
			} else {
				Ok(())
			}
		}

		fn on_initialize(_now: T::BlockNumber) -> Weight {
			// just return the weight of the on_finalize.
			T::DbWeight::get().reads(1)
		}

		fn on_finalize(_n: BlockNumberFor<T>) {
			// Set the start of the first era.
			if let Some(mut active_era) = Self::active_era() {
				if active_era.start.is_none() {
					let now_as_millis_u64 = T::UnixTime::now().as_millis() as _;
					active_era.start = Some(now_as_millis_u64);
					// This write only ever happens once, we don't include it in the weight in general
					<ActiveEra<T>>::put(active_era);
				}
			}
			// `on_finalize` weight is tracked in `on_initialize`
		}

		fn integrity_test() {
			sp_std::if_std! {
				sp_io::TestExternalities::new_empty().execute_with(||{
					let slash_defer_duration = T::SlashDeferDuration::get();
					let bonding_duration_in_era = T::BondingDurationInEra::get();

					assert!(
						slash_defer_duration < bonding_duration_in_era || bonding_duration_in_era == 0,
						"As per documentation, slash defer duration ({}) should be less than bonding duration ({}).",
						slash_defer_duration,
						bonding_duration_in_era,
					)
				});
			}
		}
	}
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Take the origin account as a stash and lock up `value` of its balance. `controller` will
		/// be the account that controls it.
		///
		/// `value` must be more than the `minimum_balance` specified by `T::Currency`.
		///
		/// The dispatch origin for this call must be _Signed_ by the stash account.
		///
		/// # <weight>
		/// - Independent of the arguments. Moderate complexity.
		/// - O(1).
		/// - Three extra DB entries.
		///
		/// NOTE: Two of the storage writes (`Self::bonded`, `Self::payee`) are _never_ cleaned
		/// unless the `origin` falls below _existential deposit_ and gets removed as dust.
		/// ------------------
		/// Weight: O(1)
		/// DB Weight:
		/// - Read: Bonded, Ledger, [Origin Account], Current Era, History Depth, Locks
		/// - Write: Bonded, Payee, [Origin Account], Locks, Ledger
		/// # </weight>
		#[pallet::weight(T::WeightInfo::bond())]
		pub fn bond(
			origin: OriginFor<T>,
			controller: <T::Lookup as StaticLookup>::Source,
			value: StakingBalanceT<T>,
			payee: RewardDestination<AccountId<T>>,
			promise_month: u8,
		) -> DispatchResult {
			let stash = ensure_signed(origin)?;
			ensure!(
				!<Bonded<T>>::contains_key(&stash),
				<Error<T>>::AlreadyBonded
			);

			let controller = T::Lookup::lookup(controller)?;
			ensure!(
				!<Ledger<T>>::contains_key(&controller),
				<Error<T>>::AlreadyPaired
			);

			match value {
				StakingBalance::RingBalance(value) => {
					// Reject a bond which is considered to be _dust_.
					ensure!(
						value >= T::RingCurrency::minimum_balance(),
						<Error<T>>::InsufficientBond,
					);
				}
				StakingBalance::KtonBalance(value) => {
					// Reject a bond which is considered to be _dust_.
					ensure!(
						value >= T::KtonCurrency::minimum_balance(),
						<Error<T>>::InsufficientBond,
					);
				}
			}

			<frame_system::Pallet<T>>::inc_consumers(&stash).map_err(|_| <Error<T>>::BadState)?;

			// You're auto-bonded forever, here. We might improve this by only bonding when
			// you actually validate/nominate and remove once you unbond __everything__.
			<Bonded<T>>::insert(&stash, &controller);
			<Payee<T>>::insert(&stash, payee);

			let ledger = StakingLedger {
				stash: stash.clone(),
				claimed_rewards: {
					let current_era = <CurrentEra<T>>::get().unwrap_or(0);
					let last_reward_era = current_era.saturating_sub(Self::history_depth());
					(last_reward_era..current_era).collect()
				},
				..Default::default()
			};

			match value {
				StakingBalance::RingBalance(value) => {
					let stash_balance = T::RingCurrency::free_balance(&stash);
					let value = value.min(stash_balance);
					let promise_month = promise_month.min(36);
					let (start_time, expire_time) =
						Self::bond_ring(&stash, &controller, value, promise_month, ledger)?;

					Self::deposit_event(Event::BondRing(value, start_time, expire_time));
				}
				StakingBalance::KtonBalance(value) => {
					let stash_balance = T::KtonCurrency::free_balance(&stash);
					let value = value.min(stash_balance);

					Self::bond_kton(&controller, value, ledger)?;
					Self::deposit_event(Event::BondKton(value));
				}
			}

			Ok(())
		}

		/// Add some extra amount that have appeared in the stash `free_balance` into the balance up
		/// for staking.
		///
		/// Use this if there are additional funds in your stash account that you wish to bond.
		/// Unlike [`bond`] or [`unbond`] this function does not impose any limitation on the amount
		/// that can be added.
		///
		/// The dispatch origin for this call must be _Signed_ by the stash, not the controller and
		/// it can be only called when [`EraElectionStatus`] is `Closed`.
		///
		/// Emits `Bonded`.
		///
		/// # <weight>
		/// - Independent of the arguments. Insignificant complexity.
		/// - O(1).
		/// - One DB entry.
		/// ------------
		/// DB Weight:
		/// - Read: Era Election Status, Bonded, Ledger, [Origin Account], Locks
		/// - Write: [Origin Account], Locks, Ledger
		/// # </weight>
		#[pallet::weight(T::WeightInfo::bond_extra())]
		pub fn bond_extra(
			origin: OriginFor<T>,
			max_additional: StakingBalanceT<T>,
			promise_month: u8,
		) -> DispatchResult {
			let stash = ensure_signed(origin)?;
			let controller = Self::bonded(&stash).ok_or(<Error<T>>::NotStash)?;
			let ledger = Self::ledger(&controller).ok_or(<Error<T>>::NotController)?;
			let promise_month = promise_month.min(36);

			match max_additional {
				StakingBalance::RingBalance(max_additional) => {
					let stash_balance = T::RingCurrency::free_balance(&stash);

					if let Some(extra) = stash_balance.checked_sub(
						&ledger.ring_locked_amount_at(<frame_system::Pallet<T>>::block_number()),
					) {
						let extra = extra.min(max_additional);
						let (start_time, expire_time) =
							Self::bond_ring(&stash, &controller, extra, promise_month, ledger)?;

						Self::deposit_event(Event::BondRing(extra, start_time, expire_time));
					}
				}
				StakingBalance::KtonBalance(max_additional) => {
					let stash_balance = T::KtonCurrency::free_balance(&stash);

					if let Some(extra) = stash_balance.checked_sub(
						&ledger.kton_locked_amount_at(<frame_system::Pallet<T>>::block_number()),
					) {
						let extra = extra.min(max_additional);

						Self::bond_kton(&controller, extra, ledger)?;
						Self::deposit_event(Event::BondKton(extra));
					}
				}
			}

			Ok(())
		}

		/// Deposit some extra amount ring, and return kton to the controller.
		///
		/// The dispatch origin for this call must be _Signed_ by the stash, not the controller.
		///
		/// Is a no-op if value to be deposited is zero.
		///
		/// # <weight>
		/// - Independent of the arguments. Insignificant complexity.
		/// - O(1).
		/// - One DB entry.
		/// ------------
		/// DB Weight:
		/// - Read: Era Election Status, Bonded, Ledger, [Origin Account]
		/// - Write: [Origin Account], Ledger
		/// # </weight>
		#[pallet::weight(T::WeightInfo::deposit_extra())]
		pub fn deposit_extra(
			origin: OriginFor<T>,
			value: RingBalance<T>,
			promise_month: u8,
		) -> DispatchResult {
			let stash = ensure_signed(origin)?;
			let controller = Self::bonded(&stash).ok_or(<Error<T>>::NotStash)?;
			let ledger = Self::ledger(&controller).ok_or(<Error<T>>::NotController)?;

			if value.is_zero() {
				return Ok(());
			}

			let start_time = T::UnixTime::now().as_millis().saturated_into::<TsInMs>();
			let promise_month = promise_month.max(1).min(36);
			let expire_time = start_time + promise_month as TsInMs * MONTH_IN_MILLISECONDS;
			let mut ledger = Self::clear_mature_deposits(ledger).0;
			let StakingLedger {
				stash,
				active_ring,
				active_deposit_ring,
				deposit_items,
				..
			} = &mut ledger;
			let value = value.min(active_ring.saturating_sub(*active_deposit_ring));

			if value.is_zero() {
				return Ok(());
			}

			let kton_return = inflation::compute_kton_reward::<T>(value, promise_month);
			let kton_positive_imbalance = T::KtonCurrency::deposit_creating(&stash, kton_return);

			T::KtonReward::on_unbalanced(kton_positive_imbalance);
			*active_deposit_ring = active_deposit_ring.saturating_add(value);
			deposit_items.push(TimeDepositItem {
				value,
				start_time,
				expire_time,
			});

			<Ledger<T>>::insert(&controller, ledger);
			Self::deposit_event(Event::BondRing(value, start_time, expire_time));

			Ok(())
		}

		/// Schedule a portion of the stash to be unlocked ready for transfer out after the bond
		/// period ends. If this leaves an amount actively bonded less than
		/// T::Currency::minimum_balance(), then it is increased to the full amount.
		///
		/// Once the unlock period is done, the funds will be withdrew automatically and ready for transfer.
		///
		/// No more than a limited number of unlocking chunks (see `MAX_UNLOCKING_CHUNKS`)
		/// can co-exists at the same time. In that case,  [`StakingLock::shrink`] need
		/// to be called first to remove some of the chunks (if possible).
		///
		/// If a user encounters the `InsufficientBond` error when calling this extrinsic,
		/// they should call `chill` first in order to free up their bonded funds.
		///
		/// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
		/// And, it can be only called when [`EraElectionStatus`] is `Closed`.
		///
		/// After all pledged Ring and Kton are unbonded, the bonded accounts, namely stash and
		/// controller, will also be unbonded.  Once user want to bond again, the `bond` method
		/// should be called. If there are still pledged Ring or Kton and user want to bond more
		/// values, the `bond_extra` method should be called.
		///
		/// # <weight>
		/// - Independent of the arguments. Limited but potentially exploitable complexity.
		/// - Contains a limited number of reads.
		/// - Each call (requires the remainder of the bonded balance to be above `minimum_balance`)
		///   will cause a new entry to be inserted into a vector (`StakingLock.unbondings`) kept in storage.
		/// - One DB entry.
		/// ----------
		/// Weight: O(1)
		/// DB Weight:
		/// - Read: EraElectionStatus, Ledger, CurrentEra, Locks, BalanceOf Stash,
		/// - Write: Locks, Ledger, BalanceOf Stash,
		/// </weight>
		#[pallet::weight(T::WeightInfo::unbond())]
		pub fn unbond(origin: OriginFor<T>, value: StakingBalanceT<T>) -> DispatchResult {
			let controller = ensure_signed(origin)?;
			let mut ledger = Self::clear_mature_deposits(
				Self::ledger(&controller).ok_or(<Error<T>>::NotController)?,
			)
			.0;
			let StakingLedger {
				stash,
				active_ring,
				active_deposit_ring,
				active_kton,
				ring_staking_lock,
				kton_staking_lock,
				..
			} = &mut ledger;
			let now = <frame_system::Pallet<T>>::block_number();

			ring_staking_lock.update(now);
			kton_staking_lock.update(now);

			// Due to the macro parser, we've to add a bracket.
			// Actually, this's totally wrong:
			//	 `a as u32 + b as u32 < c`
			// Workaround:
			//	 1. `(a as u32 + b as u32) < c`
			//	 2. `let c_ = a as u32 + b as u32; c_ < c`
			ensure!(
				(ring_staking_lock.unbondings.len() + kton_staking_lock.unbondings.len())
					< MAX_UNLOCKING_CHUNKS,
				<Error<T>>::NoMoreChunks,
			);

			let mut unbond_ring: RingBalance<T> = Zero::zero();
			let mut unbond_kton: KtonBalance<T> = Zero::zero();

			match value {
				StakingBalance::RingBalance(r) => {
					// Only active normal ring can be unbond:
					// `active_ring = active_normal_ring + active_deposit_ring`
					let active_normal_ring = *active_ring - *active_deposit_ring;
					unbond_ring = r.min(active_normal_ring);

					if !unbond_ring.is_zero() {
						*active_ring -= unbond_ring;

						// Avoid there being a dust balance left in the staking system.
						if (*active_ring < T::RingCurrency::minimum_balance())
							&& (*active_kton < T::KtonCurrency::minimum_balance())
						{
							unbond_ring += *active_ring;
							unbond_kton += *active_kton;

							*active_ring = Zero::zero();
							*active_kton = Zero::zero();
						}

						let min_active_bond = if <Nominators<T>>::contains_key(&*stash) {
							<MinNominatorBond<T>>::get()
						} else if <Validators<T>>::contains_key(&*stash) {
							<MinValidatorBond<T>>::get()
						} else {
							Zero::zero()
						};

						// Make sure that the user maintains enough active bond for their role.
						// If a user runs into this error, they should chill first.
						ensure!(
							*active_ring >= min_active_bond,
							<Error<T>>::InsufficientBond
						);

						ring_staking_lock
							.unbondings
							.try_push(Unbonding {
								amount: unbond_ring,
								until: now + T::BondingDurationInBlockNumber::get(),
							})
							.expect("ALREADY CHECKED THE BOUNDARY MUST NOT FAIL!");

						Self::deposit_event(Event::UnbondRing(unbond_ring, now));

						if !unbond_kton.is_zero() {
							kton_staking_lock
								.unbondings
								.try_push(Unbonding {
									amount: unbond_kton,
									until: now + T::BondingDurationInBlockNumber::get(),
								})
								.expect("ALREADY CHECKED THE BOUNDARY MUST NOT FAIL!");

							Self::deposit_event(Event::UnbondKton(unbond_kton, now));
						}
					}
				}
				StakingBalance::KtonBalance(k) => {
					unbond_kton = k.min(*active_kton);

					if !unbond_kton.is_zero() {
						*active_kton -= unbond_kton;

						// Avoid there being a dust balance left in the staking system.
						if (*active_kton < T::KtonCurrency::minimum_balance())
							&& (*active_ring < T::RingCurrency::minimum_balance())
						{
							unbond_kton += *active_kton;
							unbond_ring += *active_ring;

							*active_kton = Zero::zero();
							*active_ring = Zero::zero();
						}

						kton_staking_lock
							.unbondings
							.try_push(Unbonding {
								amount: unbond_kton,
								until: now + T::BondingDurationInBlockNumber::get(),
							})
							.expect("ALREADY CHECKED THE BOUNDARY MUST NOT FAIL!");

						Self::deposit_event(Event::UnbondKton(unbond_kton, now));

						if !unbond_ring.is_zero() {
							ring_staking_lock
								.unbondings
								.try_push(Unbonding {
									amount: unbond_ring,
									until: now + T::BondingDurationInBlockNumber::get(),
								})
								.expect("ALREADY CHECKED THE BOUNDARY MUST NOT FAIL!");

							Self::deposit_event(Event::UnbondRing(unbond_ring, now));
						}
					}
				}
			}

			Self::update_ledger(&controller, &mut ledger);

			// TODO: https://github.com/darwinia-network/darwinia-common/issues/96
			// FIXME: https://github.com/darwinia-network/darwinia-common/issues/121
			// let StakingLedger {
			// 	active_ring,
			// 	active_kton,
			// 	..
			// } = ledger;

			// // All bonded *RING* and *KTON* is withdrawing, then remove Ledger to save storage
			// if active_ring.is_zero() && active_kton.is_zero() {
			// 	//
			// 	// `OnKilledAccount` would be a method to collect the locks.
			// 	//
			// 	// These locks are still in the system, and should be removed after 14 days
			// 	//
			// 	// There two situations should be considered after the 14 days
			// 	// - the user never bond again, so the locks should be released.
			// 	// - the user is bonded again in the 14 days, so the after 14 days
			// 	//   the lock should not be removed
			// 	//
			// 	// If the locks are not deleted, this lock will waste the storage in the future
			// 	// blocks.
			// 	//
			// 	// T::Ring::remove_lock(STAKING_ID, &stash);
			// 	// T::Kton::remove_lock(STAKING_ID, &stash);
			// 	// Self::kill_stash(&stash)?;
			// }

			Ok(())
		}

		/// Stash accounts can get their ring back after the depositing time exceeded,
		/// and the ring getting back is still in staking status.
		///
		/// # <weight>
		/// - Independent of the arguments. Insignificant complexity.
		/// - One storage read.
		/// - One storage write.
		/// - Writes are limited to the `origin` account key.
		/// ----------
		/// DB Weight:
		/// - Read: Ledger, [Origin Account]
		/// - Write: [Origin Account], Ledger
		/// # </weight>
		#[pallet::weight(50 * WEIGHT_PER_MICROS + T::DbWeight::get().reads_writes(2, 2))]
		pub fn claim_mature_deposits(origin: OriginFor<T>) -> DispatchResult {
			let controller = ensure_signed(origin)?;
			let (ledger, mutated) = Self::clear_mature_deposits(
				Self::ledger(&controller).ok_or(<Error<T>>::NotController)?,
			);

			if mutated {
				<Ledger<T>>::insert(controller, ledger);
			}

			Ok(())
		}

		/// Claim deposits while the depositing time has not been exceeded, the ring
		/// will not be slashed, but the account is required to pay KTON as punish.
		///
		/// Refer to https://talk.darwinia.network/topics/55
		///
		/// Assume the `expire_time` is a unique ID for the deposit
		///
		/// # <weight>
		/// - Independent of the arguments. Insignificant complexity.
		/// - One storage read.
		/// - One storage write.
		/// - Writes are limited to the `origin` account key.
		/// ----------
		/// DB Weight:
		/// - Read: Ledger, Locks, [Origin Account]
		/// - Write: [Origin Account], Locks, Ledger
		/// # </weight>
		#[pallet::weight(50 * WEIGHT_PER_MICROS + T::DbWeight::get().reads_writes(3, 2))]
		pub fn try_claim_deposits_with_punish(
			origin: OriginFor<T>,
			expire_time: TsInMs,
		) -> DispatchResult {
			let controller = ensure_signed(origin)?;
			let mut ledger = Self::ledger(&controller).ok_or(<Error<T>>::NotController)?;
			let now = T::UnixTime::now().as_millis().saturated_into::<TsInMs>();

			if expire_time <= now {
				return Ok(());
			}

			let mut claim_deposits_with_punish = (false, Zero::zero());

			{
				let StakingLedger {
					stash,
					active_deposit_ring,
					deposit_items,
					..
				} = &mut ledger;

				deposit_items.retain(|item| {
					if item.expire_time != expire_time {
						return true;
					}

					let kton_slash = {
						let plan_duration_in_months = {
							let plan_duration_in_milliseconds =
								item.expire_time.saturating_sub(item.start_time);

							plan_duration_in_milliseconds / MONTH_IN_MILLISECONDS
						};
						let passed_duration_in_months = {
							let passed_duration_in_milliseconds =
								now.saturating_sub(item.start_time);

							passed_duration_in_milliseconds / MONTH_IN_MILLISECONDS
						};

						(inflation::compute_kton_reward::<T>(
							item.value,
							plan_duration_in_months as _,
						) - inflation::compute_kton_reward::<T>(
							item.value,
							passed_duration_in_months as _,
						))
						.max(1u32.into()) * 3u32.into()
					};

					// check total free balance and locked one
					// strict on punishing in kton
					if T::KtonCurrency::usable_balance(stash) >= kton_slash {
						*active_deposit_ring = active_deposit_ring.saturating_sub(item.value);

						let imbalance = T::KtonCurrency::slash(stash, kton_slash).0;
						T::KtonSlash::on_unbalanced(imbalance);

						claim_deposits_with_punish = (true, kton_slash);

						false
					} else {
						true
					}
				});
			}

			<Ledger<T>>::insert(&controller, &ledger);

			if claim_deposits_with_punish.0 {
				Self::deposit_event(Event::DepositsClaimedWithPunish(
					ledger.stash.clone(),
					claim_deposits_with_punish.1,
				));
			}

			Ok(())
		}

		/// Declare the desire to validate for the origin controller.
		///
		/// Effects will be felt at the beginning of the next era.
		///
		/// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
		/// And, it can be only called when [`EraElectionStatus`] is `Closed`.
		///
		/// # <weight>
		/// - Independent of the arguments. Insignificant complexity.
		/// - Contains a limited number of reads.
		/// - Writes are limited to the `origin` account key.
		/// -----------
		/// Weight: O(1)
		/// DB Weight:
		/// - Read: Era Election Status, Ledger
		/// - Write: Nominators, Validators
		/// # </weight>
		#[pallet::weight(T::WeightInfo::validate())]
		pub fn validate(origin: OriginFor<T>, prefs: ValidatorPrefs) -> DispatchResult {
			let controller = ensure_signed(origin)?;
			let ledger = Self::ledger(&controller).ok_or(<Error<T>>::NotController)?;

			ensure!(
				ledger.active_ring >= <MinValidatorBond<T>>::get(),
				<Error<T>>::InsufficientBond
			);

			let stash = &ledger.stash;

			// Only check limits if they are not already a validator.
			if !<Validators<T>>::contains_key(stash) {
				// If this error is reached, we need to adjust the `MinValidatorBond` and start calling `chill_other`.
				// Until then, we explicitly block new validators to protect the runtime.
				if let Some(max_validators) = <MaxValidatorsCount<T>>::get() {
					ensure!(
						<CounterForValidators<T>>::get() < max_validators,
						<Error<T>>::TooManyValidators
					);
				}
			}

			Self::do_remove_nominator(stash);
			Self::do_add_validator(stash, prefs);

			Ok(())
		}

		/// Declare the desire to nominate `targets` for the origin controller.
		///
		/// Effects will be felt at the beginning of the next era. This can only be called when
		/// [`EraElectionStatus`] is `Closed`.
		///
		/// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
		/// And, it can be only called when [`EraElectionStatus`] is `Closed`.
		///
		/// # <weight>
		/// - The transaction's complexity is proportional to the size of `targets` (N)
		/// which is capped at CompactAssignments::LIMIT (MAX_NOMINATIONS).
		/// - Both the reads and writes follow a similar pattern.
		/// ---------
		/// Weight: O(N)
		/// where N is the number of targets
		/// DB Weight:
		/// - Reads: Era Election Status, Ledger, Current Era
		/// - Writes: Validators, Nominators
		/// # </weight>
		#[pallet::weight(T::WeightInfo::nominate(targets.len() as u32))]
		pub fn nominate(
			origin: OriginFor<T>,
			targets: Vec<<T::Lookup as StaticLookup>::Source>,
		) -> DispatchResult {
			let controller = ensure_signed(origin)?;
			let ledger = Self::ledger(&controller).ok_or(<Error<T>>::NotController)?;

			ensure!(
				ledger.active_ring >= <MinNominatorBond<T>>::get(),
				<Error<T>>::InsufficientBond
			);

			let stash = &ledger.stash;

			// Only check limits if they are not already a nominator.
			if !<Nominators<T>>::contains_key(stash) {
				// If this error is reached, we need to adjust the `MinNominatorBond` and start calling `chill_other`.
				// Until then, we explicitly block new nominators to protect the runtime.
				if let Some(max_nominators) = <MaxNominatorsCount<T>>::get() {
					ensure!(
						<CounterForNominators<T>>::get() < max_nominators,
						<Error<T>>::TooManyNominators
					);
				}
			}

			ensure!(!targets.is_empty(), <Error<T>>::EmptyTargets);
			ensure!(
				targets.len() <= T::MAX_NOMINATIONS as usize,
				<Error<T>>::TooManyTargets
			);

			let old = <Nominators<T>>::get(stash).map_or_else(Vec::new, |x| x.targets);
			let targets = targets
				.into_iter()
				.map(|t| T::Lookup::lookup(t).map_err(DispatchError::from))
				.map(|n| {
					n.and_then(|n| {
						if old.contains(&n) || !<Validators<T>>::get(&n).blocked {
							Ok(n)
						} else {
							Err(<Error<T>>::BadTarget.into())
						}
					})
				})
				.collect::<Result<Vec<AccountId<T>>, _>>()?;
			let nominations = Nominations {
				targets,
				// Initial nominations are considered submitted at era 0. See `Nominations` doc
				submitted_in: Self::current_era().unwrap_or(0),
				suppressed: false,
			};

			Self::do_remove_validator(stash);
			Self::do_add_nominator(stash, nominations);

			Ok(())
		}

		/// Declare no desire to either validate or nominate.
		///
		/// Effects will be felt at the beginning of the next era.
		///
		/// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
		/// And, it can be only called when [`EraElectionStatus`] is `Closed`.
		///
		/// # <weight>
		/// - Independent of the arguments. Insignificant complexity.
		/// - Contains one read.
		/// - Writes are limited to the `origin` account key.
		/// --------
		/// Weight: O(1)
		/// DB Weight:
		/// - Read: EraElectionStatus, Ledger
		/// - Write: Validators, Nominators
		/// # </weight>
		#[pallet::weight(T::WeightInfo::chill())]
		pub fn chill(origin: OriginFor<T>) -> DispatchResult {
			let controller = ensure_signed(origin)?;
			let ledger = Self::ledger(&controller).ok_or(<Error<T>>::NotController)?;

			Self::chill_stash(&ledger.stash);

			Ok(())
		}

		/// (Re-)set the payment target for a controller.
		///
		/// Effects will be felt at the beginning of the next era.
		///
		/// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
		///
		/// # <weight>
		/// - Independent of the arguments. Insignificant complexity.
		/// - Contains a limited number of reads.
		/// - Writes are limited to the `origin` account key.
		/// ---------
		/// - Weight: O(1)
		/// - DB Weight:
		///     - Read: Ledger
		///     - Write: Payee
		/// # </weight>
		#[pallet::weight(T::WeightInfo::set_payee())]
		pub fn set_payee(
			origin: OriginFor<T>,
			payee: RewardDestination<AccountId<T>>,
		) -> DispatchResult {
			let controller = ensure_signed(origin)?;
			let ledger = Self::ledger(&controller).ok_or(<Error<T>>::NotController)?;
			let stash = &ledger.stash;

			<Payee<T>>::insert(stash, payee);

			Ok(())
		}

		/// (Re-)set the controller of a stash.
		///
		/// Effects will be felt at the beginning of the next era.
		///
		/// The dispatch origin for this call must be _Signed_ by the stash, not the controller.
		///
		/// # <weight>
		/// - Independent of the arguments. Insignificant complexity.
		/// - Contains a limited number of reads.
		/// - Writes are limited to the `origin` account key.
		/// ----------
		/// Weight: O(1)
		/// DB Weight:
		/// - Read: Bonded, Ledger New Controller, Ledger Old Controller
		/// - Write: Bonded, Ledger New Controller, Ledger Old Controller
		/// # </weight>
		#[pallet::weight(T::WeightInfo::set_controller())]
		pub fn set_controller(
			origin: OriginFor<T>,
			controller: <T::Lookup as StaticLookup>::Source,
		) -> DispatchResult {
			let stash = ensure_signed(origin)?;
			let old_controller = Self::bonded(&stash).ok_or(<Error<T>>::NotStash)?;
			let controller = T::Lookup::lookup(controller)?;

			ensure!(
				!<Ledger<T>>::contains_key(&controller),
				<Error<T>>::AlreadyPaired
			);

			if controller != old_controller {
				<Bonded<T>>::insert(&stash, &controller);
				if let Some(l) = <Ledger<T>>::take(&old_controller) {
					<Ledger<T>>::insert(&controller, l);
				}
			}

			Ok(())
		}

		// --- root call ---

		/// Sets the ideal number of validators.
		///
		/// The dispatch origin must be Root.
		///
		/// # <weight>
		/// Weight: O(1)
		/// Write: Validator Count
		/// # </weight>
		#[pallet::weight(T::WeightInfo::set_validator_count())]
		pub fn set_validator_count(
			origin: OriginFor<T>,
			#[pallet::compact] new: u32,
		) -> DispatchResult {
			ensure_root(origin)?;

			<ValidatorCount<T>>::put(new);

			Ok(())
		}

		/// Increments the ideal number of validators.
		///
		/// The dispatch origin must be Root.
		///
		/// # <weight>
		/// Same as [`set_validator_count`].
		/// # </weight>
		#[pallet::weight(T::WeightInfo::set_validator_count())]
		pub fn increase_validator_count(
			origin: OriginFor<T>,
			#[pallet::compact] additional: u32,
		) -> DispatchResult {
			ensure_root(origin)?;

			<ValidatorCount<T>>::mutate(|n| *n += additional);

			Ok(())
		}

		/// Scale up the ideal number of validators by a factor.
		///
		/// The dispatch origin must be Root.
		///
		/// # <weight>
		/// Same as [`set_validator_count`].
		/// # </weight>
		#[pallet::weight(T::WeightInfo::set_validator_count())]
		pub fn scale_validator_count(origin: OriginFor<T>, factor: Percent) -> DispatchResult {
			ensure_root(origin)?;

			<ValidatorCount<T>>::mutate(|n| *n += factor * *n);

			Ok(())
		}

		/// Force there to be no new eras indefinitely.
		///
		/// The dispatch origin must be Root.
		///
		/// # Warning
		///
		/// The election process starts multiple blocks before the end of the era.
		/// Thus the election process may be ongoing when this is called. In this case the
		/// election will continue until the next era is triggered.
		///
		/// # <weight>
		/// - No arguments.
		/// - Weight: O(1)
		/// - Write: ForceEra
		/// # </weight>
		#[pallet::weight(T::WeightInfo::force_no_eras())]
		pub fn force_no_eras(origin: OriginFor<T>) -> DispatchResult {
			ensure_root(origin)?;

			<ForceEra<T>>::put(Forcing::ForceNone);

			Ok(())
		}

		/// Force there to be a new era at the end of the next session. After this, it will be
		/// reset to normal (non-forced) behaviour.
		///
		/// The dispatch origin must be Root.
		///
		/// # Warning
		///
		/// The election process starts multiple blocks before the end of the era.
		/// If this is called just before a new era is triggered, the election process may not
		/// have enough blocks to get a result.
		///
		/// # <weight>
		/// - No arguments.
		/// - Weight: O(1)
		/// - Write ForceEra
		/// # </weight>
		#[pallet::weight(T::WeightInfo::force_new_era())]
		pub fn force_new_era(origin: OriginFor<T>) -> DispatchResult {
			ensure_root(origin)?;

			<ForceEra<T>>::put(Forcing::ForceNew);

			Ok(())
		}

		/// Set the validators who cannot be slashed (if any).
		///
		/// The dispatch origin must be Root.
		///
		/// # <weight>
		/// - O(V)
		/// - Write: Invulnerables
		/// # </weight>
		#[pallet::weight(T::WeightInfo::set_invulnerables(invulnerables.len() as u32))]
		pub fn set_invulnerables(
			origin: OriginFor<T>,
			invulnerables: Vec<AccountId<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			<Invulnerables<T>>::put(invulnerables);

			Ok(())
		}

		/// Force a current staker to become completely unstaked, immediately.
		///
		/// The dispatch origin must be Root.
		///
		/// # <weight>
		/// O(S) where S is the number of slashing spans to be removed
		/// Reads: Bonded, Slashing Spans, Account, Locks
		/// Writes: Bonded, Slashing Spans (if S > 0), Ledger, Payee, Validators, Nominators, Account, Locks
		/// Writes Each: SpanSlash * S
		/// # </weight>
		#[pallet::weight(T::WeightInfo::force_unstake(*num_slashing_spans))]
		pub fn force_unstake(
			origin: OriginFor<T>,
			stash: AccountId<T>,
			num_slashing_spans: u32,
		) -> DispatchResult {
			ensure_root(origin)?;

			// Remove all staking-related information.
			Self::kill_stash(&stash, num_slashing_spans)?;

			// Remove the lock.
			T::RingCurrency::remove_lock(STAKING_ID, &stash);
			T::KtonCurrency::remove_lock(STAKING_ID, &stash);

			Ok(())
		}

		/// Force there to be a new era at the end of sessions indefinitely.
		///
		/// The dispatch origin must be Root.
		///
		/// # Warning
		///
		/// The election process starts multiple blocks before the end of the era.
		/// If this is called just before a new era is triggered, the election process may not
		/// have enough blocks to get a result.
		///
		/// # <weight>
		/// - Weight: O(1)
		/// - Write: ForceEra
		/// # </weight>
		#[pallet::weight(T::WeightInfo::force_new_era_always())]
		pub fn force_new_era_always(origin: OriginFor<T>) -> DispatchResult {
			ensure_root(origin)?;

			<ForceEra<T>>::put(Forcing::ForceAlways);

			Ok(())
		}

		/// Cancel enactment of a deferred slash.
		///
		/// Can be called by the `T::SlashCancelOrigin`.
		///
		/// Parameters: era and indices of the slashes for that era to kill.
		///
		/// # <weight>
		/// Complexity: O(U + S)
		/// with U unapplied slashes weighted with U=1000
		/// and S is the number of slash indices to be canceled.
		/// - Read: Unapplied Slashes
		/// - Write: Unapplied Slashes
		/// # </weight>
		#[pallet::weight(T::WeightInfo::cancel_deferred_slash(slash_indices.len() as u32))]
		pub fn cancel_deferred_slash(
			origin: OriginFor<T>,
			era: EraIndex,
			slash_indices: Vec<u32>,
		) -> DispatchResult {
			T::SlashCancelOrigin::ensure_origin(origin)?;

			ensure!(!slash_indices.is_empty(), <Error<T>>::EmptyTargets);
			ensure!(
				is_sorted_and_unique(&slash_indices),
				<Error<T>>::NotSortedAndUnique
			);

			let mut unapplied = <Self as Store>::UnappliedSlashes::get(&era);
			let last_item = slash_indices[slash_indices.len() - 1];
			ensure!(
				(last_item as usize) < unapplied.len(),
				<Error<T>>::InvalidSlashIndex
			);

			for (removed, index) in slash_indices.into_iter().enumerate() {
				let index = (index as usize) - removed;
				unapplied.remove(index);
			}

			<Self as Store>::UnappliedSlashes::insert(&era, &unapplied);

			Ok(())
		}

		/// Pay out all the stakers behind a single validator for a single era.
		///
		/// - `validator_stash` is the stash account of the validator. Their nominators, up to
		///   `T::MaxNominatorRewardedPerValidator`, will also receive their rewards.
		/// - `era` may be any era between `[current_era - history_depth; current_era]`.
		///
		/// The origin of this call must be _Signed_. Any account can call this function, even if
		/// it is not one of the stakers.
		///
		/// This can only be called when [`EraElectionStatus`] is `Closed`.
		///
		/// # <weight>
		/// - Time complexity: at most O(MaxNominatorRewardedPerValidator).
		/// - Contains a limited number of reads and writes.
		/// -----------
		/// N is the Number of payouts for the validator (including the validator)
		/// Weight:
		/// - Reward Destination Staked: O(N)
		/// - Reward Destination Controller (Creating): O(N)
		/// DB Weight:
		/// - Read: EraElectionStatus, CurrentEra, HistoryDepth, ErasValidatorReward,
		///         ErasStakersClipped, ErasRewardPoints, ErasValidatorPrefs (8 items)
		/// - Read Each: Bonded, Ledger, Payee, Locks, System Account (5 items)
		/// - Write Each: System Account, Locks, Ledger (3 items)
		///
		///   NOTE: weights are assuming that payouts are made to alive stash account (Staked).
		///   Paying even a dead controller is cheaper weight-wise. We don't do any refunds here.
		/// # </weight>
		#[pallet::weight(T::WeightInfo::payout_stakers_alive_staked(
			T::MaxNominatorRewardedPerValidator::get()
		))]
		pub fn payout_stakers(
			origin: OriginFor<T>,
			validator_stash: AccountId<T>,
			era: EraIndex,
		) -> DispatchResultWithPostInfo {
			ensure_signed(origin)?;

			Self::do_payout_stakers(validator_stash, era)
		}

		/// Rebond a portion of the stash scheduled to be unlocked.
		///
		/// The dispatch origin must be signed by the controller, and it can be only called when
		/// [`EraElectionStatus`] is `Closed`.
		///
		/// # <weight>
		/// - Time complexity: O(L), where L is unlocking chunks
		/// - Bounded by `MAX_UNLOCKING_CHUNKS`.
		/// - Storage changes: Can't increase storage, only decrease it.
		/// ---------------
		/// - DB Weight:
		///     - Reads: EraElectionStatus, Ledger, Locks, [Origin Account]
		///     - Writes: [Origin Account], Locks, Ledger
		/// # </weight>
		#[pallet::weight(T::WeightInfo::rebond(MAX_UNLOCKING_CHUNKS as u32))]
		pub fn rebond(
			origin: OriginFor<T>,
			#[pallet::compact] plan_to_rebond_ring: RingBalance<T>,
			#[pallet::compact] plan_to_rebond_kton: KtonBalance<T>,
		) -> DispatchResultWithPostInfo {
			let controller = ensure_signed(origin)?;
			let mut ledger = Self::ledger(&controller).ok_or(<Error<T>>::NotController)?;
			let now = <frame_system::Pallet<T>>::block_number();

			ledger.ring_staking_lock.update(now);
			ledger.kton_staking_lock.update(now);

			ensure!(
				!ledger.ring_staking_lock.unbondings.is_empty()
					|| !ledger.kton_staking_lock.unbondings.is_empty(),
				<Error<T>>::NoUnlockChunk
			);

			let origin_active_ring = ledger.active_ring;
			let origin_active_kton = ledger.active_kton;

			ledger.rebond(plan_to_rebond_ring, plan_to_rebond_kton);

			// Last check: the new active amount of ledger must be more than ED.
			ensure!(
				ledger.active_ring >= T::RingCurrency::minimum_balance()
					|| ledger.active_kton >= T::KtonCurrency::minimum_balance(),
				<Error<T>>::InsufficientBond
			);

			Self::update_ledger(&controller, &mut ledger);

			let rebond_ring = ledger.active_ring.saturating_sub(origin_active_ring);
			let rebond_kton = ledger.active_kton.saturating_sub(origin_active_kton);

			if !rebond_ring.is_zero() {
				let now = T::UnixTime::now().as_millis().saturated_into::<TsInMs>();

				Self::deposit_event(Event::BondRing(rebond_ring, now, now));
			}
			if !rebond_kton.is_zero() {
				Self::deposit_event(Event::BondKton(rebond_kton));
			}

			Ok(Some(
				35 * WEIGHT_PER_MICROS
					+ 50 * WEIGHT_PER_NANOS
						* (ledger.ring_staking_lock.unbondings.len() as Weight
							+ ledger.kton_staking_lock.unbondings.len() as Weight)
					+ T::DbWeight::get().reads_writes(3, 2),
			)
			.into())
		}

		/// Set `HistoryDepth` value. This function will delete any history information
		/// when `HistoryDepth` is reduced.
		///
		/// Parameters:
		/// - `new_history_depth`: The new history depth you would like to set.
		/// - `era_items_deleted`: The number of items that will be deleted by this dispatch.
		///    This should report all the storage items that will be deleted by clearing old
		///    era history. Needed to report an accurate weight for the dispatch. Trusted by
		///    `Root` to report an accurate number.
		///
		/// Origin must be root.
		///
		/// # <weight>
		/// - E: Number of history depths removed, i.e. 10 -> 7 = 3
		/// - Weight: O(E)
		/// - DB Weight:
		///     - Reads: Current Era, History Depth
		///     - Writes: History Depth
		///     - Clear Prefix Each: Era Stakers, EraStakersClipped, ErasValidatorPrefs
		///     - Writes Each: ErasValidatorReward, ErasRewardPoints, ErasTotalStake, ErasStartSessionIndex
		/// # </weight>
		#[pallet::weight(T::WeightInfo::set_history_depth(*_era_items_deleted))]
		pub fn set_history_depth(
			origin: OriginFor<T>,
			#[pallet::compact] new_history_depth: EraIndex,
			#[pallet::compact] _era_items_deleted: u32,
		) -> DispatchResult {
			ensure_root(origin)?;

			if let Some(current_era) = Self::current_era() {
				<HistoryDepth<T>>::mutate(|history_depth| {
					let last_kept = current_era.checked_sub(*history_depth).unwrap_or(0);
					let new_last_kept = current_era.checked_sub(new_history_depth).unwrap_or(0);
					for era_index in last_kept..new_last_kept {
						Self::clear_era_information(era_index);
					}
					*history_depth = new_history_depth
				})
			}

			Ok(())
		}

		/// Remove all data structure concerning a staker/stash once its balance is at the minimum.
		/// This is essentially equivalent to `withdraw_unbonded` except it can be called by anyone
		/// and the target `stash` must have no funds left beyond the ED.
		///
		/// This can be called from any origin.
		///
		/// - `stash`: The stash account to reap. Its balance must be zero.
		///
		/// # <weight>
		/// Complexity: O(S) where S is the number of slashing spans on the account.
		/// DB Weight:
		/// - Reads: Stash Account, Bonded, Slashing Spans, Locks
		/// - Writes: Bonded, Slashing Spans (if S > 0), Ledger, Payee, Validators, Nominators, Stash Account, Locks
		/// - Writes Each: SpanSlash * S
		/// # </weight>
		#[pallet::weight(T::WeightInfo::reap_stash(*num_slashing_spans))]
		pub fn reap_stash(
			_origin: OriginFor<T>,
			stash: AccountId<T>,
			num_slashing_spans: u32,
		) -> DispatchResult {
			let total_ring = T::RingCurrency::total_balance(&stash);
			let minimum_ring = T::RingCurrency::minimum_balance();
			let total_kton = T::KtonCurrency::total_balance(&stash);
			let minimum_kton = T::KtonCurrency::minimum_balance();
			let at_minimum = (total_ring == minimum_ring && total_kton <= minimum_kton)
				|| (total_kton == minimum_kton && total_ring <= minimum_ring);

			ensure!(at_minimum, <Error<T>>::FundedTarget);

			Self::kill_stash(&stash, num_slashing_spans)?;
			T::RingCurrency::remove_lock(STAKING_ID, &stash);
			T::KtonCurrency::remove_lock(STAKING_ID, &stash);

			Ok(())
		}

		/// Remove the given nominations from the calling validator.
		///
		/// Effects will be felt at the beginning of the next era.
		///
		/// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
		/// And, it can be only called when [`EraElectionStatus`] is `Closed`. The controller
		/// account should represent a validator.
		///
		/// - `who`: A list of nominator stash accounts who are nominating this validator which
		///   should no longer be nominating this validator.
		///
		/// Note: Making this call only makes sense if you first set the validator preferences to
		/// block any further nominations.
		#[pallet::weight(T::WeightInfo::kick(who.len() as u32))]
		pub fn kick(
			origin: OriginFor<T>,
			who: Vec<<T::Lookup as StaticLookup>::Source>,
		) -> DispatchResult {
			let controller = ensure_signed(origin)?;
			let ledger = Self::ledger(&controller).ok_or(<Error<T>>::NotController)?;
			let stash = &ledger.stash;

			for nom_stash in who
				.into_iter()
				.map(T::Lookup::lookup)
				.collect::<Result<Vec<AccountId<T>>, _>>()?
				.into_iter()
			{
				<Nominators<T>>::mutate(&nom_stash, |maybe_nom| {
					if let Some(ref mut nom) = maybe_nom {
						if let Some(pos) = nom.targets.iter().position(|v| v == stash) {
							nom.targets.swap_remove(pos);
							Self::deposit_event(Event::Kicked(nom_stash.clone(), stash.clone()));
						}
					}
				});
			}

			Ok(())
		}

		/// Update the various staking limits this pallet.
		///
		/// * `min_nominator_bond`: The minimum active bond needed to be a nominator.
		/// * `min_validator_bond`: The minimum active bond needed to be a validator.
		/// * `max_nominator_count`: The max number of users who can be a nominator at once.
		///   When set to `None`, no limit is enforced.
		/// * `max_validator_count`: The max number of users who can be a validator at once.
		///   When set to `None`, no limit is enforced.
		///
		/// Origin must be Root to call this function.
		///
		/// NOTE: Existing nominators and validators will not be affected by this update.
		/// to kick people under the new limits, `chill_other` should be called.
		#[pallet::weight(T::WeightInfo::set_staking_limits())]
		pub fn set_staking_limits(
			origin: OriginFor<T>,
			min_nominator_bond: RingBalance<T>,
			min_validator_bond: RingBalance<T>,
			max_nominator_count: Option<u32>,
			max_validator_count: Option<u32>,
			threshold: Option<Percent>,
		) -> DispatchResult {
			ensure_root(origin)?;

			<MinNominatorBond<T>>::set(min_nominator_bond);
			<MinValidatorBond<T>>::set(min_validator_bond);
			<MaxNominatorsCount<T>>::set(max_nominator_count);
			<MaxValidatorsCount<T>>::set(max_validator_count);
			<ChillThreshold<T>>::set(threshold);

			Ok(())
		}

		/// Declare a `controller` to stop participating as either a validator or nominator.
		///
		/// Effects will be felt at the beginning of the next era.
		///
		/// The dispatch origin for this call must be _Signed_, but can be called by anyone.
		///
		/// If the caller is the same as the controller being targeted, then no further checks are
		/// enforced, and this function behaves just like `chill`.
		///
		/// If the caller is different than the controller being targeted, the following conditions
		/// must be met:
		/// * A `ChillThreshold` must be set and checked which defines how close to the max
		///   nominators or validators we must reach before users can start chilling one-another.
		/// * A `MaxNominatorCount` and `MaxValidatorCount` must be set which is used to determine
		///   how close we are to the threshold.
		/// * A `MinNominatorBond` and `MinValidatorBond` must be set and checked, which determines
		///   if this is a person that should be chilled because they have not met the threshold
		///   bond required.
		///
		/// This can be helpful if bond requirements are updated, and we need to remove old users
		/// who do not satisfy these requirements.
		///
		// TODO: Maybe we can deprecate `chill` in the future.
		// https://github.com/paritytech/substrate/issues/9111
		#[pallet::weight(T::WeightInfo::chill_other())]
		pub fn chill_other(origin: OriginFor<T>, controller: T::AccountId) -> DispatchResult {
			// Anyone can call this function.
			let caller = ensure_signed(origin)?;
			let ledger = Self::ledger(&controller).ok_or(<Error<T>>::NotController)?;
			let stash = ledger.stash;

			// In order for one user to chill another user, the following conditions must be met:
			// * A `ChillThreshold` is set which defines how close to the max nominators or
			//   validators we must reach before users can start chilling one-another.
			// * A `MaxNominatorCount` and `MaxValidatorCount` which is used to determine how close
			//   we are to the threshold.
			// * A `MinNominatorBond` and `MinValidatorBond` which is the final condition checked to
			//   determine this is a person that should be chilled because they have not met the
			//   threshold bond required.
			//
			// Otherwise, if caller is the same as the controller, this is just like `chill`.
			if caller != controller {
				let threshold = <ChillThreshold<T>>::get().ok_or(<Error<T>>::CannotChillOther)?;
				let min_active_bond = if <Nominators<T>>::contains_key(&stash) {
					let max_nominator_count =
						<MaxNominatorsCount<T>>::get().ok_or(<Error<T>>::CannotChillOther)?;
					let current_nominator_count = <CounterForNominators<T>>::get();
					ensure!(
						threshold * max_nominator_count < current_nominator_count,
						<Error<T>>::CannotChillOther
					);

					<MinNominatorBond<T>>::get()
				} else if <Validators<T>>::contains_key(&stash) {
					let max_validator_count =
						<MaxValidatorsCount<T>>::get().ok_or(<Error<T>>::CannotChillOther)?;
					let current_validator_count = <CounterForValidators<T>>::get();
					ensure!(
						threshold * max_validator_count < current_validator_count,
						<Error<T>>::CannotChillOther
					);

					<MinValidatorBond<T>>::get()
				} else {
					Zero::zero()
				};

				ensure!(
					ledger.active_ring < min_active_bond,
					<Error<T>>::CannotChillOther
				);
			}

			Self::chill_stash(&stash);

			Ok(())
		}
	}
	impl<T: Config> Pallet<T> {
		pub fn account_id() -> AccountId<T> {
			T::PalletId::get().into_account()
		}

		/// Update the ledger while bonding ring and compute the *KTON* reward
		pub fn bond_ring(
			stash: &AccountId<T>,
			controller: &AccountId<T>,
			value: RingBalance<T>,
			promise_month: u8,
			mut ledger: StakingLedgerT<T>,
		) -> Result<(TsInMs, TsInMs), DispatchError> {
			let StakingLedger {
				active_ring,
				active_deposit_ring,
				deposit_items,
				active_kton,
				..
			} = &mut ledger;

			let start_time = T::UnixTime::now().as_millis().saturated_into::<TsInMs>();
			let mut expire_time = start_time;

			*active_ring = active_ring.saturating_add(value);

			// Last check: the new active amount of ledger must be more than ED.
			ensure!(
				*active_ring >= T::RingCurrency::minimum_balance()
					|| *active_kton >= T::KtonCurrency::minimum_balance(),
				<Error<T>>::InsufficientBond
			);

			// If stash promise to an extra-lock
			// there will be extra reward (*KTON*), which can also be used for staking
			if promise_month > 0 {
				expire_time += promise_month as TsInMs * MONTH_IN_MILLISECONDS;
				*active_deposit_ring += value;

				let kton_return = inflation::compute_kton_reward::<T>(value, promise_month);
				let kton_positive_imbalance =
					T::KtonCurrency::deposit_creating(&stash, kton_return);

				T::KtonReward::on_unbalanced(kton_positive_imbalance);
				deposit_items.push(TimeDepositItem {
					value,
					start_time,
					expire_time,
				});
			}

			Self::update_ledger(&controller, &mut ledger);

			Ok((start_time, expire_time))
		}

		/// Update the ledger while bonding controller with *KTON*
		pub fn bond_kton(
			controller: &AccountId<T>,
			value: KtonBalance<T>,
			mut ledger: StakingLedgerT<T>,
		) -> DispatchResult {
			ledger.active_kton = ledger.active_kton.saturating_add(value);

			// Last check: the new active amount of ledger must be more than ED.
			ensure!(
				ledger.active_ring >= T::RingCurrency::minimum_balance()
					|| ledger.active_kton >= T::KtonCurrency::minimum_balance(),
				<Error<T>>::InsufficientBond
			);

			Self::update_ledger(&controller, &mut ledger);

			Ok(())
		}

		/// Turn the expired deposit items into normal bond
		pub fn clear_mature_deposits(mut ledger: StakingLedgerT<T>) -> (StakingLedgerT<T>, bool) {
			let now = T::UnixTime::now().as_millis().saturated_into::<TsInMs>();
			let StakingLedger {
				stash,
				active_deposit_ring,
				deposit_items,
				..
			} = &mut ledger;
			let mut mutated = false;

			deposit_items.retain(|item| {
				if item.expire_time > now {
					true
				} else {
					mutated = true;
					*active_deposit_ring = active_deposit_ring.saturating_sub(item.value);

					false
				}
			});

			if mutated {
				Self::deposit_event(Event::DepositsClaimed(stash.to_owned()));
			}

			(ledger, mutated)
		}

		// power is a mixture of ring and kton
		// For *RING* power = ring_ratio * POWER_COUNT / 2
		// For *KTON* power = kton_ratio * POWER_COUNT / 2
		pub fn currency_to_power<S: TryInto<Balance>>(active: S, pool: S) -> Power {
			(Perquintill::from_rational(
				active.saturated_into::<Balance>(),
				pool.saturated_into::<Balance>().max(1),
			) * (T::TotalPower::get() as Balance / 2)) as _
		}

		/// The total power that can be slashed from a stash account as of right now.
		pub fn power_of(stash: &AccountId<T>) -> Power {
			// Weight note: consider making the stake accessible through stash.
			Self::bonded(stash)
				.and_then(Self::ledger)
				.map(|l| {
					Self::currency_to_power::<_>(l.active_ring, Self::ring_pool())
						+ Self::currency_to_power::<_>(l.active_kton, Self::kton_pool())
				})
				.unwrap_or_default()
		}

		darwinia_support::impl_rpc! {
			pub fn power_of_rpc(
				stash: impl sp_std::borrow::Borrow<AccountId<T>>,
			) -> RuntimeDispatchInfo<Power> {
				RuntimeDispatchInfo { power: Self::power_of(stash.borrow()) }
			}
		}

		pub fn stake_of(stash: &AccountId<T>) -> (RingBalance<T>, KtonBalance<T>) {
			// Weight note: consider making the stake accessible through stash.
			Self::bonded(stash)
				.and_then(Self::ledger)
				.map(|l| (l.active_ring, l.active_kton))
				.unwrap_or_default()
		}

		pub fn do_payout_stakers(
			validator_stash: AccountId<T>,
			era: EraIndex,
		) -> DispatchResultWithPostInfo {
			// Validate input data
			let current_era = <CurrentEra<T>>::get().ok_or(
				<Error<T>>::InvalidEraToReward
					.with_weight(T::WeightInfo::payout_stakers_alive_staked(0)),
			)?;
			ensure!(
				era <= current_era,
				<Error<T>>::InvalidEraToReward
					.with_weight(T::WeightInfo::payout_stakers_alive_staked(0))
			);
			let history_depth = Self::history_depth();
			ensure!(
				era >= current_era.saturating_sub(history_depth),
				<Error<T>>::InvalidEraToReward
					.with_weight(T::WeightInfo::payout_stakers_alive_staked(0))
			);

			// Note: if era has no reward to be claimed, era may be future. better not to update
			// `ledger.claimed_rewards` in this case.
			let era_payout = <ErasValidatorReward<T>>::get(&era).ok_or_else(|| {
				<Error<T>>::InvalidEraToReward
					.with_weight(T::WeightInfo::payout_stakers_alive_staked(0))
			})?;

			let controller = Self::bonded(&validator_stash).ok_or(
				<Error<T>>::NotStash.with_weight(T::WeightInfo::payout_stakers_alive_staked(0)),
			)?;
			let mut ledger =
				<Ledger<T>>::get(&controller).ok_or_else(|| <Error<T>>::NotController)?;

			ledger
				.claimed_rewards
				.retain(|&x| x >= current_era.saturating_sub(history_depth));
			match ledger.claimed_rewards.binary_search(&era) {
				Ok(_) => Err(<Error<T>>::AlreadyClaimed
					.with_weight(T::WeightInfo::payout_stakers_alive_staked(0)))?,
				Err(pos) => ledger.claimed_rewards.insert(pos, era),
			}

			let exposure = <ErasStakersClipped<T>>::get(&era, &ledger.stash);

			/* Input data seems good, no errors allowed after this point */

			<Ledger<T>>::insert(&controller, &ledger);

			// Get Era reward points. It has TOTAL and INDIVIDUAL
			// Find the fraction of the era reward that belongs to the validator
			// Take that fraction of the eras rewards to split to nominator and validator
			//
			// Then look at the validator, figure out the proportion of their reward
			// which goes to them and each of their nominators.

			let era_reward_points = <ErasRewardPoints<T>>::get(&era);
			let total_reward_points = era_reward_points.total;
			let validator_reward_points = era_reward_points
				.individual
				.get(&ledger.stash)
				.map(|points| *points)
				.unwrap_or_else(|| Zero::zero());

			// Nothing to do if they have no reward points.
			if validator_reward_points.is_zero() {
				return Ok(Some(T::WeightInfo::payout_stakers_alive_staked(0)).into());
			}

			// This is the fraction of the total reward that the validator and the
			// nominators will get.
			let validator_total_reward_part =
				Perbill::from_rational(validator_reward_points, total_reward_points);

			// This is how much validator + nominators are entitled to.
			let validator_total_payout = validator_total_reward_part * era_payout;

			let module_account = Self::account_id();

			ensure!(
				T::RingCurrency::usable_balance(&module_account) >= validator_total_payout,
				<Error<T>>::PayoutIns
			);

			let validator_prefs = Self::eras_validator_prefs(&era, &validator_stash);
			// Validator first gets a cut off the top.
			let validator_commission = validator_prefs.commission;
			let validator_commission_payout = validator_commission * validator_total_payout;

			let validator_leftover_payout = validator_total_payout - validator_commission_payout;
			// Now let's calculate how this is split to the validator.
			let validator_exposure_part =
				Perbill::from_rational(exposure.own_power, exposure.total_power);
			let validator_staking_payout = validator_exposure_part * validator_leftover_payout;

			// Due to the `payout * percent` there might be some losses
			let mut actual_payout = <RingPositiveImbalance<T>>::zero();

			// We can now make total validator payout:
			if let Some(imbalance) = Self::make_payout(
				&ledger.stash,
				validator_staking_payout + validator_commission_payout,
			) {
				let payout = imbalance.peek();

				actual_payout.subsume(imbalance);

				Self::deposit_event(Event::Reward(ledger.stash, payout));
			}

			// Track the number of payout ops to nominators. Note: `WeightInfo::payout_stakers_alive_staked`
			// always assumes at least a validator is paid out, so we do not need to count their payout op.
			let mut nominator_payout_count: u32 = 0;

			// Lets now calculate how this is split to the nominators.
			// Reward only the clipped exposures. Note this is not necessarily sorted.
			for nominator in exposure.others.iter() {
				let nominator_exposure_part =
					Perbill::from_rational(nominator.power, exposure.total_power);

				let nominator_reward: RingBalance<T> =
					nominator_exposure_part * validator_leftover_payout;
				// We can now make nominator payout:
				if let Some(imbalance) = Self::make_payout(&nominator.who, nominator_reward) {
					let payout = imbalance.peek();

					actual_payout.subsume(imbalance);

					// Note: this logic does not count payouts for `RewardDestination::None`.
					nominator_payout_count += 1;

					Self::deposit_event(Event::Reward(nominator.who.clone(), payout));
				}
			}

			T::RingCurrency::settle(
				&module_account,
				actual_payout,
				WithdrawReasons::all(),
				KeepAlive,
			)
			.map_err(|_| <Error<T>>::PayoutIns)?;

			debug_assert!(nominator_payout_count <= T::MaxNominatorRewardedPerValidator::get());
			Ok(Some(T::WeightInfo::payout_stakers_alive_staked(
				nominator_payout_count,
			))
			.into())
		}

		/// Update the ledger for a controller.
		///
		/// BE CAREFUL:
		/// 	This will also update the stash lock.
		/// 	DO NOT modify the locks' staking amount outside this function.
		pub fn update_ledger(controller: &AccountId<T>, ledger: &mut StakingLedgerT<T>) {
			let StakingLedger {
				active_ring,
				active_kton,
				ring_staking_lock,
				kton_staking_lock,
				..
			} = ledger;

			if *active_ring != ring_staking_lock.staking_amount {
				let origin_active_ring = ring_staking_lock.staking_amount;

				ring_staking_lock.staking_amount = *active_ring;

				<RingPool<T>>::mutate(|pool| {
					if origin_active_ring > *active_ring {
						*pool = pool.saturating_sub(origin_active_ring - *active_ring);
					} else {
						*pool = pool.saturating_add(*active_ring - origin_active_ring);
					}
				});

				T::RingCurrency::set_lock(
					STAKING_ID,
					&ledger.stash,
					LockFor::Staking(ledger.ring_staking_lock.clone()),
					WithdrawReasons::all(),
				);
			}

			if *active_kton != kton_staking_lock.staking_amount {
				let origin_active_kton = kton_staking_lock.staking_amount;

				kton_staking_lock.staking_amount = *active_kton;

				<KtonPool<T>>::mutate(|pool| {
					if origin_active_kton > *active_kton {
						*pool = pool.saturating_sub(origin_active_kton - *active_kton);
					} else {
						*pool = pool.saturating_add(*active_kton - origin_active_kton);
					}
				});

				T::KtonCurrency::set_lock(
					STAKING_ID,
					&ledger.stash,
					LockFor::Staking(ledger.kton_staking_lock.clone()),
					WithdrawReasons::all(),
				);
			}

			<Ledger<T>>::insert(controller, ledger);
		}

		/// Chill a stash account.
		pub fn chill_stash(stash: &AccountId<T>) {
			let chilled_as_validator = Self::do_remove_validator(stash);
			let chilled_as_nominator = Self::do_remove_nominator(stash);

			if chilled_as_validator || chilled_as_nominator {
				Self::deposit_event(<Event<T>>::Chilled(stash.clone()));
			}
		}

		/// Actually make a payment to a staker. This uses the currency's reward function
		/// to pay the right payee for the given staker account.
		pub fn make_payout(
			stash: &AccountId<T>,
			amount: RingBalance<T>,
		) -> Option<RingPositiveImbalance<T>> {
			let dest = Self::payee(stash);
			match dest {
				RewardDestination::Controller => Self::bonded(stash).and_then(|controller| {
					Some(T::RingCurrency::deposit_creating(&controller, amount))
				}),
				RewardDestination::Stash => {
					T::RingCurrency::deposit_into_existing(stash, amount).ok()
				}
				RewardDestination::Staked => Self::bonded(stash)
					.and_then(|c| Self::ledger(&c).map(|l| (c, l)))
					.and_then(|(c, mut l)| {
						let r = T::RingCurrency::deposit_into_existing(stash, amount).ok();

						if r.is_some() {
							l.active_ring += amount;

							Self::update_ledger(&c, &mut l);
						}

						r
					}),
				RewardDestination::Account(dest_account) => {
					Some(T::RingCurrency::deposit_creating(&dest_account, amount))
				}
				RewardDestination::None => None,
			}
		}

		/// Plan a new session potentially trigger a new era.
		pub fn new_session(
			session_index: SessionIndex,
			is_genesis: bool,
		) -> Option<Vec<AccountId<T>>> {
			if let Some(current_era) = Self::current_era() {
				// Initial era has been set.
				let current_era_start_session_index = Self::eras_start_session_index(current_era)
					.unwrap_or_else(|| {
						frame_support::print(
							"Error: start_session_index must be set for current_era",
						);
						0
					});

				let era_length = session_index
					.checked_sub(current_era_start_session_index)
					.unwrap_or(0); // Must never happen.

				match <ForceEra<T>>::get() {
					// Will be set to `NotForcing` again if a new era has been triggered.
					Forcing::ForceNew => (),
					// Short circuit to `try_trigger_new_era`.
					Forcing::ForceAlways => (),
					// Only go to `try_trigger_new_era` if deadline reached.
					Forcing::NotForcing if era_length >= T::SessionsPerEra::get() => (),
					_ => {
						// Either `Forcing::ForceNone`,
						// or `Forcing::NotForcing if era_length >= T::SessionsPerEra::get()`.
						return None;
					}
				}

				// New era.
				let maybe_new_era_validators = Self::try_trigger_new_era(session_index, is_genesis);
				if maybe_new_era_validators.is_some()
					&& matches!(<ForceEra<T>>::get(), Forcing::ForceNew)
				{
					<ForceEra<T>>::put(Forcing::NotForcing);
				}

				maybe_new_era_validators
			} else {
				// Set initial era.
				log!(debug, "Starting the first era.");
				Self::try_trigger_new_era(session_index, is_genesis)
			}
		}

		/// Start a session potentially starting an era.
		pub fn start_session(start_session: SessionIndex) {
			let next_active_era = Self::active_era().map(|e| e.index + 1).unwrap_or(0);
			// This is only `Some` when current era has already progressed to the next era, while the
			// active era is one behind (i.e. in the *last session of the active era*, or *first session
			// of the new current era*, depending on how you look at it).
			if let Some(next_active_era_start_session_index) =
				Self::eras_start_session_index(next_active_era)
			{
				if next_active_era_start_session_index == start_session {
					Self::start_era(start_session);
				} else if next_active_era_start_session_index < start_session {
					// This arm should never happen, but better handle it than to stall the staking
					// pallet.
					frame_support::print("Warning: A session appears to have been skipped.");
					Self::start_era(start_session);
				}
			}
		}

		/// End a session potentially ending an era.
		pub fn end_session(session_index: SessionIndex) {
			if let Some(active_era) = Self::active_era() {
				let next_active_era_start_session_index =
					Self::eras_start_session_index(active_era.index + 1).unwrap_or_else(|| {
						frame_support::print(
							"Error: start_session_index must be set for active_era + 1",
						);
						0
					});

				if next_active_era_start_session_index == session_index + 1 {
					Self::end_era(active_era, session_index);
				}
			}
		}

		/// * Increment `active_era.index`,
		/// * reset `active_era.start`,
		/// * update `BondedEras` and apply slashes.
		pub fn start_era(start_session: SessionIndex) {
			let active_era = <ActiveEra<T>>::mutate(|active_era| {
				let new_index = active_era.as_ref().map(|info| info.index + 1).unwrap_or(0);
				*active_era = Some(ActiveEraInfo {
					index: new_index,
					// Set new active era start in next `on_finalize`. To guarantee usage of `Time`
					start: None,
				});
				new_index
			});

			let bonding_duration = T::BondingDurationInEra::get();

			<BondedEras<T>>::mutate(|bonded| {
				bonded.push((active_era, start_session));

				if active_era > bonding_duration {
					let first_kept = active_era - bonding_duration;

					// Prune out everything that's from before the first-kept index.
					let n_to_prune = bonded
						.iter()
						.take_while(|&&(era_idx, _)| era_idx < first_kept)
						.count();

					// Kill slashing metadata.
					for (pruned_era, _) in bonded.drain(..n_to_prune) {
						slashing::clear_era_metadata::<T>(pruned_era);
					}

					if let Some(&(_, first_session)) = bonded.first() {
						T::SessionInterface::prune_historical_up_to(first_session);
					}
				}
			});

			Self::apply_unapplied_slashes(active_era);
		}

		/// Compute payout for era.
		pub fn end_era(active_era: ActiveEraInfo, _session_index: SessionIndex) {
			// Note: active_era_start can be None if end era is called during genesis config.
			if let Some(active_era_start) = active_era.start {
				let now = T::UnixTime::now().as_millis().saturated_into::<TsInMs>();
				let living_time = Self::living_time();
				let era_duration = now - active_era_start;

				let (validator_payout, max_payout) = inflation::compute_total_payout::<T>(
					era_duration,
					Self::living_time(),
					T::Cap::get().saturating_sub(T::RingCurrency::total_issuance()),
					<PayoutFraction<T>>::get(),
				);
				let rest = max_payout.saturating_sub(validator_payout);

				Self::deposit_event(Event::EraPayout(active_era.index, validator_payout, rest));

				<LivingTime<T>>::put(living_time + era_duration);
				// Set ending era reward.
				<ErasValidatorReward<T>>::insert(&active_era.index, validator_payout);
				T::RingCurrency::deposit_creating(&Self::account_id(), validator_payout);
				T::RingRewardRemainder::on_unbalanced(T::RingCurrency::issue(rest));
			}
		}

		/// Plan a new era.
		///
		/// * Bump the current era storage (which holds the latest planned era).
		/// * Store start session index for the new planned era.
		/// * Clean old era information.
		/// * Store staking information for the new planned era
		///
		/// Returns the new validator set.
		pub fn trigger_new_era(
			start_session_index: SessionIndex,
			exposures: Vec<(AccountId<T>, ExposureT<T>)>,
		) -> Vec<AccountId<T>> {
			// Increment or set current era.
			let new_planned_era = <CurrentEra<T>>::mutate(|s| {
				*s = Some(s.map(|s| s + 1).unwrap_or(0));
				s.unwrap()
			});
			<ErasStartSessionIndex<T>>::insert(&new_planned_era, &start_session_index);

			// Clean old era information.
			if let Some(old_era) = new_planned_era.checked_sub(Self::history_depth() + 1) {
				Self::clear_era_information(old_era);
			}

			// Set staking information for the new era.
			Self::store_stakers_info(exposures, new_planned_era)
		}

		/// Potentially plan a new era.
		///
		/// Get election result from `T::ElectionProvider`.
		/// In case election result has more than [`MinimumValidatorCount`] validator trigger a new era.
		///
		/// In case a new era is planned, the new validator set is returned.
		fn try_trigger_new_era(
			start_session_index: SessionIndex,
			is_genesis: bool,
		) -> Option<Vec<AccountId<T>>> {
			let (election_result, weight) = if is_genesis {
				T::GenesisElectionProvider::elect().map_err(|e| {
					log!(warn, "genesis election provider failed due to {:?}", e);

					Self::deposit_event(Event::StakingElectionFailed);
				})
			} else {
				T::ElectionProvider::elect().map_err(|e| {
					log!(warn, "election provider failed due to {:?}", e);

					Self::deposit_event(Event::StakingElectionFailed);
				})
			}
			.ok()?;

			<frame_system::Pallet<T>>::register_extra_weight_unchecked(
				weight,
				frame_support::weights::DispatchClass::Mandatory,
			);

			let exposures = Self::collect_exposures(election_result);

			if (exposures.len() as u32) < Self::minimum_validator_count().max(1) {
				// Session will panic if we ever return an empty validator set, thus max(1) ^^.
				match <CurrentEra<T>>::get() {
					Some(current_era) if current_era > 0 => log!(
						warn,
						"chain does not have enough staking candidates to operate for era {:?} ({} \
						elected, minimum is {})",
						<CurrentEra<T>>::get().unwrap_or(0),
						exposures.len(),
						Self::minimum_validator_count(),
					),
					None => {
						// The initial era is allowed to have no exposures.
						// In this case the SessionManager is expected to choose a sensible validator
						// set.
						// TODO: this should be simplified #8911
						<CurrentEra<T>>::put(0);
						<ErasStartSessionIndex<T>>::insert(0, &start_session_index);
					}
					_ => (),
				}

				Self::deposit_event(Event::StakingElectionFailed);

				return None;
			}

			Self::deposit_event(Event::StakingElection);

			Some(Self::trigger_new_era(start_session_index, exposures))
		}

		/// Process the output of the election.
		///
		/// Store staking information for the new planned era
		pub fn store_stakers_info(
			exposures: Vec<(AccountId<T>, ExposureT<T>)>,
			new_planned_era: EraIndex,
		) -> Vec<AccountId<T>> {
			let elected_stashes = exposures
				.iter()
				.cloned()
				.map(|(x, _)| x)
				.collect::<Vec<_>>();
			// Populate stakers, exposures, and the snapshot of validator prefs.
			let mut total_stake = 0;

			exposures.into_iter().for_each(|(stash, exposure)| {
				total_stake = total_stake.saturating_add(exposure.total_power);

				<ErasStakers<T>>::insert(new_planned_era, &stash, &exposure);

				let mut exposure_clipped = exposure;
				let clipped_max_len = T::MaxNominatorRewardedPerValidator::get() as usize;

				if exposure_clipped.others.len() > clipped_max_len {
					exposure_clipped
						.others
						.sort_by(|a, b| a.power.cmp(&b.power).reverse());
					exposure_clipped.others.truncate(clipped_max_len);
				}

				<ErasStakersClipped<T>>::insert(&new_planned_era, &stash, exposure_clipped);
			});

			// Insert current era staking information
			<ErasTotalStake<T>>::insert(&new_planned_era, total_stake);

			// Collect the pref of all winners
			for stash in &elected_stashes {
				let pref = Self::validators(stash);

				<ErasValidatorPrefs<T>>::insert(&new_planned_era, stash, pref);
			}

			if new_planned_era > 0 {
				log!(
					info,
					"new validator set of size {:?} has been processed for era {:?}",
					elected_stashes.len(),
					new_planned_era,
				);
			}

			elected_stashes
		}

		/// Consume a set of [`Supports`] from [`sp_npos_elections`] and collect them into a
		/// [`Exposure`].
		pub fn collect_exposures(
			supports: Supports<AccountId<T>>,
		) -> Vec<(AccountId<T>, ExposureT<T>)> {
			supports
				.into_iter()
				.map(|(validator, support)| {
					// Build `struct exposure` from `support`
					let mut own_ring_balance: RingBalance<T> = Zero::zero();
					let mut own_kton_balance: KtonBalance<T> = Zero::zero();
					let mut own_power = 0;
					let mut total_power = 0;
					let mut others = Vec::with_capacity(support.voters.len());

					support
						.voters
						.into_iter()
						.for_each(|(nominator, power_u128)| {
							// `T::TotalPower::get() == 1_000_000_000_u32`, will never overflow or get truncated; qed
							let power = power_u128 as _;
							let origin_power = Self::power_of(&nominator);
							let origin_power_u128 = origin_power as _;

							let (origin_ring_balance, origin_kton_balance) =
								Self::stake_of(&nominator);
							let ring_balance = if let Ok(ring_balance) =
								helpers_128bit::multiply_by_rational(
									origin_ring_balance.saturated_into(),
									power_u128,
									origin_power_u128,
								) {
								ring_balance.saturated_into()
							} else {
								log!(
									error,
									"[staking] Origin RING: {:?}, Weight: {:?}, Origin Weight: {:?}",
									origin_ring_balance,
									power_u128,
									origin_power_u128
								);
								Zero::zero()
							};
							let kton_balance = if let Ok(kton_balance) =
								helpers_128bit::multiply_by_rational(
									origin_kton_balance.saturated_into(),
									power_u128,
									origin_power_u128,
								) {
								kton_balance.saturated_into()
							} else {
								log!(
									error,
									"[staking] Origin KTON: {:?}, Weight: {:?}, Origin Weight: {:?}",
									origin_kton_balance,
									power_u128,
									origin_power_u128
								);
								Zero::zero()
							};

							if nominator == validator {
								own_ring_balance = own_ring_balance.saturating_add(ring_balance);
								own_kton_balance = own_kton_balance.saturating_add(kton_balance);
								own_power = own_power.saturating_add(power);
							} else {
								others.push(IndividualExposure {
									who: nominator,
									ring_balance,
									kton_balance,
									power,
								});
							}
							total_power = total_power.saturating_add(power);
						});

					let exposure = Exposure {
						own_ring_balance,
						own_kton_balance,
						own_power,
						total_power,
						others,
					};

					(validator, exposure)
				})
				.collect()
		}

		/// Remove all associated data of a stash account from the staking system.
		///
		/// Assumes storage is upgraded before calling.
		///
		/// This is called:
		/// - after a `withdraw_unbond()` call that frees all of a stash's bonded balance.
		/// - through `reap_stash()` if the balance has fallen to zero (through slashing).
		pub fn kill_stash(stash: &AccountId<T>, num_slashing_spans: u32) -> DispatchResult {
			let controller = <Bonded<T>>::get(stash).ok_or(<Error<T>>::NotStash)?;

			slashing::clear_stash_metadata::<T>(stash, num_slashing_spans)?;

			<Bonded<T>>::remove(stash);
			<Ledger<T>>::remove(&controller);

			<Payee<T>>::remove(stash);

			Self::do_remove_validator(stash);
			Self::do_remove_nominator(stash);

			<frame_system::Pallet<T>>::dec_consumers(stash);

			Ok(())
		}

		/// Clear all era information for given era.
		pub fn clear_era_information(era_index: EraIndex) {
			<ErasStakers<T>>::remove_prefix(era_index, None);
			<ErasStakersClipped<T>>::remove_prefix(era_index, None);
			<ErasValidatorPrefs<T>>::remove_prefix(era_index, None);
			<ErasValidatorReward<T>>::remove(era_index);
			<ErasRewardPoints<T>>::remove(era_index);
			<ErasTotalStake<T>>::remove(era_index);
			<ErasStartSessionIndex<T>>::remove(era_index);
		}

		/// Apply previously-unapplied slashes on the beginning of a new era, after a delay.
		pub fn apply_unapplied_slashes(active_era: EraIndex) {
			let slash_defer_duration = T::SlashDeferDuration::get();
			<Self as Store>::EarliestUnappliedSlash::mutate(|earliest| {
				if let Some(ref mut earliest) = earliest {
					let keep_from = active_era.saturating_sub(slash_defer_duration);
					for era in (*earliest)..keep_from {
						let era_slashes = <Self as Store>::UnappliedSlashes::take(&era);
						for slash in era_slashes {
							slashing::apply_slash::<T>(slash);
						}
					}

					*earliest = (*earliest).max(keep_from)
				}
			})
		}

		/// Add reward points to validators using their stash account ID.
		///
		/// Validators are keyed by stash account ID and must be in the current elected set.
		///
		/// For each element in the iterator the given number of points in u32 is added to the
		/// validator, thus duplicates are handled.
		///
		/// At the end of the era each the total payout will be distributed among validator
		/// relatively to their points.
		///
		/// COMPLEXITY: Complexity is `number_of_validator_to_reward x current_elected_len`.
		/// If you need to reward lots of validator consider using `reward_by_indices`.
		pub fn reward_by_ids(validators_points: impl IntoIterator<Item = (AccountId<T>, u32)>) {
			if let Some(active_era) = Self::active_era() {
				<ErasRewardPoints<T>>::mutate(active_era.index, |era_rewards| {
					for (validator, points) in validators_points.into_iter() {
						*era_rewards.individual.entry(validator).or_default() += points;
						era_rewards.total += points;
					}
				});
			}
		}

		/// Ensures that at the end of the current session there will be a new era.
		pub fn ensure_new_era() {
			match <ForceEra<T>>::get() {
				Forcing::ForceAlways | Forcing::ForceNew => (),
				_ => <ForceEra<T>>::put(Forcing::ForceNew),
			}
		}

		#[cfg(feature = "runtime-benchmarks")]
		pub fn add_era_stakers(
			current_era: EraIndex,
			controller: AccountId<T>,
			exposure: ExposureT<T>,
		) {
			<ErasStakers<T>>::insert(&current_era, &controller, &exposure);
		}

		#[cfg(feature = "runtime-benchmarks")]
		pub fn set_slash_reward_fraction(fraction: Perbill) {
			SlashRewardFraction::put(fraction);
		}

		/// Get all of the voters that are eligible for the npos election.
		///
		/// This will use all on-chain nominators, and all the validators will inject a self vote.
		///
		/// ### Slashing
		///
		/// All nominations that have been submitted before the last non-zero slash of the validator are
		/// auto-chilled.
		///
		/// Note that this is VERY expensive. Use with care.
		pub fn get_npos_voters() -> Vec<(AccountId<T>, VoteWeight, Vec<AccountId<T>>)> {
			let weight_of =
				|account_id: &AccountId<T>| -> VoteWeight { Self::power_of(account_id) as _ };
			let mut all_voters = Vec::new();

			for (validator, _) in <Validators<T>>::iter() {
				// Append self vote
				let self_vote = (
					validator.clone(),
					weight_of(&validator),
					vec![validator.clone()],
				);
				all_voters.push(self_vote);
			}

			// Collect all slashing spans into a BTreeMap for further queries.
			let slashing_spans = <SlashingSpans<T>>::iter().collect::<BTreeMap<_, _>>();

			for (nominator, nominations) in <Nominators<T>>::iter() {
				let Nominations {
					submitted_in,
					mut targets,
					suppressed: _,
				} = nominations;

				// Filter out nomination targets which were nominated before the most recent
				// slashing span.
				targets.retain(|stash| {
					slashing_spans
						.get(stash)
						.map_or(true, |spans| submitted_in >= spans.last_nonzero_slash())
				});

				if !targets.is_empty() {
					let vote_weight = weight_of(&nominator);
					all_voters.push((nominator, vote_weight, targets))
				}
			}

			all_voters
		}

		/// This is a very expensive function and result should be cached versus being called multiple times.
		pub fn get_npos_targets() -> Vec<AccountId<T>> {
			<Validators<T>>::iter().map(|(v, _)| v).collect::<Vec<_>>()
		}

		/// This function will add a nominator to the `Nominators` storage map,
		/// and keep track of the `CounterForNominators`.
		///
		/// If the nominator already exists, their nominations will be updated.
		pub fn do_add_nominator(who: &T::AccountId, nominations: Nominations<T::AccountId>) {
			if !<Nominators<T>>::contains_key(who) {
				<CounterForNominators<T>>::mutate(|x| x.saturating_inc())
			}

			<Nominators<T>>::insert(who, nominations);
		}

		/// This function will remove a nominator from the `Nominators` storage map,
		/// and keep track of the `CounterForNominators`.
		///
		/// Returns true if `who` was removed from `Nominators`, otherwise false.
		pub fn do_remove_nominator(who: &T::AccountId) -> bool {
			if <Nominators<T>>::contains_key(who) {
				<Nominators<T>>::remove(who);
				<CounterForNominators<T>>::mutate(|x| x.saturating_dec());

				true
			} else {
				false
			}
		}

		/// This function will add a validator to the `Validators` storage map,
		/// and keep track of the `CounterForValidators`.
		///
		/// If the validator already exists, their preferences will be updated.
		pub fn do_add_validator(who: &T::AccountId, prefs: ValidatorPrefs) {
			if !<Validators<T>>::contains_key(who) {
				<CounterForValidators<T>>::mutate(|x| x.saturating_inc())
			}

			<Validators<T>>::insert(who, prefs);
		}

		/// This function will remove a validator from the `Validators` storage map,
		/// and keep track of the `CounterForValidators`.
		///
		/// Returns true if `who` was removed from `Validators`, otherwise false.
		pub fn do_remove_validator(who: &T::AccountId) -> bool {
			if <Validators<T>>::contains_key(who) {
				<Validators<T>>::remove(who);
				<CounterForValidators<T>>::mutate(|x| x.saturating_dec());
				true
			} else {
				false
			}
		}
	}
	impl<T: Config> ElectionDataProvider<AccountId<T>, T::BlockNumber> for Pallet<T> {
		const MAXIMUM_VOTES_PER_VOTER: u32 = T::MAX_NOMINATIONS;

		fn desired_targets() -> data_provider::Result<(u32, Weight)> {
			Ok((
				Self::validator_count(),
				<T as frame_system::Config>::DbWeight::get().reads(1),
			))
		}

		fn voters(
			maybe_max_len: Option<usize>,
		) -> data_provider::Result<(Vec<(AccountId<T>, VoteWeight, Vec<AccountId<T>>)>, Weight)> {
			let nominator_count = <CounterForNominators<T>>::get();
			let validator_count = <CounterForValidators<T>>::get();
			let voter_count = nominator_count.saturating_add(validator_count) as usize;

			debug_assert!(
				<Nominators<T>>::iter().count() as u32 == <CounterForNominators<T>>::get()
			);
			debug_assert!(
				<Validators<T>>::iter().count() as u32 == <CounterForValidators<T>>::get()
			);

			if maybe_max_len.map_or(false, |max_len| voter_count > max_len) {
				return Err("Voter snapshot too big");
			}

			let slashing_span_count = <SlashingSpans<T>>::iter().count();
			let weight = T::WeightInfo::get_npos_voters(
				validator_count as u32,
				nominator_count as u32,
				slashing_span_count as u32,
			);
			Ok((Self::get_npos_voters(), weight))
		}

		fn targets(
			maybe_max_len: Option<usize>,
		) -> data_provider::Result<(Vec<AccountId<T>>, Weight)> {
			let target_count = <CounterForValidators<T>>::get() as usize;

			if maybe_max_len.map_or(false, |max_len| target_count > max_len) {
				return Err("Target snapshot too big");
			}

			let weight = <T as frame_system::Config>::DbWeight::get().reads(target_count as u64);
			Ok((Self::get_npos_targets(), weight))
		}

		fn next_election_prediction(now: T::BlockNumber) -> T::BlockNumber {
			let current_era = Self::current_era().unwrap_or(0);
			let current_session = Self::current_planned_session();
			let current_era_start_session_index =
				Self::eras_start_session_index(current_era).unwrap_or(0);
			let era_progress = current_session
				.saturating_sub(current_era_start_session_index)
				.min(T::SessionsPerEra::get());
			let until_this_session_end = T::NextNewSession::estimate_next_new_session(now)
				.0
				.unwrap_or_default()
				.saturating_sub(now);
			let session_length = T::NextNewSession::average_session_length();
			let sessions_left: T::BlockNumber = match <ForceEra<T>>::get() {
				Forcing::ForceNone => Bounded::max_value(),
				Forcing::ForceNew | Forcing::ForceAlways => Zero::zero(),
				Forcing::NotForcing if era_progress >= T::SessionsPerEra::get() => Zero::zero(),
				Forcing::NotForcing => T::SessionsPerEra::get()
					.saturating_sub(era_progress)
					// One session is computed in this_session_end.
					.saturating_sub(1)
					.into(),
			};

			now.saturating_add(
				until_this_session_end.saturating_add(sessions_left.saturating_mul(session_length)),
			)
		}

		#[cfg(feature = "runtime-benchmarks")]
		fn put_snapshot(
			voters: Vec<(AccountId<T>, VoteWeight, Vec<AccountId<T>>)>,
			targets: Vec<AccountId<T>>,
			target_stake: Option<VoteWeight>,
		) {
			use sp_std::convert::TryFrom;
			targets.into_iter().for_each(|v| {
				let stake: BalanceOf<T> = target_stake
					.and_then(|w| <BalanceOf<T>>::try_from(w).ok())
					.unwrap_or(<MinNominatorBond<T>>::get() * 100u32.into());
				<Bonded<T>>::insert(v.clone(), v.clone());
				<Ledger<T>>::insert(
					v.clone(),
					StakingLedger {
						stash: v.clone(),
						active: stake,
						total: stake,
						unlocking: vec![],
						claimed_rewards: vec![],
					},
				);
				Self::do_add_validator(
					&v,
					ValidatorPrefs {
						commission: Perbill::zero(),
						blocked: false,
					},
				);
			});

			voters.into_iter().for_each(|(v, s, t)| {
				let stake = <BalanceOf<T>>::try_from(s).unwrap_or_else(|_| {
					panic!("cannot convert a VoteWeight into BalanceOf, benchmark needs reconfiguring.")
				});
				<Bonded<T>>::insert(v.clone(), v.clone());
				<Ledger<T>>::insert(
					v.clone(),
					StakingLedger {
						stash: v.clone(),
						active: stake,
						total: stake,
						unlocking: vec![],
						claimed_rewards: vec![],
					},
				);
				Self::do_add_nominator(
					&v,
					Nominations {
						targets: t,
						submitted_in: 0,
						suppressed: false,
					},
				);
			});
		}
	}
	impl<T: Config> pallet_session::SessionManager<AccountId<T>> for Pallet<T> {
		fn new_session(new_index: SessionIndex) -> Option<Vec<AccountId<T>>> {
			log!(trace, "planning new session {}", new_index);

			<CurrentPlannedSession<T>>::put(new_index);

			Self::new_session(new_index, false)
		}
		fn new_session_genesis(new_index: SessionIndex) -> Option<Vec<AccountId<T>>> {
			log!(trace, "planning new session {} at genesis", new_index);

			<CurrentPlannedSession<T>>::put(new_index);

			Self::new_session(new_index, true)
		}
		fn start_session(start_index: SessionIndex) {
			log!(trace, "starting session {}", start_index);

			Self::start_session(start_index)
		}
		fn end_session(end_index: SessionIndex) {
			log!(trace, "ending session {}", end_index);

			Self::end_session(end_index)
		}
	}
	impl<T: Config> pallet_session::historical::SessionManager<AccountId<T>, ExposureT<T>>
		for Pallet<T>
	{
		fn new_session(new_index: SessionIndex) -> Option<Vec<(AccountId<T>, ExposureT<T>)>> {
			<Self as pallet_session::SessionManager<_>>::new_session(new_index).map(|validators| {
				let current_era = Self::current_era()
					// Must be some as a new era has been created.
					.unwrap_or(0);

				validators
					.into_iter()
					.map(|v| {
						let exposure = Self::eras_stakers(current_era, &v);
						(v, exposure)
					})
					.collect()
			})
		}
		fn new_session_genesis(
			new_index: SessionIndex,
		) -> Option<Vec<(AccountId<T>, ExposureT<T>)>> {
			<Self as pallet_session::SessionManager<_>>::new_session_genesis(new_index).map(
				|validators| {
					let current_era = Self::current_era()
						// Must be some as a new era has been created.
						.unwrap_or(0);

					validators
						.into_iter()
						.map(|v| {
							let exposure = Self::eras_stakers(current_era, &v);
							(v, exposure)
						})
						.collect()
				},
			)
		}
		fn start_session(start_index: SessionIndex) {
			<Self as pallet_session::SessionManager<_>>::start_session(start_index)
		}
		fn end_session(end_index: SessionIndex) {
			<Self as pallet_session::SessionManager<_>>::end_session(end_index)
		}
	}
	/// This is intended to be used with `FilterHistoricalOffences`.
	impl<T>
		OnOffenceHandler<AccountId<T>, pallet_session::historical::IdentificationTuple<T>, Weight>
		for Pallet<T>
	where
		T: Config
			+ pallet_session::Config<ValidatorId = AccountId<T>>
			+ pallet_session::historical::Config<
				FullIdentification = ExposureT<T>,
				FullIdentificationOf = ExposureOf<T>,
			>,
		T::SessionHandler: pallet_session::SessionHandler<AccountId<T>>,
		T::SessionManager: pallet_session::SessionManager<AccountId<T>>,
		T::ValidatorIdOf: Convert<AccountId<T>, Option<AccountId<T>>>,
	{
		fn on_offence(
			offenders: &[OffenceDetails<
				AccountId<T>,
				pallet_session::historical::IdentificationTuple<T>,
			>],
			slash_fraction: &[Perbill],
			slash_session: SessionIndex,
		) -> Weight {
			let reward_proportion = <SlashRewardFraction<T>>::get();
			let mut consumed_weight: Weight = 0;
			let mut add_db_reads_writes = |reads, writes| {
				consumed_weight += T::DbWeight::get().reads_writes(reads, writes);
			};

			let active_era = {
				let active_era = Self::active_era();
				add_db_reads_writes(1, 0);
				if active_era.is_none() {
					// This offence need not be re-submitted.
					return consumed_weight;
				}
				active_era
					.expect("value checked not to be `None`; qed")
					.index
			};
			let active_era_start_session_index = Self::eras_start_session_index(active_era)
				.unwrap_or_else(|| {
					frame_support::print("Error: start_session_index must be set for current_era");
					0
				});
			add_db_reads_writes(1, 0);

			let window_start = active_era.saturating_sub(T::BondingDurationInEra::get());

			// Fast path for active-era report - most likely.
			// `slash_session` cannot be in a future active era. It must be in `active_era` or before.
			let slash_era = if slash_session >= active_era_start_session_index {
				active_era
			} else {
				let eras = <BondedEras<T>>::get();
				add_db_reads_writes(1, 0);

				// Reverse because it's more likely to find reports from recent eras.
				match eras
					.iter()
					.rev()
					.filter(|&&(_, ref sesh)| sesh <= &slash_session)
					.next()
				{
					Some(&(ref slash_era, _)) => *slash_era,
					// Before bonding period. defensive - should be filtered out.
					None => return consumed_weight,
				}
			};

			<Self as Store>::EarliestUnappliedSlash::mutate(|earliest| {
				if earliest.is_none() {
					*earliest = Some(active_era)
				}
			});
			add_db_reads_writes(1, 1);

			let slash_defer_duration = T::SlashDeferDuration::get();

			let invulnerables = Self::invulnerables();
			add_db_reads_writes(1, 0);

			for (details, slash_fraction) in offenders.iter().zip(slash_fraction) {
				let (stash, exposure) = &details.offender;

				// Skip if the validator is invulnerable.
				if invulnerables.contains(stash) {
					continue;
				}

				let unapplied = slashing::compute_slash::<T>(slashing::SlashParams {
					stash,
					slash: *slash_fraction,
					exposure,
					slash_era,
					window_start,
					now: active_era,
					reward_proportion,
				});

				if let Some(mut unapplied) = unapplied {
					let nominators_len = unapplied.others.len() as u64;
					let reporters_len = details.reporters.len() as u64;

					{
						let upper_bound = 1 /* Validator/NominatorSlashInEra */ + 2 /* fetch_spans */;
						let rw = upper_bound + nominators_len * upper_bound;
						add_db_reads_writes(rw, rw);
					}
					unapplied.reporters = details.reporters.clone();
					if slash_defer_duration == 0 {
						// Apply right away.
						slashing::apply_slash::<T>(unapplied);
						{
							let slash_cost = (6, 5);
							let reward_cost = (2, 2);
							add_db_reads_writes(
								(1 + nominators_len) * slash_cost.0 + reward_cost.0 * reporters_len,
								(1 + nominators_len) * slash_cost.1 + reward_cost.1 * reporters_len,
							);
						}
					} else {
						// Defer to end of some `slash_defer_duration` from now.
						<Self as Store>::UnappliedSlashes::mutate(active_era, move |for_later| {
							for_later.push(unapplied)
						});
						add_db_reads_writes(1, 1);
					}
				} else {
					add_db_reads_writes(4 /* fetch_spans */, 5 /* kick_out_if_recent */)
				}
			}

			consumed_weight
		}
	}
	impl<T: Config> OnDepositRedeem<AccountId<T>, RingBalance<T>> for Pallet<T> {
		fn on_deposit_redeem(
			backing: &AccountId<T>,
			stash: &AccountId<T>,
			amount: RingBalance<T>,
			start_time: TsInMs,
			months: u8,
		) -> DispatchResult {
			// The timestamp unit is different between Ethereum and Darwinia
			// Converting from seconds to milliseconds
			let start_time = start_time * 1000;
			let promise_month = months.min(36);
			let expire_time = start_time + promise_month as TsInMs * MONTH_IN_MILLISECONDS;

			if let Some(controller) = Self::bonded(&stash) {
				let mut ledger = Self::ledger(&controller).ok_or(<Error<T>>::NotController)?;

				T::RingCurrency::transfer(&backing, &stash, amount, KeepAlive)?;

				let StakingLedger {
					active_ring,
					active_deposit_ring,
					deposit_items,
					..
				} = &mut ledger;

				*active_ring = active_ring.saturating_add(amount);
				*active_deposit_ring = active_deposit_ring.saturating_add(amount);
				deposit_items.push(TimeDepositItem {
					value: amount,
					start_time,
					expire_time,
				});

				Self::update_ledger(&controller, &mut ledger);
			} else {
				ensure!(
					!<Bonded<T>>::contains_key(&stash),
					<Error<T>>::AlreadyBonded
				);

				let controller = stash;

				ensure!(
					!<Ledger<T>>::contains_key(controller),
					<Error<T>>::AlreadyPaired
				);

				T::RingCurrency::transfer(&backing, &stash, amount, KeepAlive)?;

				<Bonded<T>>::insert(&stash, controller);
				<Payee<T>>::insert(&stash, RewardDestination::Stash);

				<frame_system::Pallet<T>>::inc_consumers(&stash)
					.map_err(|_| <Error<T>>::BadState)?;

				let mut ledger = StakingLedger {
					stash: stash.clone(),
					active_ring: amount,
					active_deposit_ring: amount,
					deposit_items: vec![TimeDepositItem {
						value: amount,
						start_time,
						expire_time,
					}],
					claimed_rewards: {
						let current_era = <CurrentEra<T>>::get().unwrap_or(0);
						let last_reward_era = current_era.saturating_sub(Self::history_depth());
						(last_reward_era..current_era).collect()
					},
					..Default::default()
				};

				Self::update_ledger(controller, &mut ledger);
			};

			Self::deposit_event(Event::BondRing(amount, start_time, expire_time));

			Ok(())
		}
	}
	/// Add reward points to block authors:
	/// * 20 points to the block producer for producing a (non-uncle) block in the relay chain,
	/// * 2 points to the block producer for each reference to a previously unreferenced uncle, and
	/// * 1 point to the producer of each referenced uncle block.
	impl<T> pallet_authorship::EventHandler<AccountId<T>, T::BlockNumber> for Pallet<T>
	where
		T: Config + pallet_authorship::Config + pallet_session::Config,
	{
		fn note_author(author: AccountId<T>) {
			Self::reward_by_ids(vec![(author, 20)]);
		}
		fn note_uncle(author: AccountId<T>, _age: T::BlockNumber) {
			Self::reward_by_ids(vec![
				(<pallet_authorship::Pallet<T>>::author(), 2),
				(author, 1),
			]);
		}
	}

	/// Means for interacting with a specialized version of the `session` trait.
	///
	/// This is needed because `Staking` sets the `ValidatorIdOf` of the `pallet_session::Config`
	pub trait SessionInterface<AccountId>: frame_system::Config {
		/// Disable a given validator by stash ID.
		///
		/// Returns `true` if new era should be forced at the end of this session.
		/// This allows preventing a situation where there is too many validators
		/// disabled and block production stalls.
		fn disable_validator(validator: &AccountId) -> Result<bool, ()>;
		/// Get the validators from session.
		fn validators() -> Vec<AccountId>;
		/// Prune historical session tries up to but not including the given index.
		fn prune_historical_up_to(up_to: SessionIndex);
	}
	impl<T: Config> SessionInterface<AccountId<T>> for T
	where
		T: pallet_session::Config<ValidatorId = AccountId<T>>,
		T: pallet_session::historical::Config<
			FullIdentification = Exposure<AccountId<T>, RingBalance<T>, KtonBalance<T>>,
			FullIdentificationOf = ExposureOf<T>,
		>,
		T::SessionHandler: pallet_session::SessionHandler<AccountId<T>>,
		T::SessionManager: pallet_session::SessionManager<AccountId<T>>,
		T::ValidatorIdOf: Convert<AccountId<T>, Option<AccountId<T>>>,
	{
		fn disable_validator(validator: &AccountId<T>) -> Result<bool, ()> {
			<pallet_session::Pallet<T>>::disable(validator)
		}

		fn validators() -> Vec<AccountId<T>> {
			<pallet_session::Pallet<T>>::validators()
		}

		fn prune_historical_up_to(up_to: SessionIndex) {
			<pallet_session::historical::Pallet<T>>::prune_up_to(up_to);
		}
	}

	/// A typed conversion from stash account ID to the active exposure of nominators
	/// on that account.
	///
	/// Active exposure is the exposure of the validator set currently validating, i.e. in
	/// `active_era`. It can differ from the latest planned exposure in `current_era`.
	pub struct ExposureOf<T>(PhantomData<T>);
	impl<T: Config> Convert<AccountId<T>, Option<ExposureT<T>>> for ExposureOf<T> {
		fn convert(validator: AccountId<T>) -> Option<ExposureT<T>> {
			<Pallet<T>>::active_era()
				.map(|active_era| <Pallet<T>>::eras_stakers(active_era.index, &validator))
		}
	}
	/// A snapshot of the stake backing a single validator in the system.
	#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Encode, Decode, Default, RuntimeDebug)]
	pub struct Exposure<AccountId, RingBalance, KtonBalance>
	where
		RingBalance: HasCompact,
		KtonBalance: HasCompact,
	{
		/// The validator's own stash that is exposed.
		#[codec(compact)]
		pub own_ring_balance: RingBalance,
		#[codec(compact)]
		pub own_kton_balance: KtonBalance,
		pub own_power: Power,
		/// The total balance backing this validator.
		pub total_power: Power,
		/// The portions of nominators stashes that are exposed.
		pub others: Vec<IndividualExposure<AccountId, RingBalance, KtonBalance>>,
	}
	/// The amount of exposure (to slashing) than an individual nominator has.
	#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Encode, Decode, RuntimeDebug)]
	pub struct IndividualExposure<AccountId, RingBalance, KtonBalance>
	where
		RingBalance: HasCompact,
		KtonBalance: HasCompact,
	{
		/// The stash account of the nominator in question.
		pub who: AccountId,
		/// Amount of funds exposed.
		#[codec(compact)]
		pub ring_balance: RingBalance,
		#[codec(compact)]
		pub kton_balance: KtonBalance,
		pub power: Power,
	}

	/// Information regarding the active era (era in used in session).
	#[derive(Encode, Decode, RuntimeDebug)]
	pub struct ActiveEraInfo {
		/// Index of era.
		pub index: EraIndex,
		/// Moment of start expressed as millisecond from `$UNIX_EPOCH`.
		///
		/// Start can be none if start hasn't been set for the era yet,
		/// Start is set on the first on_finalize of the era to guarantee usage of `Time`.
		pub start: Option<u64>,
	}

	/// The ledger of a (bonded) stash.
	#[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug)]
	pub struct StakingLedger<AccountId, RingBalance, KtonBalance, BlockNumber>
	where
		RingBalance: HasCompact,
		KtonBalance: HasCompact,
	{
		/// The stash account whose balance is actually locked and at stake.
		pub stash: AccountId,

		/// The total amount of the stash's *RING* that will be at stake in any forthcoming
		/// rounds.
		#[codec(compact)]
		pub active_ring: RingBalance,
		/// active time-deposit ring
		#[codec(compact)]
		pub active_deposit_ring: RingBalance,

		/// The total amount of the stash's *KTON* that will be at stake in any forthcoming
		/// rounds.
		#[codec(compact)]
		pub active_kton: KtonBalance,

		/// If you deposit *RING* for a minimum period,
		/// you can get *KTON* as bonus which can also be used for staking.
		pub deposit_items: Vec<TimeDepositItem<RingBalance>>,

		/// The staking lock on *RING* balance, use for updating darwinia balance pallet's lock
		pub ring_staking_lock: StakingLock<RingBalance, BlockNumber>,
		/// The staking lock on *KTON* balance, use for updating darwinia balance pallet's lock
		pub kton_staking_lock: StakingLock<KtonBalance, BlockNumber>,

		/// List of eras for which the stakers behind a validator have claimed rewards. Only updated
		/// for validators.
		pub claimed_rewards: Vec<EraIndex>,
	}
	impl<AccountId, RingBalance, KtonBalance, BlockNumber>
		StakingLedger<AccountId, RingBalance, KtonBalance, BlockNumber>
	where
		RingBalance: Copy + AtLeast32BitUnsigned + Saturating,
		KtonBalance: Copy + AtLeast32BitUnsigned + Saturating,
		BlockNumber: Copy + PartialOrd,
		TsInMs: PartialOrd,
	{
		pub fn ring_locked_amount_at(&self, at: BlockNumber) -> RingBalance {
			self.ring_staking_lock.locked_amount(at)
		}

		pub fn kton_locked_amount_at(&self, at: BlockNumber) -> KtonBalance {
			self.kton_staking_lock.locked_amount(at)
		}

		/// Re-bond funds that were scheduled for unlocking.
		fn rebond(&mut self, plan_to_rebond_ring: RingBalance, plan_to_rebond_kton: KtonBalance) {
			fn update<Balance, _M>(
				bonded: &mut Balance,
				lock: &mut StakingLock<Balance, _M>,
				plan_to_rebond: Balance,
			) where
				Balance: Copy + AtLeast32BitUnsigned + Saturating,
			{
				let mut rebonded = Balance::zero();

				while let Some(Unbonding { amount, .. }) = lock.unbondings.as_mut().last_mut() {
					let new_rebonded = rebonded.saturating_add(*amount);

					if new_rebonded <= plan_to_rebond {
						rebonded = new_rebonded;
						*bonded = bonded.saturating_add(*amount);

						lock.unbondings.remove(lock.unbondings.len() - 1);
					} else {
						let diff = plan_to_rebond.saturating_sub(rebonded);

						rebonded = rebonded.saturating_add(diff);
						*bonded = bonded.saturating_add(diff);
						*amount = amount.saturating_sub(diff);
					}

					if rebonded >= plan_to_rebond {
						break;
					}
				}
			}

			update(
				&mut self.active_ring,
				&mut self.ring_staking_lock,
				plan_to_rebond_ring,
			);
			update(
				&mut self.active_kton,
				&mut self.kton_staking_lock,
				plan_to_rebond_kton,
			);
		}

		/// Slash the validator for a given amount of balance. This can grow the value
		/// of the slash in the case that the validator has less than `minimum_balance`
		/// active funds. Returns the amount of funds actually slashed.
		///
		/// Slashes from `active` funds first, and then `unlocking`, starting with the
		/// chunks that are closest to unlocking.
		pub fn slash(
			&mut self,
			slash_ring: RingBalance,
			slash_kton: KtonBalance,
			bn: BlockNumber,
			ts: TsInMs,
		) -> (RingBalance, KtonBalance) {
			let slash_out_of = |active_ring: &mut RingBalance,
			                    active_deposit_ring: &mut RingBalance,
			                    deposit_item: &mut Vec<TimeDepositItem<RingBalance>>,
			                    active_kton: &mut KtonBalance,
			                    slash_ring: &mut RingBalance,
			                    slash_kton: &mut KtonBalance| {
				let slashable_active_ring = (*slash_ring).min(*active_ring);
				let slashable_active_kton = (*slash_kton).min(*active_kton);

				if !slashable_active_ring.is_zero() {
					let slashable_normal_ring = *active_ring - *active_deposit_ring;
					if let Some(mut slashable_deposit_ring) =
						slashable_active_ring.checked_sub(&slashable_normal_ring)
					{
						*active_deposit_ring -= slashable_deposit_ring;

						deposit_item.drain_filter(|item| {
							if ts >= item.expire_time {
								true
							} else {
								if slashable_deposit_ring.is_zero() {
									false
								} else {
									if let Some(new_slashable_deposit_ring) =
										slashable_deposit_ring.checked_sub(&item.value)
									{
										slashable_deposit_ring = new_slashable_deposit_ring;
										true
									} else {
										item.value -=
											mem::replace(&mut slashable_deposit_ring, Zero::zero());
										false
									}
								}
							}
						});
					}

					*active_ring -= slashable_active_ring;
					*slash_ring -= slashable_active_ring;
				}

				if !slashable_active_kton.is_zero() {
					*active_kton -= slashable_active_kton;
					*slash_kton -= slashable_active_kton;
				}
			};

			let (mut apply_slash_ring, mut apply_slash_kton) = (slash_ring, slash_kton);
			let StakingLedger {
				active_ring,
				active_deposit_ring,
				deposit_items,
				active_kton,
				ring_staking_lock,
				kton_staking_lock,
				..
			} = self;

			slash_out_of(
				active_ring,
				active_deposit_ring,
				deposit_items,
				active_kton,
				&mut apply_slash_ring,
				&mut apply_slash_kton,
			);

			if !apply_slash_ring.is_zero() {
				// `WeakBoundedVec` not support `drain_filter` yet
				let mut unbondings = mem::take(&mut ring_staking_lock.unbondings).into_inner();

				unbondings.drain_filter(|lock| {
					if bn >= lock.until {
						true
					} else {
						if apply_slash_ring.is_zero() {
							false
						} else {
							if apply_slash_ring >= lock.amount {
								apply_slash_ring -= lock.amount;
								true
							} else {
								lock.amount -= mem::replace(&mut apply_slash_ring, Zero::zero());
								false
							}
						}
					}
				});

				ring_staking_lock.unbondings =
					WeakBoundedVec::force_from(unbondings, Some("Staking Update Locks"));
			}
			if !apply_slash_kton.is_zero() {
				// `WeakBoundedVec` not support `drain_filter` yet
				let mut unbondings = mem::take(&mut kton_staking_lock.unbondings).into_inner();

				unbondings.drain_filter(|lock| {
					if bn >= lock.until {
						true
					} else {
						if apply_slash_kton.is_zero() {
							false
						} else {
							if apply_slash_kton > lock.amount {
								apply_slash_kton -= lock.amount;

								true
							} else {
								lock.amount -= mem::replace(&mut apply_slash_kton, Zero::zero());
								false
							}
						}
					}
				});

				kton_staking_lock.unbondings =
					WeakBoundedVec::force_from(unbondings, Some("Staking Update Locks"));
			}

			(slash_ring - apply_slash_ring, slash_kton - apply_slash_kton)
		}
	}
	/// The *RING* under deposit.
	#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
	pub struct TimeDepositItem<RingBalance: HasCompact> {
		#[codec(compact)]
		pub value: RingBalance,
		#[codec(compact)]
		pub start_time: TsInMs,
		#[codec(compact)]
		pub expire_time: TsInMs,
	}

	/// A destination account for payment.
	#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
	pub enum RewardDestination<AccountId> {
		/// Pay into the stash account, increasing the amount at stake accordingly.
		Staked,
		/// Pay into the stash account, not increasing the amount at stake.
		Stash,
		/// Pay into the controller account.
		Controller,
		/// Pay into a specified account.
		Account(AccountId),
		/// Receive no reward.
		None,
	}
	impl<AccountId> Default for RewardDestination<AccountId> {
		fn default() -> Self {
			RewardDestination::Staked
		}
	}

	/// Preference of what happens regarding validation.
	#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
	pub struct ValidatorPrefs {
		/// Reward that validator takes up-front; only the rest is split between themselves and
		/// nominators.
		#[codec(compact)]
		pub commission: Perbill,
		/// Whether or not this validator is accepting more nominations. If `true`, then no nominator
		/// who is not already nominating this validator may nominate them. By default, validators
		/// are accepting nominations.
		pub blocked: bool,
	}
	impl Default for ValidatorPrefs {
		fn default() -> Self {
			ValidatorPrefs {
				commission: Perbill::zero(),
				blocked: false,
			}
		}
	}

	/// A record of the nominations made by a specific account.
	#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
	pub struct Nominations<AccountId> {
		/// The targets of nomination.
		pub targets: Vec<AccountId>,
		/// The era the nominations were submitted.
		///
		/// Except for initial nominations which are considered submitted at era 0.
		pub submitted_in: EraIndex,
		/// Whether the nominations have been suppressed. This can happen due to slashing of the
		/// validators, or other events that might invalidate the nomination.
		///
		/// NOTE: this for future proofing and is thus far not used.
		pub suppressed: bool,
	}

	/// Reward points of an era. Used to split era total payout between validators.
	///
	/// This points will be used to reward validators and their respective nominators.
	#[derive(PartialEq, Encode, Decode, Default, Debug)]
	pub struct EraRewardPoints<AccountId: Ord> {
		/// Total number of points. Equals the sum of reward points for each validator.
		pub total: RewardPoint,
		/// The reward points earned by a given validator.
		pub individual: BTreeMap<AccountId, RewardPoint>,
	}

	/// Mode of era-forcing.
	#[derive(Copy, Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug)]
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	pub enum Forcing {
		/// Not forcing anything - just let whatever happen.
		NotForcing,
		/// Force a new era, then reset to `NotForcing` as soon as it is done.
		/// Note that this will force to trigger an election until a new era is triggered, if the
		/// election failed, the next session end will trigger a new election again, until success.
		ForceNew,
		/// Avoid a new era indefinitely.
		ForceNone,
		/// Force a new era at the end of all sessions indefinitely.
		ForceAlways,
	}
	impl Default for Forcing {
		fn default() -> Self {
			Forcing::NotForcing
		}
	}

	/// A pending slash record. The value of the slash has been computed but not applied yet,
	/// rather deferred for several eras.
	#[derive(Encode, Decode, Default, RuntimeDebug)]
	pub struct UnappliedSlash<AccountId, RingBalance, KtonBalance> {
		/// The stash ID of the offending validator.
		pub validator: AccountId,
		/// The validator's own slash.
		pub own: slashing::RK<RingBalance, KtonBalance>,
		/// All other slashed stakers and amounts.
		pub others: Vec<(AccountId, slashing::RK<RingBalance, KtonBalance>)>,
		/// Reporters of the offence; bounty payout recipients.
		pub reporters: Vec<AccountId>,
		/// The amount of payout.
		pub payout: slashing::RK<RingBalance, KtonBalance>,
	}

	// A value placed in storage that represents the current version of the Staking storage. This value
	// is used by the `on_runtime_upgrade` logic to determine whether we run storage migration logic.
	// This should match directly with the semantic versions of the Rust crate.
	#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug)]
	pub enum Releases {
		V1_0_0Ancient,
		V2_0_0,
		V3_0_0,
		V4_0_0,
		V5_0_0, // blockable validators.
		V6_0_0, // removal of all storage associated with offchain phragmen.
		V7_0_0, // keep track of number of nominators / validators in map
	}
	impl Default for Releases {
		fn default() -> Self {
			Releases::V7_0_0
		}
	}

	/// Indicates the initial status of the staker.
	#[derive(RuntimeDebug)]
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	pub enum StakerStatus<AccountId> {
		/// Chilling.
		Idle,
		/// Declared desire in validating or already participating in it.
		Validator,
		/// Nominating for a group of other stakers.
		Nominator(Vec<AccountId>),
	}

	/// To unify *RING* and *KTON* balances.
	#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
	pub enum StakingBalance<RingBalance, KtonBalance>
	where
		RingBalance: HasCompact,
		KtonBalance: HasCompact,
	{
		RingBalance(RingBalance),
		KtonBalance(KtonBalance),
	}
	impl<RingBalance, KtonBalance> Default for StakingBalance<RingBalance, KtonBalance>
	where
		RingBalance: Zero + HasCompact,
		KtonBalance: Zero + HasCompact,
	{
		fn default() -> Self {
			StakingBalance::RingBalance(Zero::zero())
		}
	}

	/// A `Convert` implementation that finds the stash of the given controller account,
	/// if any.
	pub struct StashOf<T>(PhantomData<T>);
	impl<T: Config> Convert<AccountId<T>, Option<AccountId<T>>> for StashOf<T> {
		fn convert(controller: AccountId<T>) -> Option<AccountId<T>> {
			<Pallet<T>>::ledger(&controller).map(|l| l.stash)
		}
	}

	/// Filter historical offences out and only allow those from the bonding period.
	pub struct FilterHistoricalOffences<T, R> {
		_inner: PhantomData<(T, R)>,
	}
	impl<T, Reporter, Offender, R, O> ReportOffence<Reporter, Offender, O>
		for FilterHistoricalOffences<Pallet<T>, R>
	where
		T: Config,
		R: ReportOffence<Reporter, Offender, O>,
		O: Offence<Offender>,
	{
		fn report_offence(reporters: Vec<Reporter>, offence: O) -> Result<(), OffenceError> {
			// Disallow any slashing from before the current bonding period.
			let offence_session = offence.session_index();
			let bonded_eras = <BondedEras<T>>::get();

			if bonded_eras
				.first()
				.filter(|(_, start)| offence_session >= *start)
				.is_some()
			{
				R::report_offence(reporters, offence)
			} else {
				<Pallet<T>>::deposit_event(Event::OldSlashingReportDiscarded(offence_session));
				Ok(())
			}
		}

		fn is_known_offence(offenders: &[Offender], time_slot: &O::TimeSlot) -> bool {
			R::is_known_offence(offenders, time_slot)
		}
	}

	/// Check that list is sorted and has no duplicates.
	fn is_sorted_and_unique(list: &Vec<u32>) -> bool {
		list.windows(2).all(|w| w[0] < w[1])
	}
}
pub use pallet::*;

pub mod migration {
	// --- paritytech ---
	use frame_support::{traits::Get, weights::Weight};
	use sp_runtime::traits::Zero;
	// --- darwinia-network ---
	use crate::*;

	pub fn pre_migrate<T: Config>() -> Result<(), &'static str> {
		assert!(
			<CounterForValidators<T>>::get().is_zero(),
			"CounterForValidators already set."
		);
		assert!(
			<CounterForNominators<T>>::get().is_zero(),
			"CounterForNominators already set."
		);
		assert!(<StorageVersion<T>>::get() == Releases::V6_0_0);
		Ok(())
	}

	pub fn migrate<T: Config>() -> Weight {
		log!(info, "Migrating staking to Releases::V7_0_0");
		let validator_count = <Validators<T>>::iter().count() as u32;
		let nominator_count = <Nominators<T>>::iter().count() as u32;

		<CounterForValidators<T>>::put(validator_count);
		<CounterForNominators<T>>::put(nominator_count);

		<StorageVersion<T>>::put(Releases::V7_0_0);
		log!(info, "Completed staking migration to Releases::V7_0_0");

		T::DbWeight::get().reads_writes(validator_count.saturating_add(nominator_count).into(), 2)
	}
}
