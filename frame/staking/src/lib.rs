// This file is part of Darwinia.
//
// Copyright (C) 2018-2022 Darwinia Network
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
//! #### Voting
//!
//! Staking is closely related to elections; actual validators are chosen from among all potential
//! validators via election by the potential validators and nominators. To reduce use of the phrase
//! "potential validators and nominators", we often use the term **voters**, who are simply
//! the union of potential validators and nominators.
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
#![allow(clippy::all)]
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
mod mock;
#[cfg(test)]
mod substrate_tests;
#[cfg(test)]
mod testing_utils;

pub mod impls;
pub use impls::*;
pub mod inflation;
pub mod migrations;
pub mod slashing;
pub mod structs;
pub use structs::*;
pub mod primitives;
pub use primitives::*;
pub mod weights;
pub use weights::*;

#[frame_support::pallet]
pub mod pallet {
	// --- paritytech ---
	use frame_election_provider_support::{ElectionProvider, *};
	use frame_support::{
		pallet_prelude::*,
		traits::{
			Currency, EstimateNextNewSession, LockableCurrency, OnUnbalanced, UnixTime,
			WithdrawReasons,
		},
		PalletId,
	};
	use frame_system::{offchain::SendTransactionTypes, pallet_prelude::*};
	use sp_runtime::{
		traits::{CheckedSub, Saturating, StaticLookup, Zero},
		Perbill, Percent, SaturatedConversion,
	};
	use sp_staking::{EraIndex, SessionIndex};
	use sp_std::prelude::*;
	// --- darwinia-network ---
	use crate::*;
	use darwinia_support::balance::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
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
		/// It is guaranteed to start being called from the first `on_finalize`. Thus value at
		/// genesis is not used.
		type UnixTime: UnixTime;

		/// Something that provides the election functionality.
		type ElectionProvider: ElectionProvider<
			AccountId = Self::AccountId,
			BlockNumber = Self::BlockNumber,
			// we only accept an election provider that has staking as data provider.
			DataProvider = Pallet<Self>,
		>;

		/// Something that provides the election functionality at genesis.
		type GenesisElectionProvider: ElectionProvider<
			AccountId = Self::AccountId,
			BlockNumber = Self::BlockNumber,
			DataProvider = Pallet<Self>,
		>;

		/// Number of sessions per era.
		#[pallet::constant]
		type SessionsPerEra: Get<SessionIndex>;
		/// Interface for interacting with a session pallet.
		type SessionInterface: self::SessionInterface<Self::AccountId>;
		/// Something that can estimate the next session change, accurately or as a best effort
		/// guess.
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
		/// For each validator only the `$MaxNominatorRewardedPerValidator` biggest stakers can
		/// claim their reward. This used to limit the i/o cost for the nominator payout.
		#[pallet::constant]
		type MaxNominatorRewardedPerValidator: Get<u32>;

		/// The fraction of the validator set that is safe to be offending.
		/// After the threshold is reached a new era will be forced.
		type OffendingValidatorsThreshold: Get<Perbill>;

		/// Something that can provide a sorted list of voters in a somewhat sorted way. The
		/// original use case for this was designed with [`pallet_bags_list::Pallet`] in mind. If
		/// the bags-list is not desired, [`impls::UseNominatorsMap`] is likely the desired option.
		type SortedListProvider: SortedListProvider<Self::AccountId>;

		/// Number of eras that staked funds must remain bonded for.
		#[pallet::constant]
		type BondingDurationInEra: Get<EraIndex>;
		/// Number of eras that staked funds must remain bonded for.
		#[pallet::constant]
		type BondingDurationInBlockNumber: Get<Self::BlockNumber>;

		/// The *RING* currency.
		type RingCurrency: LockableCurrency<Self::AccountId>;
		/// Tokens have been minted and are unused for validator-reward.
		/// See [Era payout](./index.html#era-payout).
		type RingRewardRemainder: OnUnbalanced<RingNegativeImbalance<Self>>;
		/// Handler for the unbalanced *RING* reduction when slashing a staker.
		type RingSlash: OnUnbalanced<RingNegativeImbalance<Self>>;
		/// Handler for the unbalanced *RING* increment when rewarding a staker.
		type RingReward: OnUnbalanced<RingPositiveImbalance<Self>>;

		/// The *KTON* currency.
		type KtonCurrency: LockableCurrency<Self::AccountId>;
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
	pub enum Event<T: Config> {
		/// The era payout has been set; the first balance is the validator-payout; the second is
		/// the remainder from the maximum amount of reward.
		/// \[era_index, validator_payout, remainder\]
		EraPaid(EraIndex, RingBalance<T>, RingBalance<T>),

		/// The nominator has been rewarded by this amount. \[stash, amount\]
		Rewarded(AccountId<T>, RingBalance<T>),

		/// One validator (and its nominators) has been slashed by the given amount.
		/// \[validator, amount, amount\]
		Slashed(AccountId<T>, RingBalance<T>, KtonBalance<T>),
		/// An old slashing report from a prior era was discarded because it could
		/// not be processed. \[session_index\]
		OldSlashingReportDiscarded(SessionIndex),

		/// A new set of stakers was elected.
		StakersElected,

		/// An account has bonded this amount. \[amount, start, end\]
		///
		/// NOTE: This event is only emitted when funds are bonded via a dispatchable. Notably,
		/// it will not be emitted for staking rewards when they are added to stake.
		RingBonded(AccountId<T>, RingBalance<T>, TsInMs, TsInMs),
		/// An account has bonded this amount. \[account, amount, start, end\]
		///
		/// NOTE: This event is only emitted when funds are bonded via a dispatchable. Notably,
		/// it will not be emitted for staking rewards when they are added to stake.
		KtonBonded(AccountId<T>, KtonBalance<T>),

		/// An account has unbonded this amount. \[amount\]
		RingUnbonded(AccountId<T>, RingBalance<T>),
		/// An account has unbonded this amount. \[account, amount\]
		KtonUnbonded(AccountId<T>, KtonBalance<T>),

		/// A nominator has been kicked from a validator. \[nominator, stash\]
		Kicked(AccountId<T>, AccountId<T>),

		/// The election failed. No new era is planned.
		StakingElectionFailed,

		/// An account has stopped participating as either a validator or nominator.
		/// \[stash\]
		Chilled(T::AccountId),

		/// The stakers' rewards are getting paid. \[era_index, validator_stash\]
		PayoutStarted(EraIndex, T::AccountId),

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
		/// Cannot have a validator or nominator role, with value less than the minimum defined by
		/// governance (see `MinValidatorBond` and `MinNominatorBond`). If unbonding is the
		/// intention, `chill` first to remove one's role as validator/nominator.
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
		/// There are too many nominators in the system. Governance needs to adjust the staking
		/// settings to keep things safe for the runtime.
		TooManyNominators,
		/// There are too many validators in the system. Governance needs to adjust the staking
		/// settings to keep things safe for the runtime.
		TooManyValidators,
		/// Commission is too low. Must be at least `MinCommission`.
		CommissionTooLow,
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

	/// The minimum amount of commission that validators can set.
	///
	/// If set to `0`, no limit exists.
	#[pallet::storage]
	pub type MinCommission<T: Config> = StorageValue<_, Perbill, ValueQuery>;

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
		CountedStorageMap<_, Twox64Concat, AccountId<T>, ValidatorPrefs, ValueQuery>;

	/// The maximum validator count before we stop allowing new validators to join.
	///
	/// When this value is not set, no limits are enforced.
	#[pallet::storage]
	pub type MaxValidatorsCount<T> = StorageValue<_, u32, OptionQuery>;

	/// The map from nominator stash key to the set of stash keys of all validators to nominate.
	#[pallet::storage]
	#[pallet::getter(fn nominators)]
	pub type Nominators<T: Config> =
		CountedStorageMap<_, Twox64Concat, AccountId<T>, Nominations<AccountId<T>>>;

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
	/// This is basically in sync with the call to [`pallet_session::SessionManager::new_session`].
	#[pallet::storage]
	#[pallet::getter(fn current_planned_session)]
	pub type CurrentPlannedSession<T> = StorageValue<_, SessionIndex, ValueQuery>;

	/// Indices of validators that have offended in the active era and whether they are currently
	/// disabled.
	///
	/// This value should be a superset of disabled validators since not all offences lead to the
	/// validator being disabled (if there was no slash). This is needed to track the percentage of
	/// validators that have offended in the current era, ensuring a new era is forced if
	/// `OffendingValidatorsThreshold` is reached. The vec is always kept sorted so that we can find
	/// whether a given validator has previously offended using binary search. It gets cleared when
	/// the era ends.
	#[pallet::storage]
	#[pallet::getter(fn offending_validators)]
	pub type OffendingValidators<T: Config> = StorageValue<_, Vec<(u32, bool)>, ValueQuery>;

	/// True if network has been upgraded to this version.
	/// Storage version of the pallet.
	///
	/// This is set to v7.0.0 for new networks.
	#[pallet::storage]
	pub type StorageVersion<T: Config> = StorageValue<_, Releases, ValueQuery>;

	/// The threshold for when users can start calling `chill_other` for other validators /
	/// nominators. The threshold is compared to the actual number of validators / nominators
	/// (`CountFor*`) in the system compared to the configured max (`Max*Count`).
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
		pub stakers: Vec<(AccountId<T>, AccountId<T>, RingBalance<T>, StakerStatus<AccountId<T>>)>,
		pub min_nominator_bond: RingBalance<T>,
		pub min_validator_bond: RingBalance<T>,
		pub max_validator_count: Option<u32>,
		pub max_nominator_count: Option<u32>,
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
				max_validator_count: None,
				max_nominator_count: None,
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
			if let Some(x) = self.max_validator_count {
				MaxValidatorsCount::<T>::put(x);
			}
			if let Some(x) = self.max_nominator_count {
				MaxNominatorsCount::<T>::put(x);
			}
			<PayoutFraction<T>>::put(self.payout_fraction);

			for (stash, controller, ring_to_be_bonded, status) in &self.stakers {
				log!(
					trace,
					"inserting genesis staker: {:?} => {:?} => {:?}",
					stash,
					ring_to_be_bonded,
					status
				);
				assert!(
					T::RingCurrency::free_balance(&stash) >= *ring_to_be_bonded,
					"Stash does not have enough balance to bond.",
				);

				frame_support::assert_ok!(<Pallet<T>>::bond(
					T::Origin::from(Some(stash.to_owned()).into()),
					T::Lookup::unlookup(controller.to_owned()),
					StakingBalance::RingBalance(*ring_to_be_bonded),
					RewardDestination::Staked,
					0,
				));
				frame_support::assert_ok!(match status {
					StakerStatus::Validator => <Pallet<T>>::validate(
						T::Origin::from(Some(controller.to_owned()).into()),
						Default::default(),
					),
					StakerStatus::Nominator(votes) => <Pallet<T>>::nominate(
						T::Origin::from(Some(controller.to_owned()).into()),
						votes.iter().map(|l| T::Lookup::unlookup(l.to_owned())).collect(),
					),
					_ => Ok(()),
				});
				T::RingCurrency::make_free_balance_be(
					&<Pallet<T>>::account_id(),
					T::RingCurrency::minimum_balance(),
				);
			}

			// all voters are reported to the `SortedListProvider`.
			assert_eq!(
				T::SortedListProvider::count(),
				<Nominators<T>>::count(),
				"not all genesis stakers were inserted into sorted list provider, something is wrong."
			);
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
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
					// This write only ever happens once, we don't include it in the weight in
					// general
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
		/// Emits `Bonded`.
		/// # <weight>
		/// - Independent of the arguments. Moderate complexity.
		/// - O(1).
		/// - Three extra DB entries.
		///
		/// NOTE: Two of the storage writes (`Self::bonded`, `Self::payee`) are _never_ cleaned
		/// unless the `origin` falls below _existential deposit_ and gets removed as dust.
		/// ------------------
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
			ensure!(!<Bonded<T>>::contains_key(&stash), <Error<T>>::AlreadyBonded);

			let controller = T::Lookup::lookup(controller)?;
			ensure!(!<Ledger<T>>::contains_key(&controller), <Error<T>>::AlreadyPaired);

			match value {
				StakingBalance::RingBalance(value) => {
					// Reject a bond which is considered to be _dust_.
					ensure!(
						value >= T::RingCurrency::minimum_balance(),
						<Error<T>>::InsufficientBond,
					);
				},
				StakingBalance::KtonBalance(value) => {
					// Reject a bond which is considered to be _dust_.
					ensure!(
						value >= T::KtonCurrency::minimum_balance(),
						<Error<T>>::InsufficientBond,
					);
				},
			}

			<frame_system::Pallet<T>>::inc_consumers(&stash).map_err(|_| <Error<T>>::BadState)?;

			// You're auto-bonded forever, here. We might improve this by only bonding when
			// you actually validate/nominate and remove once you unbond __everything__.
			<Bonded<T>>::insert(&stash, &controller);
			<Payee<T>>::insert(&stash, payee);

			let ledger = {
				let mut l = StakingLedger::default_from(stash.clone());

				l.claimed_rewards = {
					let current_era = <CurrentEra<T>>::get().unwrap_or(0);
					let last_reward_era = current_era.saturating_sub(Self::history_depth());

					(last_reward_era..current_era).collect()
				};

				l
			};

			match value {
				StakingBalance::RingBalance(value) => {
					let stash_balance = T::RingCurrency::free_balance(&stash);
					let value = value.min(stash_balance);
					let promise_month = promise_month.min(36);
					let (start_time, expire_time) =
						Self::bond_ring(&stash, &controller, value, promise_month, ledger)?;

					Self::deposit_event(Event::RingBonded(stash, value, start_time, expire_time));
				},
				StakingBalance::KtonBalance(value) => {
					let stash_balance = T::KtonCurrency::free_balance(&stash);
					let value = value.min(stash_balance);

					Self::bond_kton(&controller, value, ledger)?;
					Self::deposit_event(Event::KtonBonded(stash, value));
				},
			}

			Ok(())
		}

		/// Add some extra amount that have appeared in the stash `free_balance` into the balance up
		/// for staking.
		///
		/// The dispatch origin for this call must be _Signed_ by the stash, not the controller.
		///
		/// Use this if there are additional funds in your stash account that you wish to bond.
		/// Unlike [`bond`](Self::bond) or [`unbond`](Self::unbond) this function does not impose
		/// any limitation on the amount that can be added.
		///
		/// Emits `Bonded`.
		///
		/// # <weight>
		/// - Independent of the arguments. Insignificant complexity.
		/// - O(1).
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
			let now = <frame_system::Pallet<T>>::block_number();

			match max_additional {
				StakingBalance::RingBalance(max_additional) => {
					let stash_balance = T::RingCurrency::free_balance(&stash);

					if let Some(extra) = stash_balance.checked_sub(
						&(ledger.active + ledger.ring_staking_lock.total_unbond_at(now)),
					) {
						let extra = extra.min(max_additional);
						let (start_time, expire_time) =
							Self::bond_ring(&stash, &controller, extra, promise_month, ledger)?;

						Self::deposit_event(Event::RingBonded(
							stash.clone(),
							extra,
							start_time,
							expire_time,
						));
					}
				},
				StakingBalance::KtonBalance(max_additional) => {
					let stash_balance = T::KtonCurrency::free_balance(&stash);

					if let Some(extra) = stash_balance.checked_sub(
						&(ledger.active_kton + ledger.kton_staking_lock.total_unbond_at(now)),
					) {
						let extra = extra.min(max_additional);

						Self::bond_kton(&controller, extra, ledger)?;
						Self::deposit_event(Event::KtonBonded(stash.clone(), extra));
					}
				},
			}

			// update this staker in the sorted list, if they exist in it.
			if T::SortedListProvider::contains(&stash) {
				T::SortedListProvider::on_update(&stash, Self::weight_of(&stash));
				debug_assert_eq!(T::SortedListProvider::sanity_check(), Ok(()));
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
			let mut ledger = Self::ledger(&controller).ok_or(<Error<T>>::NotController)?;

			if value.is_zero() {
				return Ok(());
			}

			let start_time = T::UnixTime::now().as_millis().saturated_into::<TsInMs>();
			let promise_month = promise_month.max(1).min(36);
			let expire_time = start_time + promise_month as TsInMs * MONTH_IN_MILLISECONDS;
			let StakingLedger { active, active_deposit_ring, deposit_items, .. } = &mut ledger;
			let value = value.min(active.saturating_sub(*active_deposit_ring));

			if value.is_zero() {
				return Ok(());
			}

			let kton_return = inflation::compute_kton_reward::<T>(value, promise_month);
			let kton_positive_imbalance = T::KtonCurrency::deposit_creating(&stash, kton_return);

			T::KtonReward::on_unbalanced(kton_positive_imbalance);

			*active_deposit_ring = active_deposit_ring.saturating_add(value);
			deposit_items.push(TimeDepositItem { value, start_time, expire_time });

			<Ledger<T>>::insert(&controller, ledger);

			Self::deposit_event(Event::RingBonded(stash, value, start_time, expire_time));

			Ok(())
		}

		/// Schedule a portion of the stash to be unlocked ready for transfer out after the bond
		/// period ends. If this leaves an amount actively bonded less than
		/// T::Currency::minimum_balance(), then it is increased to the full amount.
		///
		/// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
		///
		/// Once the unlock period is done, the funds will be withdrew automatically and ready for
		/// transfer.
		///
		/// No more than a limited number of unlocking chunks (see `MAX_UNLOCKING_CHUNKS`)
		/// can co-exists at the same time. In that case,  [`StakingLock::shrink`] need
		/// to be called first to remove some of the chunks (if possible).
		///
		/// If a user encounters the `InsufficientBond` error when calling this extrinsic,
		/// they should call `chill` first in order to free up their bonded funds.
		///
		/// After all pledged Ring and Kton are unbonded, the bonded accounts, namely stash and
		/// controller, will also be unbonded.  Once user want to bond again, the `bond` method
		/// should be called. If there are still pledged Ring or Kton and user want to bond more
		/// values, the `bond_extra` method should be called.
		#[pallet::weight(T::WeightInfo::unbond())]
		pub fn unbond(origin: OriginFor<T>, value: StakingBalanceT<T>) -> DispatchResult {
			let controller = ensure_signed(origin)?;
			// TODO: Simplify the unbond logic, do not clear the mature deposit here.
			let mut ledger = Self::clear_mature_deposits(
				Self::ledger(&controller).ok_or(<Error<T>>::NotController)?,
			)
			.0;
			let StakingLedger {
				stash,
				active,
				active_deposit_ring,
				active_kton,
				ring_staking_lock,
				kton_staking_lock,
				..
			} = &mut ledger;
			let now = <frame_system::Pallet<T>>::block_number();

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

			let origin_active = active.clone();
			let origin_active_kton = active_kton.clone();
			let mut unbond_ring: RingBalance<T> = Zero::zero();
			let mut unbond_kton: KtonBalance<T> = Zero::zero();

			match value {
				StakingBalance::RingBalance(r) => {
					// Only active normal ring can be unbond:
					// `active = active_normal_ring + active_deposit_ring`
					let active_normal_ring = *active - *active_deposit_ring;

					unbond_ring = r.min(active_normal_ring);

					if !unbond_ring.is_zero() {
						*active -= unbond_ring;

						// Avoid there being a dust balance left in the staking system.
						if (*active < T::RingCurrency::minimum_balance())
							&& (*active_kton < T::KtonCurrency::minimum_balance())
						{
							unbond_ring += *active;
							unbond_kton += *active_kton;

							*active = Zero::zero();
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
						ensure!(*active >= min_active_bond, <Error<T>>::InsufficientBond);

						ring_staking_lock
							.unbondings
							.try_push(Unbonding {
								amount: unbond_ring,
								until: now + T::BondingDurationInBlockNumber::get(),
							})
							.expect("ALREADY CHECKED THE BOUNDARY MUST NOT FAIL!");

						Self::deposit_event(Event::RingUnbonded(stash.clone(), unbond_ring));

						if !unbond_kton.is_zero() {
							kton_staking_lock
								.unbondings
								.try_push(Unbonding {
									amount: unbond_kton,
									until: now + T::BondingDurationInBlockNumber::get(),
								})
								.expect("ALREADY CHECKED THE BOUNDARY MUST NOT FAIL!");

							Self::deposit_event(Event::KtonUnbonded(
								ledger.stash.clone(),
								unbond_kton,
							));
						}
					}
				},
				StakingBalance::KtonBalance(k) => {
					unbond_kton = k.min(*active_kton);

					if !unbond_kton.is_zero() {
						*active_kton -= unbond_kton;

						// Avoid there being a dust balance left in the staking system.
						if (*active_kton < T::KtonCurrency::minimum_balance())
							&& (*active < T::RingCurrency::minimum_balance())
						{
							unbond_kton += *active_kton;
							unbond_ring += *active;

							*active_kton = Zero::zero();
							*active = Zero::zero();
						}

						kton_staking_lock
							.unbondings
							.try_push(Unbonding {
								amount: unbond_kton,
								until: now + T::BondingDurationInBlockNumber::get(),
							})
							.expect("ALREADY CHECKED THE BOUNDARY MUST NOT FAIL!");

						Self::deposit_event(Event::KtonUnbonded(stash.clone(), unbond_kton));

						if !unbond_ring.is_zero() {
							ring_staking_lock
								.unbondings
								.try_push(Unbonding {
									amount: unbond_ring,
									until: now + T::BondingDurationInBlockNumber::get(),
								})
								.expect("ALREADY CHECKED THE BOUNDARY MUST NOT FAIL!");

							Self::deposit_event(Event::RingUnbonded(
								ledger.stash.clone(),
								unbond_ring,
							));
						}
					}
				},
			}

			Self::update_ledger(&controller, &mut ledger);
			Self::update_staking_pool(
				ledger.active,
				origin_active,
				ledger.active_kton,
				origin_active_kton,
			);

			// update this staker in the sorted list, if they exist in it.
			if T::SortedListProvider::contains(&ledger.stash) {
				T::SortedListProvider::on_update(&ledger.stash, Self::weight_of(&ledger.stash));
			}

			Ok(())
		}

		/// Remove any unlocked chunks from the `unlocking` queue from our management.
		///
		/// This essentially frees up that balance to be used by the stash account to do
		/// whatever it wants.
		///
		/// The dispatch origin for this call must be _Signed_ by the controller.
		///
		/// Emits `Withdrawn`.
		///
		/// See also [`Call::unbond`].
		///
		/// # <weight>
		/// Complexity O(S) where S is the number of slashing spans to remove
		/// NOTE: Weight annotation is the kill scenario, we refund otherwise.
		/// # </weight>
		#[pallet::weight(T::WeightInfo::withdraw_unbonded_kill(*num_slashing_spans))]
		pub fn withdraw_unbonded(
			origin: OriginFor<T>,
			num_slashing_spans: u32,
		) -> DispatchResultWithPostInfo {
			let controller = ensure_signed(origin)?;
			let mut ledger = Self::ledger(&controller).ok_or(<Error<T>>::NotController)?;

			ledger.consolidate_unbondings(<frame_system::Pallet<T>>::block_number());

			let StakingLedger {
				stash,
				active,
				active_kton,
				ring_staking_lock,
				kton_staking_lock,
				..
			} = &ledger;

			let post_info_weight = if ring_staking_lock.unbondings.is_empty()
			    // Some chains' ED might be 0.
				&& (active < &T::RingCurrency::minimum_balance() || active.is_zero())
				&& kton_staking_lock.unbondings.is_empty()
				&& (active_kton < &T::KtonCurrency::minimum_balance() || active_kton.is_zero())
			{
				// This account must have called `unbond()` with some value that caused the active
				// portion to fall below existential deposit + will have no more unlocking chunks
				// left. We can now safely remove all staking-related information.
				Self::kill_stash(stash, num_slashing_spans)?;

				// Remove the lock.
				T::RingCurrency::remove_lock(STAKING_ID, stash);
				T::KtonCurrency::remove_lock(STAKING_ID, stash);

				// This is worst case scenario, so we use the full weight and return None
				None
			} else {
				// This was the consequence of a partial unbond. just update the ledger and move on.
				Self::update_ledger(&controller, &mut ledger);

				// This is only an update, so we use less overall weight.
				Some(T::WeightInfo::withdraw_unbonded_update(num_slashing_spans))
			};

			// `old_total` should never be less than the new total because
			// `consolidate_unlocked` strictly subtracts balance.
			// if ledger.total < old_total {
			// 	// Already checked that this won't overflow by entry condition.
			// 	let value = old_total - ledger.total;
			// 	Self::deposit_event(<Event<T>>::Withdrawn(stash.clone(), value));
			// }

			Ok(post_info_weight.into())
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
		#[pallet::weight(T::WeightInfo::claim_mature_deposits())]
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
		#[pallet::weight(T::WeightInfo::try_claim_deposits_with_punish())]
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

			let StakingLedger { stash, active_deposit_ring, deposit_items, .. } = &mut ledger;

			if let Some(i) = deposit_items.iter().position(|item| item.expire_time == expire_time) {
				let item = &deposit_items[i];
				let plan_duration_in_months =
					item.expire_time.saturating_sub(item.start_time) / MONTH_IN_MILLISECONDS;
				let passed_duration_in_months =
					now.saturating_sub(item.start_time) / MONTH_IN_MILLISECONDS;
				let kton_penalty =
					(inflation::compute_kton_reward::<T>(item.value, plan_duration_in_months as _)
						- inflation::compute_kton_reward::<T>(
							item.value,
							passed_duration_in_months as _,
						))
					.max(1u32.into()) * 3u32.into();
				// Can only pay the penalty from usable balance.
				// Not allow to use bonded KTON to pay the penalty.
				let new_kton_balance =
					T::KtonCurrency::free_balance(stash).saturating_sub(kton_penalty);

				if new_kton_balance.is_zero()
					|| T::KtonCurrency::ensure_can_withdraw(
						stash,
						kton_penalty,
						WithdrawReasons::TRANSFER,
						new_kton_balance,
					)
					.is_err()
				{
					return Ok(());
				}

				T::KtonSlash::on_unbalanced(T::KtonCurrency::slash(stash, kton_penalty).0);
				*active_deposit_ring = active_deposit_ring.saturating_sub(item.value);
				deposit_items.remove(i);

				<Ledger<T>>::insert(&controller, &ledger);
				Self::deposit_event(Event::DepositsClaimedWithPunish(
					ledger.stash.clone(),
					kton_penalty,
				));
			}

			Ok(())
		}

		/// Declare the desire to validate for the origin controller.
		///
		/// Effects will be felt at the beginning of the next era.
		///
		/// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
		#[pallet::weight(T::WeightInfo::validate())]
		pub fn validate(origin: OriginFor<T>, prefs: ValidatorPrefs) -> DispatchResult {
			let controller = ensure_signed(origin)?;
			let ledger = Self::ledger(&controller).ok_or(<Error<T>>::NotController)?;

			ensure!(ledger.active >= <MinValidatorBond<T>>::get(), <Error<T>>::InsufficientBond);

			let stash = &ledger.stash;

			// ensure their commission is correct.
			ensure!(prefs.commission >= <MinCommission<T>>::get(), <Error<T>>::CommissionTooLow);

			// Only check limits if they are not already a validator.
			if !<Validators<T>>::contains_key(stash) {
				// If this error is reached, we need to adjust the `MinValidatorBond` and start
				// calling `chill_other`. Until then, we explicitly block new validators to protect
				// the runtime.
				if let Some(max_validators) = <MaxValidatorsCount<T>>::get() {
					ensure!(
						<Validators<T>>::count() < max_validators,
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
		/// Effects will be felt at the beginning of the next era.
		///
		/// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
		///
		/// # <weight>
		/// - The transaction's complexity is proportional to the size of `targets` (N)
		/// which is capped at CompactAssignments::LIMIT (MAX_NOMINATIONS).
		/// - Both the reads and writes follow a similar pattern.
		#[pallet::weight(T::WeightInfo::nominate(targets.len() as u32))]
		pub fn nominate(
			origin: OriginFor<T>,
			targets: Vec<<T::Lookup as StaticLookup>::Source>,
		) -> DispatchResult {
			let controller = ensure_signed(origin)?;
			let ledger = Self::ledger(&controller).ok_or(<Error<T>>::NotController)?;

			ensure!(ledger.active >= <MinNominatorBond<T>>::get(), <Error<T>>::InsufficientBond);

			let stash = &ledger.stash;

			// Only check limits if they are not already a nominator.
			if !<Nominators<T>>::contains_key(stash) {
				// If this error is reached, we need to adjust the `MinNominatorBond` and start
				// calling `chill_other`. Until then, we explicitly block new nominators to protect
				// the runtime.
				if let Some(max_nominators) = <MaxNominatorsCount<T>>::get() {
					ensure!(
						<Nominators<T>>::count() < max_nominators,
						<Error<T>>::TooManyNominators
					);
				}
			}

			ensure!(!targets.is_empty(), <Error<T>>::EmptyTargets);
			ensure!(targets.len() <= T::MAX_NOMINATIONS as usize, <Error<T>>::TooManyTargets);

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
		///
		/// # <weight>
		/// - Independent of the arguments. Insignificant complexity.
		/// - Contains one read.
		/// - Writes are limited to the `origin` account key.
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

			ensure!(!<Ledger<T>>::contains_key(&controller), <Error<T>>::AlreadyPaired);

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
		/// Same as [`Self::set_validator_count`].
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
		/// Same as [`Self::set_validator_count`].
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
		/// Writes: Bonded, Slashing Spans (if S > 0), Ledger, Payee, Validators, Nominators,
		/// Account, Locks Writes Each: SpanSlash * S
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
			ensure!(is_sorted_and_unique(&slash_indices), <Error<T>>::NotSortedAndUnique);

			let mut unapplied = <Self as Store>::UnappliedSlashes::get(&era);
			let last_item = slash_indices[slash_indices.len() - 1];
			ensure!((last_item as usize) < unapplied.len(), <Error<T>>::InvalidSlashIndex);

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
		/// # <weight>
		/// - Time complexity: at most O(MaxNominatorRewardedPerValidator).
		/// - Contains a limited number of reads and writes.
		/// -----------
		/// N is the Number of payouts for the validator (including the validator)
		/// Weight:
		/// - Reward Destination Staked: O(N)
		/// - Reward Destination Controller (Creating): O(N)
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
		/// The dispatch origin must be signed by the controller.
		///
		/// # <weight>
		/// - Time complexity: O(L), where L is unlocking chunks
		/// - Bounded by `MAX_UNLOCKING_CHUNKS`.
		/// - Storage changes: Can't increase storage, only decrease it.
		/// # </weight>
		#[pallet::weight(T::WeightInfo::rebond(MAX_UNLOCKING_CHUNKS as u32))]
		pub fn rebond(
			origin: OriginFor<T>,
			#[pallet::compact] plan_to_rebond_ring: RingBalance<T>,
			#[pallet::compact] plan_to_rebond_kton: KtonBalance<T>,
		) -> DispatchResultWithPostInfo {
			let controller = ensure_signed(origin)?;
			let mut ledger = Self::ledger(&controller).ok_or(<Error<T>>::NotController)?;
			let StakingLedger { ring_staking_lock, kton_staking_lock, .. } = &ledger;
			let origin_active = ledger.active.clone();
			let origin_active_kton = ledger.active_kton.clone();

			ensure!(
				!ring_staking_lock.unbondings.is_empty()
					|| !kton_staking_lock.unbondings.is_empty(),
				<Error<T>>::NoUnlockChunk
			);

			let initial_unbondings = ring_staking_lock.unbondings.len() as u32
				+ kton_staking_lock.unbondings.len() as u32;
			let (rebonded_ring, rebonded_kton) =
				ledger.rebond(plan_to_rebond_ring, plan_to_rebond_kton);

			// Last check: the new active amount of ledger must be more than ED.
			ensure!(
				ledger.active >= T::RingCurrency::minimum_balance()
					|| ledger.active_kton >= T::KtonCurrency::minimum_balance(),
				<Error<T>>::InsufficientBond
			);

			Self::update_ledger(&controller, &mut ledger);
			Self::update_staking_pool(
				ledger.active,
				origin_active,
				ledger.active_kton,
				origin_active_kton,
			);

			let StakingLedger { stash, ring_staking_lock, kton_staking_lock, .. } = &ledger;
			let ring_rebonded = !rebonded_ring.is_zero();
			let kton_rebonded = !rebonded_kton.is_zero();

			if ring_rebonded {
				let now = T::UnixTime::now().as_millis().saturated_into::<TsInMs>();

				Self::deposit_event(Event::RingBonded(stash.clone(), rebonded_ring, now, now));
			}
			if kton_rebonded {
				Self::deposit_event(Event::KtonBonded(stash.clone(), rebonded_kton));
			}
			if ring_rebonded && kton_rebonded {
				if T::SortedListProvider::contains(stash) {
					T::SortedListProvider::on_update(stash, Self::weight_of(stash));
				}
			}

			let removed_unbondings = 1.saturating_add(initial_unbondings).saturating_sub(
				ring_staking_lock.unbondings.len() as u32
					+ kton_staking_lock.unbondings.len() as u32,
			);

			Ok(Some(T::WeightInfo::rebond(removed_unbondings)).into())
		}

		/// Set `HistoryDepth` value. This function will delete any history information
		/// when `HistoryDepth` is reduced.
		///
		/// Parameters:
		/// - `new_history_depth`: The new history depth you would like to set.
		/// - `era_items_deleted`: The number of items that will be deleted by this dispatch. This
		///   should report all the storage items that will be deleted by clearing old era history.
		///   Needed to report an accurate weight for the dispatch. Trusted by `Root` to report an
		///   accurate number.
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
		///     - Writes Each: ErasValidatorReward, ErasRewardPoints, ErasTotalStake,
		///       ErasStartSessionIndex
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

		/// Remove all data structures concerning a staker/stash once it is at a state where it can
		/// be considered `dust` in the staking system. The requirements are:
		///
		/// 1. the `total_balance` of the stash is below existential deposit.
		/// 2. or, the `ledger.total` of the stash is below existential deposit.
		///
		/// The former can happen in cases like a slash; the latter when a fully unbonded account
		/// is still receiving staking rewards in `RewardDestination::Staked`.
		///
		/// It can be called by anyone, as long as `stash` meets the above requirements.
		///
		/// Refunds the transaction fees upon successful execution.
		#[pallet::weight(T::WeightInfo::reap_stash(*num_slashing_spans))]
		pub fn reap_stash(
			origin: OriginFor<T>,
			stash: AccountId<T>,
			num_slashing_spans: u32,
		) -> DispatchResultWithPostInfo {
			ensure_signed(origin)?;

			let total_ring = T::RingCurrency::total_balance(&stash);
			let total_kton = T::KtonCurrency::total_balance(&stash);
			let ed_ring = T::RingCurrency::minimum_balance();
			let ed_kton = T::KtonCurrency::minimum_balance();
			let reapable = if let Some(ledger) =
				Self::ledger(Self::bonded(stash.clone()).ok_or(Error::<T>::NotStash)?)
			{
				((total_ring.is_zero() || total_ring < ed_ring)
					&& (total_kton.is_zero() || total_kton < ed_kton))
					|| (ledger.active < ed_ring && ledger.active_kton < ed_kton)
			} else {
				true
			};

			ensure!(reapable, <Error<T>>::FundedTarget);

			Self::kill_stash(&stash, num_slashing_spans)?;
			T::RingCurrency::remove_lock(STAKING_ID, &stash);
			T::KtonCurrency::remove_lock(STAKING_ID, &stash);

			Ok(Pays::No.into())
		}

		/// Remove the given nominations from the calling validator.
		///
		/// Effects will be felt at the beginning of the next era.
		///
		/// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
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

		/// Update the various staking configurations.
		///
		/// * `min_nominator_bond`: The minimum active bond needed to be a nominator.
		/// * `min_validator_bond`: The minimum active bond needed to be a validator.
		/// * `max_nominator_count`: The max number of users who can be a nominator at once. When
		///   set to `None`, no limit is enforced.
		/// * `max_validator_count`: The max number of users who can be a validator at once. When
		///   set to `None`, no limit is enforced.
		/// * `chill_threshold`: The ratio of `max_nominator_count` or `max_validator_count` which
		///   should be filled in order for the `chill_other` transaction to work.
		/// * `min_commission`: The minimum amount of commission that each validators must maintain.
		///   This is checked only upon calling `validate`. Existing validators are not affected.
		///
		/// Origin must be Root to call this function.
		///
		/// NOTE: Existing nominators and validators will not be affected by this update.
		/// to kick people under the new limits, `chill_other` should be called.
		#[pallet::weight(T::WeightInfo::set_staking_configs())]
		pub fn set_staking_configs(
			origin: OriginFor<T>,
			min_nominator_bond: RingBalance<T>,
			min_validator_bond: RingBalance<T>,
			max_nominator_count: Option<u32>,
			max_validator_count: Option<u32>,
			chill_threshold: Option<Percent>,
			min_commission: Perbill,
		) -> DispatchResult {
			ensure_root(origin)?;

			<MinNominatorBond<T>>::set(min_nominator_bond);
			<MinValidatorBond<T>>::set(min_validator_bond);
			<MaxNominatorsCount<T>>::set(max_nominator_count);
			<MaxValidatorsCount<T>>::set(max_validator_count);
			ChillThreshold::<T>::set(chill_threshold);
			<MinCommission<T>>::set(min_commission);

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
					let current_nominator_count = <Nominators<T>>::count();
					ensure!(
						threshold * max_nominator_count < current_nominator_count,
						<Error<T>>::CannotChillOther
					);

					<MinNominatorBond<T>>::get()
				} else if <Validators<T>>::contains_key(&stash) {
					let max_validator_count =
						<MaxValidatorsCount<T>>::get().ok_or(<Error<T>>::CannotChillOther)?;
					let current_validator_count = <Validators<T>>::count();
					ensure!(
						threshold * max_validator_count < current_validator_count,
						<Error<T>>::CannotChillOther
					);

					<MinValidatorBond<T>>::get()
				} else {
					Zero::zero()
				};

				ensure!(ledger.active < min_active_bond, <Error<T>>::CannotChillOther);
			}

			Self::chill_stash(&stash);

			Ok(())
		}
	}

	/// Check that list is sorted and has no duplicates.
	fn is_sorted_and_unique(list: &Vec<u32>) -> bool {
		list.windows(2).all(|w| w[0] < w[1])
	}
}
pub use pallet::*;
