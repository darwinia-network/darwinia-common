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

mod types {
	// --- paritytech ---
	use frame_support::traits::Currency;
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

	// pub type StakingLedgerT<T> =
	// StakingLedger<AccountId<T>, RingBalance<T>, KtonBalance<T>, BlockNumberFor<T>>;
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
	// --- crates.io ---
	use codec::HasCompact;
	// --- paritytech ---
	use frame_election_provider_support::ElectionProvider;
	use frame_support::{
		pallet_prelude::*,
		traits::{EstimateNextNewSession, OnUnbalanced, UnixTime},
		PalletId,
	};
	use frame_system::offchain::SendTransactionTypes;
	use sp_runtime::traits::Convert;
	use sp_staking::SessionIndex;
	// --- darwinia-network ---
	use crate::*;
	use darwinia_support::balance::LockableCurrency;

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
	pub enum Event<T: Config> {}

	/// The active era information, it holds index and start.
	///
	/// The active era is the era being currently rewarded. Validator set of this era must be
	/// equal to [`SessionInterface::validators`].
	#[pallet::storage]
	#[pallet::getter(fn active_era)]
	pub type ActiveEra<T> = StorageValue<_, ActiveEraInfo>;

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
		T::AccountId,
		ExposureT<T>,
		ValueQuery,
	>;

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
	impl<T: Config> Convert<T::AccountId, Option<ExposureT<T>>> for ExposureOf<T> {
		fn convert(validator: T::AccountId) -> Option<ExposureT<T>> {
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
		who: AccountId,
		/// Amount of funds exposed.
		#[codec(compact)]
		ring_balance: RingBalance,
		#[codec(compact)]
		kton_balance: KtonBalance,
		power: Power,
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
}
pub use pallet::*;
