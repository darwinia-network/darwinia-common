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
//! 	pub struct Pallet<T: Config> for enum Call where origin: T::Origin {
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
//! - [Balances](../pallet_balances/index.html): Used to manage values at stake.
//! - [Session](../pallet_session/index.html): Used to manage sessions. Also, a list of new
//!   validators is stored in the Session pallet's `Validators` at the end of each era.

#![cfg_attr(not(feature = "std"), no_std)]
#![feature(drain_filter)]

pub mod weights;
pub use weights::WeightInfo;

pub mod slashing;

mod types {
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
	// pub type StakingBalanceT<T> = StakingBalance<RingBalance<T>, KtonBalance<T>>;
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
	use core::mem;
	// --- crates.io ---
	use codec::{Decode, Encode, HasCompact};
	#[cfg(feature = "std")]
	use serde::{Deserialize, Serialize};
	// --- paritytech ---
	use frame_election_provider_support::ElectionProvider;
	use frame_support::{
		pallet_prelude::*,
		traits::{Currency, EstimateNextNewSession, OnUnbalanced, UnixTime},
		PalletId, WeakBoundedVec,
	};
	use frame_system::{offchain::SendTransactionTypes, pallet_prelude::*};
	use sp_runtime::{
		traits::{AtLeast32BitUnsigned, Convert, Saturating, StaticLookup, Zero},
		Perbill,
	};
	use sp_staking::SessionIndex;
	use sp_std::collections::btree_map::BTreeMap;
	// --- darwinia-network ---
	use crate::*;
	use darwinia_support::balance::*;

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

		/// Darwinia's hard cap default 10_000_000_000 * 10^9
		#[pallet::constant]
		type Cap: Get<RingBalance<Self>>;
		/// Darwinia's staking vote default 1_000_000_000
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
		/// [era_index, validator_payout, remainder]
		EraPayout(EraIndex, RingBalance<T>, RingBalance<T>),

		/// The staker has been rewarded by this amount. [stash, amount]
		Reward(AccountId<T>, RingBalance<T>),

		/// One validator (and its nominators) has been slashed by the given amount.
		/// [validator, amount, amount]
		Slash(AccountId<T>, RingBalance<T>, KtonBalance<T>),
		/// An old slashing report from a prior era was discarded because it could
		/// not be processed. [session_index]
		OldSlashingReportDiscarded(SessionIndex),

		/// A new set of stakers was elected.
		StakingElection,

		/// An account has bonded this amount. [amount, start, end]
		///
		/// NOTE: This event is only emitted when funds are bonded via a dispatchable. Notably,
		/// it will not be emitted for staking rewards when they are added to stake.
		BondRing(RingBalance<T>, TsInMs, TsInMs),
		/// An account has bonded this amount. [amount, start, end]
		///
		/// NOTE: This event is only emitted when funds are bonded via a dispatchable. Notably,
		/// it will not be emitted for staking rewards when they are added to stake.
		BondKton(KtonBalance<T>),

		/// An account has unbonded this amount. [amount, now]
		UnbondRing(RingBalance<T>, BlockNumberFor<T>),
		/// An account has unbonded this amount. [amount, now]
		UnbondKton(KtonBalance<T>, BlockNumberFor<T>),

		/// A nominator has been kicked from a validator. \[nominator, stash\]
		Kicked(AccountId<T>, AccountId<T>),

		/// Someone claimed his deposits. [stash]
		DepositsClaimed(AccountId<T>),
		/// Someone claimed his deposits with some *KTON*s punishment. [stash, forfeit]
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
		/// Can not bond with value less than minimum balance.
		InsufficientValue,
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
	pub(crate) type HistoryDepth<T> = StorageValue<_, u32, ValueQuery, HistoryDepthOnEmpty>;
	#[pallet::type_value]
	pub(crate) fn HistoryDepthOnEmpty() -> u32 {
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
	#[pallet::storage]
	#[pallet::getter(fn validators)]
	pub type Validators<T: Config> =
		StorageMap<_, Twox64Concat, AccountId<T>, ValidatorPrefs, ValueQuery>;

	/// The map from nominator stash key to the set of stash keys of all validators to nominate.
	#[pallet::storage]
	#[pallet::getter(fn nominators)]
	pub type Nominators<T: Config> =
		StorageMap<_, Twox64Concat, AccountId<T>, Nominations<AccountId<T>>>;

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
	pub type ErasTotalStake<T: Config> =
		StorageMap<_, Twox64Concat, EraIndex, RingBalance<T>, ValueQuery>;

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
	pub(crate) type BondedEras<T: Config> =
		StorageValue<_, Vec<(EraIndex, SessionIndex)>, ValueQuery>;

	/// All slashing events on validators, mapped by era to the highest slash proportion
	/// and slash value of the era.
	#[pallet::storage]
	pub(crate) type ValidatorSlashInEra<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		EraIndex,
		Twox64Concat,
		AccountId<T>,
		(Perbill, slashing::RKT<T>),
	>;

	/// All slashing events on nominators, mapped by era to the highest slash value of the era.
	#[pallet::storage]
	pub(crate) type NominatorSlashInEra<T: Config> =
		StorageDoubleMap<_, Twox64Concat, EraIndex, Twox64Concat, AccountId<T>, slashing::RKT<T>>;

	/// Slashing spans for stash accounts.
	#[pallet::storage]
	pub(crate) type SlashingSpans<T: Config> =
		StorageMap<_, Twox64Concat, AccountId<T>, slashing::SlashingSpans>;

	/// Records information about the maximum slash of a stash within a slashing span,
	/// as well as how much reward has been paid out.
	#[pallet::storage]
	pub(crate) type SpanSlash<T: Config> = StorageMap<
		_,
		Twox64Concat,
		(AccountId<T>, slashing::SpanIndex),
		slashing::SpanRecord<RingBalance<T>, KtonBalance<T>>,
		ValueQuery,
	>;

	/// The earliest era for which we have a pending, unapplied slash.
	#[pallet::storage]
	pub(crate) type EarliestUnappliedSlash<T> = StorageValue<_, EraIndex>;

	/// The last planned session scheduled by the session pallet.
	///
	/// This is basically in sync with the call to [`SessionManager::new_session`].
	#[pallet::storage]
	#[pallet::getter(fn current_planned_session)]
	pub type CurrentPlannedSession<T> = StorageValue<_, SessionIndex, ValueQuery>;

	/// True if network has been upgraded to this version.
	/// Storage version of the pallet.
	///
	/// This is set to v6.0.0 for new networks.
	#[pallet::storage]
	pub(crate) type StorageVersion<T: Config> = StorageValue<_, Releases, ValueQuery>;

	/// The chain's running time form genesis in milliseconds,
	/// use for calculate darwinia era payout
	#[pallet::storage]
	#[pallet::getter(fn living_time)]
	pub type LivingTime<T> = StorageValue<_, TsInMs>;

	/// The percentage of the total payout that is distributed to validators and nominators
	///
	/// The reset might go to Treasury or something else.
	#[pallet::storage]
	#[pallet::getter(fn payout_fraction)]
	pub type PayoutFraction<T> = StorageValue<_, Perbill>;

	/// Total *RING* in pool.
	#[pallet::storage]
	#[pallet::getter(fn ring_pool)]
	pub type RingPool<T> = StorageValue<_, RingBalance<T>>;
	/// Total *KTON* in pool.
	#[pallet::storage]
	#[pallet::getter(fn kton_pool)]
	pub type KtonPool<T> = StorageValue<_, RingBalance<T>>;

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
				payout_fraction: Default::default(),
			}
		}
	}
	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			HistoryDepth::<T>::put(self.history_depth);
			ValidatorCount::<T>::put(self.validator_count);
			MinimumValidatorCount::<T>::put(self.minimum_validator_count);
			Invulnerables::<T>::put(&self.invulnerables);
			ForceEra::<T>::put(self.force_era);
			CanceledSlashPayout::<T>::put(self.canceled_payout);
			SlashRewardFraction::<T>::put(self.slash_reward_fraction);
			StorageVersion::<T>::put(Releases::V6_0_0);
			PayoutFraction::<T>::put(self.payout_fraction);

			for (stash, controller, ring_to_be_bonded, status) in self.stakers {
				assert!(
					T::RingCurrency::free_balance(&stash) >= ring_to_be_bonded,
					"Stash does not have enough balance to bond.",
				);
				let _ = <Pallet<T>>::bond(
					T::Origin::from(Some(stash.to_owned()).into()),
					T::Lookup::unlookup(controller.to_owned()),
					StakingBalance::RingBalance(ring_to_be_bonded),
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
		start: Option<u64>,
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
		total: RewardPoint,
		/// The reward points earned by a given validator.
		individual: BTreeMap<AccountId, RewardPoint>,
	}

	/// Mode of era-forcing.
	#[derive(Copy, Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug)]
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	pub enum Forcing {
		/// Not forcing anything - just let whatever happen.
		NotForcing,
		/// Force a new era, then reset to `NotForcing` as soon as it is done.
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
	enum Releases {
		V1_0_0Ancient,
		V2_0_0,
		V3_0_0,
		V4_0_0,
		V5_0_0, // blockable validators.
		V6_0_0, // removal of all storage associated with offchain phragmen.
	}
	impl Default for Releases {
		fn default() -> Self {
			Releases::V6_0_0
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
}
pub use pallet::*;
