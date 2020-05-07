//! # Staking Module
//!
//! The Staking module is used to manage funds at stake by network maintainers.
//!
//! - [`staking::Trait`](./trait.Trait.html)
//! - [`Call`](./enum.Call.html)
//! - [`Module`](./struct.Module.html)
//!
//! ## Overview
//!
//! The Staking module is the means by which a set of network maintainers (known as _authorities_
//! in some contexts and _validators_ in others) are chosen based upon those who voluntarily place
//! funds under deposit. Under deposit, those funds are rewarded under normal operation but are
//! held at pain of _slash_ (expropriation) should the staked maintainer be found not to be
//! discharging its duties properly.
//!
//! ### Terminology
//! <!-- Original author of paragraph: @gavofyork -->
//!
//! - Staking: The process of locking up funds for some time, placing them at risk of slashing
//! (loss) in order to become a rewarded maintainer of the network.
//! - Validating: The process of running a node to actively maintain the network, either by
//! producing blocks or guaranteeing finality of the chain.
//! - Nominating: The process of placing staked funds behind one or more validators in order to
//! share in any reward, and punishment, they take.
//! - Stash account: The account holding an owner's funds used for staking.
//! - Controller account: The account that controls an owner's funds for staking.
//! - Era: A (whole) number of sessions, which is the period that the validator set (and each
//! validator's active nominator set) is recalculated and where rewards are paid out.
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
//! Almost any interaction with the Staking module requires a process of _**bonding**_ (also known
//! as being a _staker_). To become *bonded*, a fund-holding account known as the _stash account_,
//! which holds some or all of the funds that become frozen in place as part of the staking process,
//! is paired with an active **controller** account, which issues instructions on how they shall be
//! used.
//!
//! An account pair can become bonded using the [`bond`](./enum.Call.html#variant.bond) call.
//!
//! Stash accounts can change their associated controller using the
//! [`set_controller`](./enum.Call.html#variant.set_controller) call.
//!
//! There are three possible roles that any staked account pair can be in: `Validator`, `Nominator`
//! and `Idle` (defined in [`StakerStatus`](./enum.StakerStatus.html)). There are three
//! corresponding instructions to change between roles, namely:
//! [`validate`](./enum.Call.html#variant.validate),
//! [`nominate`](./enum.Call.html#variant.nominate), and [`chill`](./enum.Call.html#variant.chill).
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
//! [`validate`](./enum.Call.html#variant.validate) call.
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
//! An account can become a nominator via the [`nominate`](enum.Call.html#variant.nominate) call.
//!
//! #### Rewards and Slash
//!
//! The **reward and slashing** procedure is the core of the Staking module, attempting to _embrace
//! valid behavior_ while _punishing any misbehavior or lack of availability_.
//!
//! Rewards must be claimed for each era before it gets too old by `$HISTORY_DEPTH` using the
//! `payout_stakers` call. Any account can call `payout_stakers`, which pays the reward to
//! the validator as well as its nominators.
//! Only the [`T::MaxNominatorRewardedPerValidator`] biggest stakers can claim their reward. This
//! is to limit the i/o cost to mutate storage for each nominator's account.
//!
//! Slashing can occur at any point in time, once misbehavior is reported. Once slashing is
//! determined, a value is deducted from the balance of the validator and all the nominators who
//! voted for this validator (values are deducted from the _stash_ account of the slashed entity).
//!
//! Slashing logic is further described in the documentation of the `slashing` module.
//!
//! Similar to slashing, rewards are also shared among a validator and its associated nominators.
//! Yet, the reward funds are not always transferred to the stash account and can be configured.
//! See [Reward Calculation](#reward-calculation) for more details.
//!
//! #### Chilling
//!
//! Finally, any of the roles above can choose to step back temporarily and just chill for a while.
//! This means that if they are a nominator, they will not be considered as voters anymore and if
//! they are validators, they will no longer be a candidate for the next election.
//!
//! An account can step back via the [`chill`](enum.Call.html#variant.chill) call.
//!
//! ### Session managing
//!
//! The module implement the trait `SessionManager`. Which is the only API to query new validator
//! set and allowing these validator set to be rewarded once their era is ended.
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! The dispatchable functions of the Staking module enable the steps needed for entities to accept
//! and change their role, alongside some helper functions to get/set the metadata of the module.
//!
//! ### Public Functions
//!
//! The Staking module contains many public storage items and (im)mutable functions.
//!
//! ## Usage
//!
//! ### Example: Rewarding a validator by id.
//!
//! ```
//! use frame_support::{decl_module, dispatch};
//! use frame_system::{self as system, ensure_signed};
//! use darwinia_staking as staking;
//!
//! pub trait Trait: staking::Trait {}
//!
//! decl_module! {
//! 	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
//!			/// Reward a validator.
//! 		#[weight = 0]
//! 		pub fn reward_myself(origin) -> dispatch::DispatchResult {
//! 			let reported = ensure_signed(origin)?;
//! 			<staking::Module<T>>::reward_by_ids(vec![(reported, 10)]);
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
//! [`T::RewardCurve`](./trait.Trait.html#associatedtype.RewardCurve) as such:
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
//! [`T::RewardRemainder`](./trait.Trait.html#associatedtype.RewardRemainder).
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
//! [`reward_by_ids`](./enum.Call.html#variant.reward_by_ids) or
//! [`reward_by_indices`](./enum.Call.html#variant.reward_by_indices).
//!
//! [`Module`](./struct.Module.html) implements
//! [`pallet_authorship::EventHandler`](../pallet_authorship/trait.EventHandler.html) to add reward
//! points to block producer and block producer of referenced uncles.
//!
//! The validator and its nominator split their reward as following:
//!
//! The validator can declare an amount, named
//! [`commission`](./struct.ValidatorPrefs.html#structfield.commission), that does not
//! get shared with the nominators at each reward payout through its
//! [`ValidatorPrefs`](./struct.ValidatorPrefs.html). This value gets deducted from the total reward
//! that is paid to the validator and its nominators. The remaining portion is split among the
//! validator and all of the nominators that nominated the validator, proportional to the value
//! staked behind this validator (_i.e._ dividing the
//! [`own`](./struct.Exposure.html#structfield.own) or
//! [`others`](./struct.Exposure.html#structfield.others) by
//! [`total`](./struct.Exposure.html#structfield.total) in [`Exposure`](./struct.Exposure.html)).
//!
//! All entities who receive a reward have the option to choose their reward destination
//! through the [`Payee`](./struct.Payee.html) storage item (see
//! [`set_payee`](enum.Call.html#variant.set_payee)), to be one of the following:
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
//! [`unbond`](enum.Call.html#variant.unbond) call. Note that the funds are not immediately
//! accessible. Instead, a duration denoted by [`BondingDurationInEra`](./struct.BondingDurationInEra.html)
//! (in number of eras) must pass until the funds can actually be removed. Once the
//! `BondingDurationInEra` is over, the [`withdraw_unbonded`](./enum.Call.html#variant.withdraw_unbonded)
//! call can be used to actually withdraw the funds.
//!
//! Note that there is a limitation to the number of fund-chunks that can be scheduled to be
//! unlocked in the future via [`unbond`](enum.Call.html#variant.unbond). In case this maximum
//! (`MAX_UNLOCKING_CHUNKS`) is reached, the bonded account _must_ first wait until a successful
//! call to `withdraw_unbonded` to remove some of the chunks.
//!
//! ### Election Algorithm
//!
//! The current election algorithm is implemented based on Phragmén.
//! The reference implementation can be found
//! [here](https://github.com/w3f/consensus/tree/master/NPoS).
//!
//! The election algorithm, aside from electing the validators with the most stake value and votes,
//! tries to divide the nominator votes among candidates in an equal manner. To further assure this,
//! an optional post-processing can be applied that iteratively normalizes the nominator staked
//! values until the total difference among votes of a particular nominator are less than a
//! threshold.
//!
//! ## GenesisConfig
//!
//! The Staking module depends on the [`GenesisConfig`](./struct.GenesisConfig.html).
//! The `GenesisConfig` is optional and allow to set some initial stakers.
//!
//! ## Related Modules
//!
//! - [Balances](../pallet_balances/index.html): Used to manage values at stake.
//! - [Session](../pallet_session/index.html): Used to manage sessions. Also, a list of new
//!   validators is stored in the Session module's `Validators` at the end of each era.

#![cfg_attr(not(feature = "std"), no_std)]
#![feature(drain_filter)]
#![recursion_limit = "128"]

// #[cfg(any(feature = "runtime-benchmarks", test))]
// pub mod benchmarking;
// TODO: offchain phragmen test https://github.com/darwinia-network/darwinia-common/issues/97
// #[cfg(features = "testing-utils")]
// pub mod testing_utils;

pub mod inflation;
pub mod offchain_election;
pub mod slashing;

// #[cfg(test)]
// mod darwinia_tests;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod substrate_tests;

mod types {
	// --- darwinia ---
	use crate::*;

	/// Counter for the number of eras that have passed.
	pub type EraIndex = u32;
	/// Counter for the number of "reward" points earned by a given validator.
	pub type RewardPoint = u32;

	/// Data type used to index nominators in the compact type
	pub type NominatorIndex = u32;
	/// Data type used to index validators in the compact type.
	pub type ValidatorIndex = u16;
	// Ensure the size of both ValidatorIndex and NominatorIndex. They both need to be well below usize.
	static_assertions::const_assert!(size_of::<ValidatorIndex>() <= size_of::<usize>());
	static_assertions::const_assert!(size_of::<NominatorIndex>() <= size_of::<usize>());

	// Note: Maximum nomination limit is set here -- 16.
	generate_compact_solution_type!(pub GenericCompactAssignments, 16);
	/// Accuracy used for on-chain phragmen.
	pub type ChainAccuracy = Perbill;
	/// Accuracy used for off-chain phragmen. This better be small.
	pub type OffchainAccuracy = PerU16;
	/// The compact type for election solutions.
	pub type CompactAssignments =
		GenericCompactAssignments<NominatorIndex, ValidatorIndex, OffchainAccuracy>;

	/// Balance of an account.
	pub type Balance = u128;
	/// Power of an account.
	pub type Power = u32;
	/// A timestamp: milliseconds since the unix epoch.
	/// `u64` is enough to represent a duration of half a billion years, when the
	/// time scale is milliseconds.
	pub type TsInMs = u64;

	pub type AccountId<T> = <T as frame_system::Trait>::AccountId;
	pub type BlockNumber<T> = <T as frame_system::Trait>::BlockNumber;

	/// The balance type of this module.
	pub type RingBalance<T> = <RingCurrency<T> as Currency<AccountId<T>>>::Balance;
	pub type RingPositiveImbalance<T> =
		<RingCurrency<T> as Currency<AccountId<T>>>::PositiveImbalance;
	pub type RingNegativeImbalance<T> =
		<RingCurrency<T> as Currency<AccountId<T>>>::NegativeImbalance;

	/// The balance type of this module.
	pub type KtonBalance<T> = <KtonCurrency<T> as Currency<AccountId<T>>>::Balance;
	pub type KtonPositiveImbalance<T> =
		<KtonCurrency<T> as Currency<AccountId<T>>>::PositiveImbalance;
	pub type KtonNegativeImbalance<T> =
		<KtonCurrency<T> as Currency<AccountId<T>>>::NegativeImbalance;

	pub type StakingLedgerT<T> =
		StakingLedger<AccountId<T>, RingBalance<T>, KtonBalance<T>, BlockNumber<T>>;
	pub type StakingBalanceT<T> = StakingBalance<RingBalance<T>, KtonBalance<T>>;

	pub type ExposureT<T> = Exposure<AccountId<T>, RingBalance<T>, KtonBalance<T>>;
	pub type ElectionResultT<T> = ElectionResult<AccountId<T>, RingBalance<T>, KtonBalance<T>>;

	type RingCurrency<T> = <T as Trait>::RingCurrency;
	type KtonCurrency<T> = <T as Trait>::KtonCurrency;
}

// --- darwinia ---
pub use types::EraIndex;

// --- crates ---
use codec::{Decode, Encode, HasCompact};
// --- substrate ---
use frame_support::{
	debug, decl_error, decl_event, decl_module, decl_storage,
	dispatch::IsSubType,
	ensure,
	storage::IterableStorageMap,
	traits::{
		Currency, EnsureOrigin, EstimateNextNewSession, ExistenceRequirement::KeepAlive, Get,
		Imbalance, OnUnbalanced, UnixTime,
	},
	weights::{DispatchClass, Weight},
};
use frame_system::{
	self as system, ensure_none, ensure_root, ensure_signed, offchain::SendTransactionTypes,
};
use sp_phragmen::{
	build_support_map, elect, evaluate_support, generate_compact_solution_type, is_score_better,
	Assignment, ExtendedBalance, PhragmenResult, PhragmenScore, SupportMap, VoteWeight,
	VotingLimit,
};
use sp_runtime::{
	helpers_128bit::multiply_by_rational,
	traits::{
		AtLeast32Bit, CheckedSub, Convert, Dispatchable, SaturatedConversion, Saturating,
		StaticLookup, Zero,
	},
	transaction_validity::{
		InvalidTransaction, TransactionPriority, TransactionSource, TransactionValidity,
		TransactionValidityError, ValidTransaction,
	},
	DispatchResult, PerThing, PerU16, Perbill, Perquintill, RuntimeDebug,
};
#[cfg(feature = "std")]
use sp_runtime::{Deserialize, Serialize};
use sp_staking::{
	offence::{Offence, OffenceDetails, OffenceError, OnOffenceHandler, ReportOffence},
	SessionIndex,
};
use sp_std::{
	collections::btree_map::BTreeMap, convert::TryInto, marker::PhantomData, mem::size_of,
	prelude::*,
};
// --- darwinia ---
use darwinia_support::{
	balance::lock::*,
	traits::{OnDepositRedeem, OnUnbalancedKton},
};
use types::*;

// syntactic sugar for logging
#[cfg(feature = "std")]
const LOG_TARGET: &'static str = "staking";
macro_rules! log {
	($level:tt, $patter:expr $(, $values:expr)* $(,)?) => {
		debug::native::$level!(
			target: LOG_TARGET,
			$patter $(, $values)*
		)
	};
}

pub(crate) const MAX_UNLOCKING_CHUNKS: usize = 32;
/// Maximum number of stakers that can be stored in a snapshot.
pub(crate) const MAX_VALIDATORS: usize = ValidatorIndex::max_value() as usize;
pub(crate) const MAX_NOMINATORS: usize = NominatorIndex::max_value() as usize;

const DEFAULT_MINIMUM_VALIDATOR_COUNT: u32 = 4;
const MONTH_IN_MINUTES: TsInMs = 30 * 24 * 60;
const MONTH_IN_MILLISECONDS: TsInMs = MONTH_IN_MINUTES * 60 * 1000;
const STAKING_ID: LockIdentifier = *b"da/staki";

#[macro_export]
macro_rules! unix_time_now {
	() => {{
		match () {
			#[cfg(feature = "build-spec")]
			_ => $crate::unix_time_now!(build_spec),
			#[cfg(not(feature = "build-spec"))]
			_ => $crate::unix_time_now!(product),
			}
		}};
	(build_spec) => {{
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.expect("`now` always greater than `UNIX_EPOCH`, never get panic; qed")
			.as_millis()
			.saturated_into::<TsInMs>()
		}};
	(product) => {{
		T::UnixTime::now().as_millis().saturated_into::<TsInMs>()
		}};
}

// --- enum ---

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

/// A destination account for payment.
#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
pub enum RewardDestination {
	/// Pay into the stash account, increasing the amount at stake accordingly.
	Staked { promise_month: u8 },
	/// Pay into the stash account, not increasing the amount at stake.
	Stash,
	/// Pay into the controller account.
	Controller,
}

impl Default for RewardDestination {
	fn default() -> Self {
		RewardDestination::Staked { promise_month: 0 }
	}
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

/// Indicate how an election round was computed.
#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode, RuntimeDebug)]
pub enum ElectionCompute {
	/// Result was forcefully computed on chain at the end of the session.
	OnChain,
	/// Result was submitted and accepted to the chain via a signed transaction.
	Signed,
	/// Result was submitted and accepted to the chain via an unsigned transaction (by an
	/// authority).
	Unsigned,
}

/// The status of the upcoming (offchain) election.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub enum ElectionStatus<BlockNumber> {
	/// Nothing has and will happen for now. submission window is not open.
	Closed,
	/// The submission window has been open since the contained block number.
	Open(BlockNumber),
}

impl<BlockNumber: PartialEq> ElectionStatus<BlockNumber> {
	fn is_open_at(&self, n: BlockNumber) -> bool {
		*self == Self::Open(n)
	}

	fn is_closed(&self) -> bool {
		match self {
			Self::Closed => true,
			_ => false,
		}
	}

	fn is_open(&self) -> bool {
		!self.is_closed()
	}
}

impl<BlockNumber> Default for ElectionStatus<BlockNumber> {
	fn default() -> Self {
		Self::Closed
	}
}

/// To unify *Ring* and *Kton* balances.
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
	RingBalance: Default + HasCompact,
	KtonBalance: Default + HasCompact,
{
	fn default() -> Self {
		StakingBalance::RingBalance(Default::default())
	}
}

// --- struct ---

/// Information regarding the active era (era in used in session).
#[derive(Debug, Encode, Decode)]
pub struct ActiveEraInfo {
	/// Index of era.
	pub index: EraIndex,
	/// Moment of start expresed as millisecond from `$UNIX_EPOCH`.
	///
	/// Start can be none if start hasn't been set for the era yet,
	/// Start is set on the first on_finalize of the era to guarantee usage of `Time`.
	start: Option<TsInMs>,
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

/// Preference of what happens regarding validation.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct ValidatorPrefs {
	/// Reward that validator takes up-front; only the rest is split between themselves and
	/// nominators.
	#[codec(compact)]
	pub commission: Perbill,
}

impl Default for ValidatorPrefs {
	fn default() -> Self {
		ValidatorPrefs {
			commission: Default::default(),
		}
	}
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
	// active time-deposit ring
	#[codec(compact)]
	pub active_deposit_ring: RingBalance,

	/// The total amount of the stash's *KTON* that will be at stake in any forthcoming
	/// rounds.
	#[codec(compact)]
	pub active_kton: KtonBalance,

	// If you deposit *RING* for a minimum period,
	// you can get *KTON* as bonus which can also be used for staking.
	pub deposit_items: Vec<TimeDepositItem<RingBalance>>,

	// TODO doc
	pub ring_staking_lock: StakingLock<RingBalance, BlockNumber>,
	// TODO doc
	pub kton_staking_lock: StakingLock<KtonBalance, BlockNumber>,

	/// List of eras for which the stakers behind a validator have claimed rewards. Only updated
	/// for validators.
	pub claimed_rewards: Vec<EraIndex>,
}

impl<AccountId, RingBalance, KtonBalance, BlockNumber>
	StakingLedger<AccountId, RingBalance, KtonBalance, BlockNumber>
where
	RingBalance: AtLeast32Bit + Saturating + Copy,
	KtonBalance: AtLeast32Bit + Saturating + Copy,
	BlockNumber: PartialOrd,
	TsInMs: PartialOrd,
{
	/// Slash the validator for a given amount of balance. This can grow the value
	/// of the slash in the case that the validator has less than `minimum_balance`
	/// active funds. Returns the amount of funds actually slashed.
	///
	/// Slashes from `active` funds first, and then `unlocking`, starting with the
	/// chunks that are closest to unlocking.
	fn slash(
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
									item.value -= sp_std::mem::replace(
										&mut slashable_deposit_ring,
										Zero::zero(),
									);
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
			ring_staking_lock.unbondings.drain_filter(|lock| {
				if bn >= lock.until {
					true
				} else {
					if apply_slash_ring.is_zero() {
						false
					} else {
						if apply_slash_ring > lock.amount {
							apply_slash_ring -= lock.amount;
							true
						} else {
							lock.amount -=
								sp_std::mem::replace(&mut apply_slash_ring, Zero::zero());
							false
						}
					}
				}
			});
		}
		if !apply_slash_kton.is_zero() {
			kton_staking_lock.unbondings.drain_filter(|lock| {
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
							lock.amount -=
								sp_std::mem::replace(&mut apply_slash_kton, Zero::zero());
							false
						}
					}
				}
			});
		}

		(slash_ring - apply_slash_ring, slash_kton - apply_slash_kton)
	}
}

/// The *Ring* under deposit.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct TimeDepositItem<RingBalance: HasCompact> {
	#[codec(compact)]
	pub value: RingBalance,
	#[codec(compact)]
	pub start_time: TsInMs,
	#[codec(compact)]
	pub expire_time: TsInMs,
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
	/// Whether the nominations have been suppressed.
	pub suppressed: bool,
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

/// A pending slash record. The value of the slash has been computed but not applied yet,
/// rather deferred for several eras.
#[derive(Encode, Decode, Default, RuntimeDebug)]
pub struct UnappliedSlash<AccountId, RingBalance, KtonBalance> {
	/// The stash ID of the offending validator.
	validator: AccountId,
	/// The validator's own slash.
	own: slashing::RK<RingBalance, KtonBalance>,
	/// All other slashed stakers and amounts.
	others: Vec<(AccountId, slashing::RK<RingBalance, KtonBalance>)>,
	/// Reporters of the offence; bounty payout recipients.
	reporters: Vec<AccountId>,
	/// The amount of payout.
	payout: slashing::RK<RingBalance, KtonBalance>,
}

/// The result of an election round.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct ElectionResult<AccountId, RingBalance: HasCompact, KtonBalance: HasCompact> {
	/// Flat list of validators who have been elected.
	elected_stashes: Vec<AccountId>,
	/// Flat list of new exposures, to be updated in the [`Exposure`] storage.
	exposures: Vec<(AccountId, Exposure<AccountId, RingBalance, KtonBalance>)>,
	/// Type of the result. This is kept on chain only to track and report the best score's
	/// submission type. An optimisation could remove this.
	compute: ElectionCompute,
}

// --- trait ---

/// Means for interacting with a specialized version of the `session` trait.
///
/// This is needed because `Staking` sets the `ValidatorIdOf` of the `pallet_session::Trait`
pub trait SessionInterface<AccountId>: frame_system::Trait {
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

impl<T: Trait> SessionInterface<AccountId<T>> for T
where
	T: pallet_session::Trait<ValidatorId = AccountId<T>>,
	T: pallet_session::historical::Trait<
		FullIdentification = Exposure<AccountId<T>, RingBalance<T>, KtonBalance<T>>,
		FullIdentificationOf = ExposureOf<T>,
	>,
	T::SessionHandler: pallet_session::SessionHandler<AccountId<T>>,
	T::SessionManager: pallet_session::SessionManager<AccountId<T>>,
	T::ValidatorIdOf: Convert<AccountId<T>, Option<AccountId<T>>>,
{
	fn disable_validator(validator: &AccountId<T>) -> Result<bool, ()> {
		<pallet_session::Module<T>>::disable(validator)
	}

	fn validators() -> Vec<AccountId<T>> {
		<pallet_session::Module<T>>::validators()
	}

	fn prune_historical_up_to(up_to: SessionIndex) {
		<pallet_session::historical::Module<T>>::prune_up_to(up_to);
	}
}

pub trait Trait: frame_system::Trait + SendTransactionTypes<Call<Self>> {
	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

	/// Time used for computing era duration.
	///
	/// It is guaranteed to start being called from the first `on_finalize`. Thus value at genesis
	/// is not used.
	type UnixTime: UnixTime;

	/// Number of sessions per era.
	type SessionsPerEra: Get<SessionIndex>;

	/// Number of eras that staked funds must remain bonded for.
	type BondingDurationInEra: Get<EraIndex>;
	/// Number of eras that staked funds must remain bonded for.
	type BondingDurationInBlockNumber: Get<Self::BlockNumber>;

	/// Number of eras that slashes are deferred by, after computation. This
	/// should be less than the bonding duration. Set to 0 if slashes should be
	/// applied immediately, without opportunity for intervention.
	type SlashDeferDuration: Get<EraIndex>;

	/// The origin which can cancel a deferred slash. Root can always do this.
	type SlashCancelOrigin: EnsureOrigin<Self::Origin>;

	/// Interface for interacting with a session module.
	type SessionInterface: self::SessionInterface<Self::AccountId>;

	/// Something that can estimate the next session change, accurately or as a best effort guess.
	type NextNewSession: EstimateNextNewSession<Self::BlockNumber>;

	/// How many blocks ahead of the era, within the last do we try to run the phragmen offchain?
	/// Setting this to zero will disable the offchain compute and only on-chain seq-phragmen will
	/// be used.
	type ElectionLookahead: Get<Self::BlockNumber>;

	/// The overarching call type.
	type Call: Dispatchable + From<Call<Self>> + IsSubType<Module<Self>, Self> + Clone;

	/// Maximum number of equalise iterations to run in the offchain submission. If set to 0,
	/// equalize will not be executed at all.
	type MaxIterations: Get<u32>;

	/// The maximum number of nominator rewarded for each validator.
	///
	/// For each validator only the `$MaxNominatorRewardedPerValidator` biggest stakers can claim
	/// their reward. This used to limit the i/o cost for the nominator payout.
	type MaxNominatorRewardedPerValidator: Get<u32>;

	/// A configuration for base priority of unsigned transactions.
	///
	/// This is exposed so that it can be tuned for particular runtime, when
	/// multiple pallets send unsigned transactions.
	type UnsignedPriority: Get<TransactionPriority>;

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
	// FIXME: Ugly hack due to https://github.com/rust-lang/rust/issues/31844#issuecomment-557918823
	/// Handler for the unbalanced *KTON* reduction when slashing a staker.
	type KtonSlash: OnUnbalancedKton<KtonNegativeImbalance<Self>>;
	/// Handler for the unbalanced *KTON* increment when rewarding a staker.
	type KtonReward: OnUnbalanced<KtonPositiveImbalance<Self>>;

	// TODO: doc
	type Cap: Get<RingBalance<Self>>;
	// TODO: doc
	type TotalPower: Get<Power>;
}

decl_storage! {
	trait Store for Module<T: Trait> as DarwiniaStaking {
		/// Number of eras to keep in history.
		///
		/// Information is kept for eras in `[current_era - history_depth; current_era]`.
		///
		/// Must be more than the number of eras delayed by session otherwise.
		/// I.e. active era must always be in history.
		/// I.e. `active_era > current_era - history_depth` must be guaranteed.
		HistoryDepth get(fn history_depth) config(): u32 = 336;

		/// The ideal number of staking participants.
		pub ValidatorCount get(fn validator_count) config(): u32;

		/// Minimum number of staking participants before emergency conditions are imposed.
		pub MinimumValidatorCount
			get(fn minimum_validator_count) config()
			: u32 = DEFAULT_MINIMUM_VALIDATOR_COUNT;

		/// Any validators that may never be slashed or forcibly kicked. It's a Vec since they're
		/// easy to initialize and the performance hit is minimal (we expect no more than four
		/// invulnerables) and restricted to testnets.
		pub Invulnerables get(fn invulnerables) config(): Vec<T::AccountId>;

		/// Map from all locked "stash" accounts to the controller account.
		pub Bonded get(fn bonded): map hasher(twox_64_concat) T::AccountId => Option<T::AccountId>;

		/// Map from all (unlocked) "controller" accounts to the info regarding the staking.
		pub Ledger get(fn ledger): map hasher(blake2_128_concat) T::AccountId => Option<StakingLedgerT<T>>;

		/// Where the reward payment should be made. Keyed by stash.
		pub Payee get(fn payee): map hasher(twox_64_concat) T::AccountId => RewardDestination;

		/// The map from (wannabe) validator stash key to the preferences of that validator.
		pub Validators
			get(fn validators)
			: map hasher(twox_64_concat) T::AccountId => ValidatorPrefs;

		/// The map from nominator stash key to the set of stash keys of all validators to nominate.
		pub Nominators
			get(fn nominators)
			: map hasher(twox_64_concat) T::AccountId => Option<Nominations<T::AccountId>>;

		/// The current era index.
		///
		/// This is the latest planned era, depending on how the Session pallet queues the validator
		/// set, it might be active or not.
		pub CurrentEra get(fn current_era): Option<EraIndex>;

		/// The active era information, it holds index and start.
		///
		/// The active era is the era currently rewarded.
		/// Validator set of this era must be equal to `SessionInterface::validators`.
		pub ActiveEra get(fn active_era): Option<ActiveEraInfo>;

		/// The session index at which the era start for the last `HISTORY_DEPTH` eras.
		pub ErasStartSessionIndex
			get(fn eras_start_session_index)
			: map hasher(twox_64_concat) EraIndex => Option<SessionIndex>;

		/// Exposure of validator at era.
		///
		/// This is keyed first by the era index to allow bulk deletion and then the stash account.
		///
		/// Is it removed after `HISTORY_DEPTH` eras.
		/// If stakers hasn't been set or has been removed then empty exposure is returned.
		pub ErasStakers
			get(fn eras_stakers)
			: double_map hasher(twox_64_concat) EraIndex, hasher(twox_64_concat) T::AccountId
				=> ExposureT<T>;

		/// Clipped Exposure of validator at era.
		///
		/// This is similar to [`ErasStakers`] but number of nominators exposed is reduced to the
		/// `T::MaxNominatorRewardedPerValidator` biggest stakers.
		/// This is used to limit the i/o cost for the nominator payout.
		///
		/// This is keyed fist by the era index to allow bulk deletion and then the stash account.
		///
		/// Is it removed after `HISTORY_DEPTH` eras.
		/// If stakers hasn't been set or has been removed then empty exposure is returned.
		pub ErasStakersClipped
			get(fn eras_stakers_clipped)
			: double_map hasher(twox_64_concat) EraIndex, hasher(twox_64_concat) T::AccountId
				=> ExposureT<T>;

		/// Similar to `ErasStakers`, this holds the preferences of validators.
		///
		/// This is keyed fist by the era index to allow bulk deletion and then the stash account.
		///
		/// Is it removed after `HISTORY_DEPTH` eras.
		// If prefs hasn't been set or has been removed then 0 commission is returned.
		pub ErasValidatorPrefs
			get(fn eras_validator_prefs)
			: double_map hasher(twox_64_concat) EraIndex, hasher(twox_64_concat) T::AccountId
				=> ValidatorPrefs;

		/// The total validator era payout for the last `HISTORY_DEPTH` eras.
		///
		/// Eras that haven't finished yet or has been removed doesn't have reward.
		pub ErasValidatorReward
			get(fn eras_validator_reward)
			: map hasher(twox_64_concat) EraIndex => Option<RingBalance<T>>;

		/// Rewards for the last `HISTORY_DEPTH` eras.
		/// If reward hasn't been set or has been removed then 0 reward is returned.
		pub ErasRewardPoints
			get(fn eras_reward_points)
			: map hasher(twox_64_concat) EraIndex => EraRewardPoints<T::AccountId>;

		/// The total amount staked for the last `HISTORY_DEPTH` eras.
		/// If total hasn't been set or has been removed then 0 stake is returned.
		pub ErasTotalStake
			get(fn eras_total_stake)
			: map hasher(twox_64_concat) EraIndex => Power;

		/// Mode of era forcing.
		pub ForceEra get(fn force_era) config(): Forcing;

		/// The percentage of the slash that is distributed to reporters.
		///
		/// The rest of the slashed value is handled by the `Slash`.
		pub SlashRewardFraction get(fn slash_reward_fraction) config(): Perbill;

		/// The amount of currency given to reporters of a slash event which was
		/// canceled by extraordinary circumstances (e.g. governance).
		pub CanceledSlashPayout get(fn canceled_payout) config(): Power;

		/// All unapplied slashes that are queued for later.
		pub UnappliedSlashes
			: map hasher(twox_64_concat) EraIndex
				=> Vec<UnappliedSlash<T::AccountId, RingBalance<T>, KtonBalance<T>>>;

		/// A mapping from still-bonded eras to the first session index of that era.
		///
		/// Must contains information for eras for the range:
		/// `[active_era - bounding_duration; active_era]`
		BondedEras: Vec<(EraIndex, SessionIndex)>;

		/// All slashing events on validators, mapped by era to the highest slash proportion
		/// and slash value of the era.
		ValidatorSlashInEra
			: double_map hasher(twox_64_concat) EraIndex, hasher(twox_64_concat) T::AccountId
				=> Option<(Perbill, slashing::RKT<T>)>;

		/// All slashing events on nominators, mapped by era to the highest slash value of the era.
		NominatorSlashInEra
			: double_map hasher(twox_64_concat) EraIndex, hasher(twox_64_concat) T::AccountId
				=> Option<slashing::RKT<T>>;

		/// Slashing spans for stash accounts.
		SlashingSpans: map hasher(twox_64_concat) T::AccountId => Option<slashing::SlashingSpans>;

		/// Records information about the maximum slash of a stash within a slashing span,
		/// as well as how much reward has been paid out.
		SpanSlash
			: map hasher(twox_64_concat) (T::AccountId, slashing::SpanIndex)
				=> slashing::SpanRecord<RingBalance<T>, KtonBalance<T>>;

		/// The earliest era for which we have a pending, unapplied slash.
		EarliestUnappliedSlash: Option<EraIndex>;

		/// Snapshot of validators at the beginning of the current election window. This should only
		/// have a value when [`EraElectionStatus`] == `ElectionStatus::Open(_)`.
		pub SnapshotValidators get(fn snapshot_validators): Option<Vec<T::AccountId>>;

		/// Snapshot of nominators at the beginning of the current election window. This should only
		/// have a value when [`EraElectionStatus`] == `ElectionStatus::Open(_)`.
		pub SnapshotNominators get(fn snapshot_nominators): Option<Vec<T::AccountId>>;

		/// The next validator set. At the end of an era, if this is available (potentially from the
		/// result of an offchain worker), it is immediately used. Otherwise, the on-chain election
		/// is executed.
		pub QueuedElected get(fn queued_elected): Option<ElectionResultT<T>>;

		/// The score of the current [`QueuedElected`].
		pub QueuedScore get(fn queued_score): Option<PhragmenScore>;

		/// Flag to control the execution of the offchain election. When `Open(_)`, we accept
		/// solutions to be submitted.
		pub EraElectionStatus get(fn era_election_status): ElectionStatus<T::BlockNumber>;

		/// True if the current **planned** session is final. Note that this does not take era
		/// forcing into account.
		pub IsCurrentSessionFinal get(fn is_current_session_final): bool = false;

		// --- darwinia ---

		// TODO: doc
		pub LivingTime get(fn living_time): TsInMs;

		/// The percentage of the total payout that is distributed to validators and nominators
		///
		/// The reset might go to Treasury or something else.
		pub PayoutFraction get(fn payout_fraction) config(): Perbill;

		/// Total *Ring* in pool.
		pub RingPool get(fn ring_pool): RingBalance<T>;
		/// Total *Kton* in pool.
		pub KtonPool get(fn kton_pool): KtonBalance<T>;
	}
	add_extra_genesis {
		config(stakers): Vec<(
			T::AccountId,
			T::AccountId,
			RingBalance<T>,
			StakerStatus<T::AccountId>
		)>;
		build(|config: &GenesisConfig<T>| {
			for &(ref stash, ref controller, ring_to_be_bonded, ref status) in &config.stakers {
				assert!(
					T::RingCurrency::free_balance(&stash) >= ring_to_be_bonded,
					"Stash does not have enough balance to bond.",
				);
				let _ = <Module<T>>::bond(
					T::Origin::from(Some(stash.to_owned()).into()),
					T::Lookup::unlookup(controller.to_owned()),
					StakingBalance::RingBalance(ring_to_be_bonded),
					RewardDestination::Staked { promise_month: 0 },
					0,
				);
				let _ = match status {
					StakerStatus::Validator => {
						<Module<T>>::validate(
							T::Origin::from(Some(controller.to_owned()).into()),
							Default::default(),
						)
					},
					StakerStatus::Nominator(votes) => {
						<Module<T>>::nominate(
							T::Origin::from(Some(controller.to_owned()).into()),
							votes.iter().map(|l| T::Lookup::unlookup(l.to_owned())).collect(),
						)
					}, _ => Ok(())
				};
			}
		});
	}
}

decl_event!(
	pub enum Event<T>
	where
		AccountId = AccountId<T>,
		BlockNumber = BlockNumber<T>,
		RingBalance = RingBalance<T>,
		KtonBalance = KtonBalance<T>,
	{
		/// The era payout has been set; the first balance is the validator-payout; the second is
		/// the remainder from the maximum amount of reward.
		EraPayout(EraIndex, RingBalance, RingBalance),

		/// The staker has been rewarded by this amount. `AccountId` is the stash account.
		Reward(AccountId, RingBalance),

		/// One validator (and its nominators) has been slashed by the given amount.
		Slash(AccountId, RingBalance, KtonBalance),
		/// An old slashing report from a prior era was discarded because it could
		/// not be processed.
		OldSlashingReportDiscarded(SessionIndex),

		/// A new set of stakers was elected with the given computation method.
		StakingElection(ElectionCompute),

		/// Bond succeed.
		/// `amount` in `RingBalance<T>`, `start_time` in `TsInMs`, `expired_time` in `TsInMs`
		BondRing(RingBalance, TsInMs, TsInMs),
		/// Bond succeed.
		/// `amount`
		BondKton(KtonBalance),

		/// Unbond succeed.
		/// `amount` in `RingBalance<T>`, `now` in `BlockNumber`
		UnbondRing(RingBalance, BlockNumber),
		/// Unbond succeed.
		/// `amount` om `KtonBalance<T>`, `now` in `BlockNumber`
		UnbondKton(KtonBalance, BlockNumber),

		/// Someone claimed his deposits with some *KTON*s punishment
		ClaimDepositsWithPunish(AccountId, KtonBalance),
	}
);

decl_error! {
	/// Error for the staking module.
	pub enum Error for Module<T: Trait> {
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
		/// The submitted result is received out of the open window.
		PhragmenEarlySubmission,
		/// The submitted result is not as good as the one stored on chain.
		PhragmenWeakSubmission,
		/// The snapshot data of the current window is missing.
		SnapshotUnavailable,
		/// Incorrect number of winners were presented.
		PhragmenBogusWinnerCount,
		/// One of the submitted winners is not an active candidate on chain (index is out of range
		/// in snapshot).
		PhragmenBogusWinner,
		/// Error while building the assignment type from the compact. This can happen if an index
		/// is invalid, or if the weights _overflow_.
		PhragmenBogusCompact,
		/// One of the submitted nominators is not an active nominator on chain.
		PhragmenBogusNominator,
		/// One of the submitted nominators has an edge to which they have not voted on chain.
		PhragmenBogusNomination,
		/// One of the submitted nominators has an edge which is submitted before the last non-zero
		/// slash of the target.
		PhragmenSlashedNomination,
		/// A self vote must only be originated from a validator to ONLY themselves.
		PhragmenBogusSelfVote,
		/// The submitted result has unknown edges that are not among the presented winners.
		PhragmenBogusEdge,
		/// The claimed score does not match with the one computed from the data.
		PhragmenBogusScore,
		/// The call is not allowed at the given time due to restrictions of election period.
		CallNotAllowed,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		/// Number of sessions per era.
		const SessionsPerEra: SessionIndex = T::SessionsPerEra::get();

		/// Number of eras that staked funds must remain bonded for.
		const BondingDurationInEra: EraIndex = T::BondingDurationInEra::get();
		/// Number of BlockNumbers that staked funds must remain bonded for.
		const BondingDurationInBlockNumber: T::BlockNumber = T::BondingDurationInBlockNumber::get();

		// TODO: doc
		const Cap: RingBalance<T> = T::Cap::get();

		// TODO: doc
		const TotalPower: Power = T::TotalPower::get();

		fn deposit_event() = default;

		/// sets `ElectionStatus` to `Open(now)` where `now` is the block number at which the
		/// election window has opened, if we are at the last session and less blocks than
		/// `T::ElectionLookahead` is remaining until the next new session schedule. The offchain
		/// worker, if applicable, will execute at the end of the current block, and solutions may
		/// be submitted.
		fn on_initialize(now: T::BlockNumber) -> Weight {
			if
				// if we don't have any ongoing offchain compute.
				Self::era_election_status().is_closed() &&
				// either current session final based on the plan, or we're forcing.
				(Self::is_current_session_final() || Self::will_era_be_forced())
			{
				if let Some(next_session_change) = T::NextNewSession::estimate_next_new_session(now){
					if let Some(remaining) = next_session_change.checked_sub(&now) {
						if remaining <= T::ElectionLookahead::get() && !remaining.is_zero() {
							// create snapshot.
							if Self::create_stakers_snapshot() {
								// Set the flag to make sure we don't waste any compute here in the same era
								// after we have triggered the offline compute.
								<EraElectionStatus<T>>::put(
									ElectionStatus::<T::BlockNumber>::Open(now)
								);
								log!(
									info,
									"💸 Election window is Open({:?}). Snapshot created",
									now
								);
							} else {
								log!(warn, "💸 Failed to create snapshot at {:?}.", now);
							}
						}
					}
				} else {
					log!(warn, "💸 Estimating next session change failed.");
				}
			}

			// weight
			50_000
		}

		/// Check if the current block number is the one at which the election window has been set
		/// to open. If so, it runs the offchain worker code.
		fn offchain_worker(now: T::BlockNumber) {
			use offchain_election::{set_check_offchain_execution_status, compute_offchain_election};

			if Self::era_election_status().is_open_at(now) {
				let offchain_status = set_check_offchain_execution_status::<T>(now);
				if let Err(why) = offchain_status {
					log!(
						debug,
						"skipping offchain worker in open election window due to [{}]",
						why
					);
				} else {
					if let Err(e) = compute_offchain_election::<T>() {
						log!(warn, "💸 Error in phragmen offchain worker: {:?}", e);
					} else {
						log!(debug, "Executed offchain worker thread without errors.");
					}
				}

			}
		}

		fn on_finalize() {
			if let Some(mut active_era) = Self::active_era() {
				if active_era.start.is_none() {
					let now = unix_time_now!();
					active_era.start = Some(now);
					ActiveEra::put(active_era);
				}
			}
		}

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
		/// # </weight>
		#[weight = 500_000_000]
		fn bond(
			origin,
			controller: <T::Lookup as StaticLookup>::Source,
			value: StakingBalanceT<T>,
			payee: RewardDestination,
			promise_month: u8
		) {
			let stash = ensure_signed(origin)?;
			ensure!(!<Bonded<T>>::contains_key(&stash), <Error<T>>::AlreadyBonded);

			let controller = T::Lookup::lookup(controller)?;
			ensure!(!<Ledger<T>>::contains_key(&controller), <Error<T>>::AlreadyPaired);

			let ledger = StakingLedger {
				stash: stash.clone(),
				claimed_rewards: {
					let current_era = CurrentEra::get().unwrap_or(0);
					let last_reward_era = current_era.saturating_sub(Self::history_depth());
					(last_reward_era..current_era).collect()
				},
				..Default::default()
			};
			let promise_month = promise_month.min(36);

			match value {
				StakingBalance::RingBalance(r) => {
					// reject a bond which is considered to be _dust_.
					ensure!(
						r >= T::RingCurrency::minimum_balance(),
						<Error<T>>::InsufficientValue,
					);

					let usable_balance = T::RingCurrency::usable_balance(&stash);
					let value = r.min(usable_balance);
					let (start_time, expire_time) = Self::bond_ring(
						&stash,
						&controller,
						value,
						promise_month,
						ledger,
					);

					<RingPool<T>>::mutate(|r| *r += value);
					Self::deposit_event(RawEvent::BondRing(value, start_time, expire_time));
				},
				StakingBalance::KtonBalance(k) => {
					// reject a bond which is considered to be _dust_.
					ensure!(
						k >= T::KtonCurrency::minimum_balance(),
						<Error<T>>::InsufficientValue,
					);

					let usable_balance = T::KtonCurrency::usable_balance(&stash);
					let value = k.min(usable_balance);

					Self::bond_kton(&controller, value, ledger);

					<KtonPool<T>>::mutate(|k| *k += value);
					Self::deposit_event(RawEvent::BondKton(value));
				},
			}

			// You're auto-bonded forever, here. We might improve this by only bonding when
			// you actually validate/nominate and remove once you unbond __everything__.
			<Bonded<T>>::insert(&stash, &controller);
			<Payee<T>>::insert(&stash, payee);

			<frame_system::Module<T>>::inc_ref(&stash);
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
		/// # </weight>
		#[weight = 500_000_000]
		fn bond_extra(origin, max_additional: StakingBalanceT<T>, promise_month: u8) {
			ensure!(Self::era_election_status().is_closed(), <Error<T>>::CallNotAllowed);

			let stash = ensure_signed(origin)?;
			let controller = Self::bonded(&stash).ok_or(<Error<T>>::NotStash)?;
			let ledger = Self::ledger(&controller).ok_or(<Error<T>>::NotController)?;
			let promise_month = promise_month.min(36);

			match max_additional {
				 StakingBalance::RingBalance(r) => {
					let extra = T::RingCurrency::usable_balance(&stash);
					let extra = extra.min(r);
					let (start_time, expire_time) = Self::bond_ring(
						&stash,
						&controller,
						extra,
						promise_month,
						ledger,
					);

					<RingPool<T>>::mutate(|r| *r += extra);
					Self::deposit_event(RawEvent::BondRing(extra, start_time, expire_time));
				},
				StakingBalance::KtonBalance(k) => {
					let extra = T::KtonCurrency::usable_balance(&stash);
					let extra = extra.min(k);

					Self::bond_kton(&controller, extra, ledger);

					<KtonPool<T>>::mutate(|k| *k += extra);
					Self::deposit_event(RawEvent::BondKton(extra));
				},
			}
		}

		/// Deposit some extra amount ring, and return kton to the controller.
		///
		/// The dispatch origin for this call must be _Signed_ by the stash, not the controller.
		///
		/// # <weight>
		/// - Independent of the arguments. Insignificant complexity.
		/// - O(1).
		/// - One DB entry.
		/// # </weight>
		#[weight = 500_000_000]
		fn deposit_extra(origin, value: RingBalance<T>, promise_month: u8) {
			let stash = ensure_signed(origin)?;
			let controller = Self::bonded(&stash).ok_or(<Error<T>>::NotStash)?;
			let ledger = Self::ledger(&controller).ok_or(<Error<T>>::NotController)?;
			let start_time = unix_time_now!();
			let promise_month = promise_month.max(3).min(36);
			let expire_time = start_time + promise_month as TsInMs * MONTH_IN_MILLISECONDS;
			let mut ledger = Self::clear_mature_deposits(ledger);
			let StakingLedger {
				stash,
				active_ring,
				active_deposit_ring,
				deposit_items,
				..
			} = &mut ledger;
			let value = value.min(*active_ring - *active_deposit_ring);
			let kton_return = inflation::compute_kton_return::<T>(value, promise_month as _);
			let kton_positive_imbalance = T::KtonCurrency::deposit_creating(&stash, kton_return);

			T::KtonReward::on_unbalanced(kton_positive_imbalance);
			*active_deposit_ring += value;
			deposit_items.push(TimeDepositItem {
				value,
				start_time,
				expire_time,
			});

			<Ledger<T>>::insert(&controller, ledger);
			Self::deposit_event(RawEvent::BondRing(value, start_time, expire_time));
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
		/// </weight>
		///
		/// Only active normal ring can be unbond
		#[weight = 500_000_000]
		fn unbond(origin, value: StakingBalanceT<T>) {
			ensure!(Self::era_election_status().is_closed(), <Error<T>>::CallNotAllowed);

			let controller = ensure_signed(origin)?;
			let mut ledger = Self::clear_mature_deposits(Self::ledger(&controller).ok_or(<Error<T>>::NotController)?);
			let StakingLedger {
				active_ring,
				active_deposit_ring,
				active_kton,
				ring_staking_lock,
				kton_staking_lock,
				..
			} = &mut ledger;
			let now = <frame_system::Module<T>>::block_number();

			ring_staking_lock.update(now);
			kton_staking_lock.update(now);

			// Due to the macro parser, we've to add a bracket.
			// Actually, this's totally wrong:
			//	 `a as u32 + b as u32 < c`
			// Workaround:
			//	 1. `(a as u32 + b as u32) < c`
			//	 2. `let c_ = a as u32 + b as u32; c_ < c`
			ensure!(
				(ring_staking_lock.unbondings.len() + kton_staking_lock.unbondings.len()) < MAX_UNLOCKING_CHUNKS,
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
							&& (*active_kton < T::KtonCurrency::minimum_balance()) {
							unbond_ring += *active_ring;
							unbond_kton += *active_kton;

							*active_ring = Zero::zero();
							*active_kton = Zero::zero();
						}

						ring_staking_lock.unbondings.push(Unbonding {
							amount: unbond_ring,
							until: now + T::BondingDurationInBlockNumber::get(),
						});

						<RingPool<T>>::mutate(|r| *r -= unbond_ring);
						Self::deposit_event(RawEvent::UnbondRing(unbond_ring, now));

						if !unbond_kton.is_zero() {
							kton_staking_lock.unbondings.push(Unbonding {
								amount: unbond_kton,
								until: now + T::BondingDurationInBlockNumber::get(),
							});

							<KtonPool<T>>::mutate(|k| *k -= unbond_kton);
							Self::deposit_event(RawEvent::UnbondKton(unbond_kton, now));
						}
					}
				},
				StakingBalance::KtonBalance(k) => {
					unbond_kton = k.min(*active_kton);

					if !unbond_kton.is_zero() {
						*active_kton -= unbond_kton;

						// Avoid there being a dust balance left in the staking system.
						if (*active_kton < T::KtonCurrency::minimum_balance())
							&& (*active_ring < T::RingCurrency::minimum_balance()) {
							unbond_kton += *active_kton;
							unbond_ring += *active_ring;

							*active_kton = Zero::zero();
							*active_ring = Zero::zero();
						}

						kton_staking_lock.unbondings.push(Unbonding {
							amount: unbond_kton,
							until: now + T::BondingDurationInBlockNumber::get(),
						});


						<KtonPool<T>>::mutate(|k| *k -= unbond_kton);
						Self::deposit_event(RawEvent::UnbondKton(unbond_kton, now));

						if !unbond_ring.is_zero() {
							ring_staking_lock.unbondings.push(Unbonding {
								amount: unbond_ring,
								until: now + T::BondingDurationInBlockNumber::get(),
							});

							<RingPool<T>>::mutate(|k| *k -= unbond_ring);
							Self::deposit_event(RawEvent::UnbondRing(unbond_ring, now));
						}
					}
				},
			}

			Self::update_ledger(&controller, &mut ledger);

			let StakingLedger {
				active_ring,
				active_kton,
				..
			} = ledger;

			// All bonded *RING* and *KTON* is withdrawing, then remove Ledger to save storage
			if active_ring.is_zero() && active_kton.is_zero() {
				// TODO: https://github.com/darwinia-network/darwinia-common/issues/96
				// FIXME: https://github.com/darwinia-network/darwinia-common/issues/121
				//
				// `OnKilledAccount` would be a method to collect the locks.
				//
				// These locks are still in the system, and should be removed after 14 days
				//
				// There two situations should be considered after the 14 days
				// - the user never bond again, so the locks should be released.
				// - the user is bonded again in the 14 days, so the after 14 days
				//   the lock should not be removed
				//
				// If the locks are not deleted, this lock will waste the storage in the future
				// blocks.
				//
				// T::Ring::remove_lock(STAKING_ID, &stash);
				// T::Kton::remove_lock(STAKING_ID, &stash);
				// Self::kill_stash(&stash)?;
			}
		}

		/// Stash accounts can get their ring back after the depositing time exceeded,
		/// and the ring getting back is still in staking status.
		///
		/// # <weight>
		/// - Independent of the arguments. Insignificant complexity.
		/// - One storage read.
		/// - One storage write.
		/// - Writes are limited to the `origin` account key.
		/// # </weight>
		#[weight = 500_000_000]
		fn claim_mature_deposits(origin) {
			let controller = ensure_signed(origin)?;
			let ledger = Self::clear_mature_deposits(Self::ledger(&controller).ok_or(<Error<T>>::NotController)?);

			<Ledger<T>>::insert(controller, ledger);
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
		/// # </weight>
		#[weight = 750_000_000]
		fn try_claim_deposits_with_punish(origin, expire_time: TsInMs) {
			let controller = ensure_signed(origin)?;
			let mut ledger = Self::ledger(&controller).ok_or(<Error<T>>::NotController)?;
			let now = unix_time_now!();

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
							let plan_duration_in_milliseconds = item.expire_time.saturating_sub(item.start_time);
							plan_duration_in_milliseconds / MONTH_IN_MILLISECONDS
						};
						let passed_duration_in_months = {
							let passed_duration_in_milliseconds = now.saturating_sub(item.start_time);
							passed_duration_in_milliseconds / MONTH_IN_MILLISECONDS
						};

						(
							inflation::compute_kton_return::<T>(item.value, plan_duration_in_months)
							-
							inflation::compute_kton_return::<T>(item.value, passed_duration_in_months)
						).max(1.into()) * 3.into()
					};

					// check total free balance and locked one
					// strict on punishing in kton
					if T::KtonCurrency::free_balance(stash)
						.checked_sub(&kton_slash)
						.and_then(|new_balance| {
							T::KtonCurrency::ensure_can_withdraw(
								stash,
								kton_slash,
								WithdrawReason::Transfer.into(),
								new_balance
							).ok()
						})
						.is_some()
					{
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
				Self::deposit_event(
					RawEvent::ClaimDepositsWithPunish(
						ledger.stash.clone(),
						claim_deposits_with_punish.1,
					));
			}
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
		/// # </weight>
		#[weight = 750_000_000]
		fn validate(origin, prefs: ValidatorPrefs) {
			ensure!(Self::era_election_status().is_closed(), <Error<T>>::CallNotAllowed);

			let controller = ensure_signed(origin)?;
			let ledger = Self::ledger(&controller).ok_or(<Error<T>>::NotController)?;
			let stash = &ledger.stash;

			<Nominators<T>>::remove(stash);
			<Validators<T>>::insert(stash, prefs);
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
		/// - The transaction's complexity is proportional to the size of `targets`,
		/// which is capped at CompactAssignments::LIMIT.
		/// - Both the reads and writes follow a similar pattern.
		/// # </weight>
		#[weight = 750_000_000]
		fn nominate(origin, targets: Vec<<T::Lookup as StaticLookup>::Source>) {
			ensure!(Self::era_election_status().is_closed(), <Error<T>>::CallNotAllowed);

			let controller = ensure_signed(origin)?;
			let ledger = Self::ledger(&controller).ok_or(<Error<T>>::NotController)?;
			let stash = &ledger.stash;

			ensure!(!targets.is_empty(), <Error<T>>::EmptyTargets);

			let targets = targets.into_iter()
				.take(<CompactAssignments as VotingLimit>::LIMIT)
				.map(|t| T::Lookup::lookup(t))
				.collect::<Result<Vec<T::AccountId>, _>>()?;
			let nominations = Nominations {
				targets,
				// initial nominations are considered submitted at era 0. See `Nominations` doc
				submitted_in: Self::current_era().unwrap_or(0),
				suppressed: false,
			};

			<Validators<T>>::remove(stash);
			<Nominators<T>>::insert(stash, &nominations);
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
		/// # </weight>
		#[weight = 500_000_000]
		fn chill(origin) {
			ensure!(Self::era_election_status().is_closed(), <Error<T>>::CallNotAllowed);

			let controller = ensure_signed(origin)?;
			let ledger = Self::ledger(&controller).ok_or(<Error<T>>::NotController)?;

			Self::chill_stash(&ledger.stash);
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
		/// # </weight>
		#[weight = 500_000_000]
		fn set_payee(origin, payee: RewardDestination) {
			let controller = ensure_signed(origin)?;
			let ledger = Self::ledger(&controller).ok_or(<Error<T>>::NotController)?;
			let stash = &ledger.stash;

			<Payee<T>>::insert(stash, payee);
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
		/// # </weight>
		#[weight = 750_000_000]
		fn set_controller(origin, controller: <T::Lookup as StaticLookup>::Source) {
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
		}

		// --- root call ---

		/// The ideal number of validators.
		#[weight = 5_000_000]
		fn set_validator_count(origin, #[compact] new: u32) {
			ensure_root(origin)?;
			ValidatorCount::put(new);
		}

		/// Force there to be no new eras indefinitely.
		///
		/// # <weight>
		/// - No arguments.
		/// # </weight>
		#[weight = 5_000_000]
		fn force_no_eras(origin) {
			ensure_root(origin)?;
			ForceEra::put(Forcing::ForceNone);
		}

		/// Force there to be a new era at the end of the next session. After this, it will be
		/// reset to normal (non-forced) behaviour.
		///
		/// # <weight>
		/// - No arguments.
		/// # </weight>
		#[weight = 5_000_000]
		fn force_new_era(origin) {
			ensure_root(origin)?;
			ForceEra::put(Forcing::ForceNew);
		}

		/// Set the validators who cannot be slashed (if any).
		#[weight = 5_000_000]
		fn set_invulnerables(origin, validators: Vec<T::AccountId>) {
			ensure_root(origin)?;
			<Invulnerables<T>>::put(validators);
		}

		/// Force a current staker to become completely unstaked, immediately.
		#[weight = 0]
		fn force_unstake(origin, stash: T::AccountId) {
			ensure_root(origin)?;

			// remove all staking-related information.
			Self::kill_stash(&stash)?;

			// remove the lock.
			T::RingCurrency::remove_lock(STAKING_ID, &stash);
			T::KtonCurrency::remove_lock(STAKING_ID, &stash);
		}

		/// Force there to be a new era at the end of sessions indefinitely.
		///
		/// # <weight>
		/// - One storage write
		/// # </weight>
		#[weight = 5_000_000]
		fn force_new_era_always(origin) {
			ensure_root(origin)?;
			ForceEra::put(Forcing::ForceAlways);
		}

		/// Cancel enactment of a deferred slash. Can be called by either the root origin or
		/// the `T::SlashCancelOrigin`.
		/// passing the era and indices of the slashes for that era to kill.
		///
		/// # <weight>
		/// - One storage write.
		/// # </weight>
		#[weight = 1_000_000_000]
		fn cancel_deferred_slash(origin, era: EraIndex, slash_indices: Vec<u32>) {
			T::SlashCancelOrigin::try_origin(origin)
				.map(|_| ())
				.or_else(ensure_root)?;

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
		/// # </weight>
		#[weight = 500_000_000]
		fn payout_stakers(origin, validator_stash: T::AccountId, era: EraIndex) -> DispatchResult {
			ensure!(Self::era_election_status().is_closed(), <Error<T>>::CallNotAllowed);

			ensure_signed(origin)?;
			Self::do_payout_stakers(validator_stash, era)
		}

		/// Set history_depth value.
		///
		/// Origin must be root.
		#[weight = (500_000_000, DispatchClass::Operational)]
		fn set_history_depth(origin, #[compact] new_history_depth: EraIndex) {
			ensure_root(origin)?;
			if let Some(current_era) = Self::current_era() {
				HistoryDepth::mutate(|history_depth| {
					let last_kept = current_era.checked_sub(*history_depth).unwrap_or(0);
					let new_last_kept = current_era.checked_sub(new_history_depth).unwrap_or(0);
					for era_index in last_kept..new_last_kept {
						Self::clear_era_information(era_index);
					}
					*history_depth = new_history_depth
				})
			}
		}

		/// Remove all data structure concerning a staker/stash once its balance is zero.
		/// This is essentially equivalent to `withdraw_unbonded` except it can be called by anyone
		/// and the target `stash` must have no funds left.
		///
		/// This can be called from any origin.
		///
		/// - `stash`: The stash account to reap. Its balance must be zero.
		#[weight = 0]
		fn reap_stash(_origin, stash: T::AccountId) {
			ensure!(T::RingCurrency::total_balance(&stash).is_zero(), <Error<T>>::FundedTarget);
			ensure!(T::KtonCurrency::total_balance(&stash).is_zero(), <Error<T>>::FundedTarget);

			Self::kill_stash(&stash)?;
			T::RingCurrency::remove_lock(STAKING_ID, &stash);
			T::KtonCurrency::remove_lock(STAKING_ID, &stash);
		}

		/// Submit a phragmen result to the chain. If the solution:
		///
		/// 1. is valid.
		/// 2. has a better score than a potentially existing solution on chain.
		///
		/// then, it will be _put_ on chain.
		///
		/// A solution consists of two pieces of data:
		///
		/// 1. `winners`: a flat vector of all the winners of the round.
		/// 2. `assignments`: the compact version of an assignment vector that encodes the edge
		///	weights.
		///
		/// Both of which may be computed using [`phragmen`], or any other algorithm.
		///
		/// Additionally, the submitter must provide:
		///
		/// - The `score` that they claim their solution has.
		///
		/// Both validators and nominators will be represented by indices in the solution. The
		/// indices should respect the corresponding types ([`ValidatorIndex`] and
		/// [`NominatorIndex`]). Moreover, they should be valid when used to index into
		/// [`SnapshotValidators`] and [`SnapshotNominators`]. Any invalid index will cause the
		/// solution to be rejected. These two storage items are set during the election window and
		/// may be used to determine the indices.
		///
		/// A solution is valid if:
		///
		/// 0. It is submitted when [`EraElectionStatus`] is `Open`.
		/// 1. Its claimed score is equal to the score computed on-chain.
		/// 2. Presents the correct number of winners.
		/// 3. All indexes must be value according to the snapshot vectors. All edge values must
		///	also be correct and should not overflow the granularity of the ratio type (i.e. 256
		///	or billion).
		/// 4. For each edge, all targets are actually nominated by the voter.
		/// 5. Has correct self-votes.
		///
		/// A solutions score is consisted of 3 parameters:
		///
		/// 1. `min { support.total }` for each support of a winner. This value should be maximized.
		/// 2. `sum { support.total }` for each support of a winner. This value should be minimized.
		/// 3. `sum { support.total^2 }` for each support of a winner. This value should be
		///	minimized (to ensure less variance)
		///
		/// # <weight>
		/// E: number of edges. m: size of winner committee. n: number of nominators. d: edge degree
		/// (16 for now) v: number of on-chain validator candidates.
		///
		/// NOTE: given a solution which is reduced, we can enable a new check the ensure `|E| < n +
		/// m`. We don't do this _yet_, but our offchain worker code executes it nonetheless.
		///
		/// major steps (all done in `check_and_replace_solution`):
		///
		/// - Storage: O(1) read `ElectionStatus`.
		/// - Storage: O(1) read `PhragmenScore`.
		/// - Storage: O(1) read `ValidatorCount`.
		/// - Storage: O(1) length read from `SnapshotValidators`.
		///
		/// - Storage: O(v) reads of `AccountId` to fetch `snapshot_validators`.
		/// - Memory: O(m) iterations to map winner index to validator id.
		/// - Storage: O(n) reads `AccountId` to fetch `snapshot_nominators`.
		/// - Memory: O(n + m) reads to map index to `AccountId` for un-compact.
		///
		/// - Storage: O(e) accountid reads from `Nomination` to read correct nominations.
		/// - Storage: O(e) calls into `slashable_balance_of_vote_weight` to convert ratio to staked.
		///
		/// - Memory: build_support_map. O(e).
		/// - Memory: evaluate_support: O(E).
		///
		/// - Storage: O(e) writes to `QueuedElected`.
		/// - Storage: O(1) write to `QueuedScore`
		///
		/// The weight of this call is 1/10th of the blocks total weight.
		/// # </weight>
		#[weight = 100_000_000_000]
		pub fn submit_election_solution(
			origin,
			winners: Vec<ValidatorIndex>,
			compact_assignments: CompactAssignments,
			score: PhragmenScore,
			era: EraIndex,
		) {
			let _who = ensure_signed(origin)?;
			Self::check_and_replace_solution(
				winners,
				compact_assignments,
				ElectionCompute::Signed,
				score,
				era,
			)?
		}

		/// Unsigned version of `submit_election_solution`.
		///
		/// Note that this must pass the [`ValidateUnsigned`] check which only allows transactions
		/// from the local node to be included. In other words, only the block author can include a
		/// transaction in the block.
		#[weight = 100_000_000_000]
		pub fn submit_election_solution_unsigned(
			origin,
			winners: Vec<ValidatorIndex>,
			compact_assignments: CompactAssignments,
			score: PhragmenScore,
			era: EraIndex,
		) {
			ensure_none(origin)?;
			Self::check_and_replace_solution(
				winners,
				compact_assignments,
				ElectionCompute::Unsigned,
				score,
				era,
			)?
			// TODO: instead of returning an error, panic. This makes the entire produced block
			// invalid.
			// This ensures that block authors will not ever try and submit a solution which is not
			// an improvement, since they will lose their authoring points/rewards.
		}
	}
}

impl<T: Trait> Module<T> {
	// Update the ledger while bonding ring and compute the kton should return.
	fn bond_ring(
		stash: &T::AccountId,
		controller: &T::AccountId,
		value: RingBalance<T>,
		promise_month: u8,
		mut ledger: StakingLedgerT<T>,
	) -> (TsInMs, TsInMs) {
		let start_time = unix_time_now!();
		let mut expire_time = start_time;

		ledger.active_ring = ledger.active_ring.saturating_add(value);
		// if stash promise to a extra-lock
		// there will be extra reward, kton, which
		// can also be use to stake.
		if promise_month >= 3 {
			expire_time += promise_month as TsInMs * MONTH_IN_MILLISECONDS;
			ledger.active_deposit_ring += value;
			// for now, kton_return is free
			// mint kton
			let kton_return = inflation::compute_kton_return::<T>(value, promise_month as _);
			let kton_positive_imbalance = T::KtonCurrency::deposit_creating(&stash, kton_return);

			T::KtonReward::on_unbalanced(kton_positive_imbalance);
			ledger.deposit_items.push(TimeDepositItem {
				value,
				start_time,
				expire_time,
			});
		}

		Self::update_ledger(&controller, &mut ledger);

		(start_time, expire_time)
	}

	fn bond_ring_for_deposit_redeem(
		controller: &T::AccountId,
		value: RingBalance<T>,
		start_time: TsInMs,
		promise_month: u8,
		mut ledger: StakingLedgerT<T>,
	) -> (TsInMs, TsInMs) {
		let expire_time = start_time + promise_month as TsInMs * MONTH_IN_MILLISECONDS;

		ledger.active_ring = ledger.active_ring.saturating_add(value);
		ledger.active_deposit_ring = ledger.active_deposit_ring.saturating_add(value);
		ledger.deposit_items.push(TimeDepositItem {
			value,
			start_time,
			expire_time,
		});

		Self::update_ledger(&controller, &mut ledger);

		(start_time, expire_time)
	}

	// Update the ledger while bonding controller with kton.
	fn bond_kton(controller: &T::AccountId, value: KtonBalance<T>, mut ledger: StakingLedgerT<T>) {
		ledger.active_kton += value;
		Self::update_ledger(&controller, &mut ledger);
	}

	// TODO: doc
	pub fn clear_mature_deposits(mut ledger: StakingLedgerT<T>) -> StakingLedgerT<T> {
		let now = unix_time_now!();
		let StakingLedger {
			active_deposit_ring,
			deposit_items,
			..
		} = &mut ledger;

		deposit_items.retain(|item| {
			if item.expire_time > now {
				true
			} else {
				*active_deposit_ring = active_deposit_ring.saturating_sub(item.value);
				false
			}
		});

		ledger
	}

	// power is a mixture of ring and kton
	// For *RING* power = ring_ratio * POWER_COUNT / 2
	// For *KTON* power = kton_ratio * POWER_COUNT / 2
	pub fn currency_to_power<S: TryInto<Balance>>(active: S, pool: S) -> Power {
		(Perquintill::from_rational_approximation(
			active.saturated_into::<Balance>(),
			pool.saturated_into::<Balance>().max(1),
		) * (T::TotalPower::get() as Balance / 2)) as _
	}

	/// The total power that can be slashed from a stash account as of right now.
	pub fn power_of(stash: &T::AccountId) -> Power {
		Self::bonded(stash)
			.and_then(Self::ledger)
			.map(|l| {
				Self::currency_to_power::<_>(l.active_ring, Self::ring_pool())
					+ Self::currency_to_power::<_>(l.active_kton, Self::kton_pool())
			})
			.unwrap_or_default()
	}

	pub fn stake_of(stash: &T::AccountId) -> (RingBalance<T>, KtonBalance<T>) {
		Self::bonded(stash)
			.and_then(Self::ledger)
			.map(|l| (l.active_ring, l.active_kton))
			.unwrap_or_default()
	}

	/// Dump the list of validators and nominators into vectors and keep them on-chain.
	///
	/// This data is used to efficiently evaluate election results. returns `true` if the operation
	/// is successful.
	fn create_stakers_snapshot() -> bool {
		let validators = <Validators<T>>::iter().map(|(v, _)| v).collect::<Vec<_>>();
		let mut nominators = <Nominators<T>>::iter().map(|(n, _)| n).collect::<Vec<_>>();

		let num_validators = validators.len();
		let num_nominators = nominators.len();
		if num_validators > MAX_VALIDATORS
			|| num_nominators.saturating_add(num_validators) > MAX_NOMINATORS
		{
			log!(
				warn,
				"💸 Snapshot size too big [{} <> {}][{} <> {}].",
				num_validators,
				MAX_VALIDATORS,
				num_nominators,
				MAX_NOMINATORS,
			);
			false
		} else {
			// all validators nominate themselves;
			nominators.extend(validators.clone());

			<SnapshotValidators<T>>::put(validators);
			<SnapshotNominators<T>>::put(nominators);
			true
		}
	}

	/// Clears both snapshots of stakers.
	fn kill_stakers_snapshot() {
		<SnapshotValidators<T>>::kill();
		<SnapshotNominators<T>>::kill();
	}

	fn do_payout_stakers(validator_stash: T::AccountId, era: EraIndex) -> DispatchResult {
		// Validate input data
		let current_era = CurrentEra::get().ok_or(<Error<T>>::InvalidEraToReward)?;
		ensure!(era <= current_era, <Error<T>>::InvalidEraToReward);
		let history_depth = Self::history_depth();
		ensure!(
			era >= current_era.saturating_sub(history_depth),
			<Error<T>>::InvalidEraToReward
		);

		// Note: if era has no reward to be claimed, era may be future. better not to update
		// `ledger.claimed_rewards` in this case.
		let era_payout =
			<ErasValidatorReward<T>>::get(&era).ok_or_else(|| <Error<T>>::InvalidEraToReward)?;

		let controller = Self::bonded(&validator_stash).ok_or(<Error<T>>::NotStash)?;
		let mut ledger = <Ledger<T>>::get(&controller).ok_or_else(|| <Error<T>>::NotController)?;

		ledger
			.claimed_rewards
			.retain(|&x| x >= current_era.saturating_sub(history_depth));
		match ledger.claimed_rewards.binary_search(&era) {
			Ok(_) => Err(<Error<T>>::AlreadyClaimed)?,
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
			return Ok(());
		}

		// This is the fraction of the total reward that the validator and the
		// nominators will get.
		let validator_total_reward_part =
			Perbill::from_rational_approximation(validator_reward_points, total_reward_points);

		// This is how much validator + nominators are entitled to.
		let validator_total_payout = validator_total_reward_part * era_payout;

		let validator_prefs = Self::eras_validator_prefs(&era, &validator_stash);
		// Validator first gets a cut off the top.
		let validator_commission = validator_prefs.commission;
		let validator_commission_payout = validator_commission * validator_total_payout;

		let validator_leftover_payout = validator_total_payout - validator_commission_payout;
		// Now let's calculate how this is split to the validator.
		let validator_exposure_part =
			Perbill::from_rational_approximation(exposure.own_power, exposure.total_power);
		let validator_staking_payout = validator_exposure_part * validator_leftover_payout;

		// We can now make total validator payout:
		if let Some(imbalance) = Self::make_payout(
			&ledger.stash,
			validator_staking_payout + validator_commission_payout,
		) {
			Self::deposit_event(RawEvent::Reward(ledger.stash, imbalance.peek()));
		}

		// Lets now calculate how this is split to the nominators.
		// Sort nominators by highest to lowest exposure, but only keep `max_nominator_payouts` of them.
		for nominator in exposure.others.iter() {
			let nominator_exposure_part =
				Perbill::from_rational_approximation(nominator.power, exposure.total_power);

			let nominator_reward: RingBalance<T> =
				nominator_exposure_part * validator_leftover_payout;
			// We can now make nominator payout:
			if let Some(imbalance) = Self::make_payout(&nominator.who, nominator_reward) {
				Self::deposit_event(RawEvent::Reward(nominator.who.clone(), imbalance.peek()));
			}
		}

		Ok(())
	}

	/// Update the ledger for a controller. This will also update the stash lock. The lock will
	/// will lock the entire funds except paying for further transactions.
	fn update_ledger(controller: &T::AccountId, ledger: &mut StakingLedgerT<T>) {
		let StakingLedger {
			active_ring,
			active_kton,
			ring_staking_lock,
			kton_staking_lock,
			..
		} = ledger;

		if *active_ring != ring_staking_lock.staking_amount {
			ring_staking_lock.staking_amount = *active_ring;

			T::RingCurrency::set_lock(
				STAKING_ID,
				&ledger.stash,
				LockFor::Staking(ledger.ring_staking_lock.clone()),
				WithdrawReasons::all(),
			);
		}

		if *active_kton != kton_staking_lock.staking_amount {
			kton_staking_lock.staking_amount = *active_kton;

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
	fn chill_stash(stash: &T::AccountId) {
		<Validators<T>>::remove(stash);
		<Nominators<T>>::remove(stash);
	}

	/// Actually make a payment to a staker. This uses the currency's reward function
	/// to pay the right payee for the given staker account.
	fn make_payout(
		stash: &T::AccountId,
		amount: RingBalance<T>,
	) -> Option<RingPositiveImbalance<T>> {
		let dest = Self::payee(stash);
		match dest {
			RewardDestination::Controller => Self::bonded(stash).and_then(|controller| {
				T::RingCurrency::deposit_into_existing(&controller, amount).ok()
			}),
			RewardDestination::Stash => T::RingCurrency::deposit_into_existing(stash, amount).ok(),
			// TODO month
			RewardDestination::Staked { promise_month: _ } => Self::bonded(stash)
				.and_then(|c| Self::ledger(&c).map(|l| (c, l)))
				.and_then(|(c, mut l)| {
					l.active_ring += amount;

					let r = T::RingCurrency::deposit_into_existing(stash, amount).ok();
					Self::update_ledger(&c, &mut l);
					r
				}),
		}
	}

	/// Plan a new session potentially trigger a new era.
	fn new_session(session_index: SessionIndex) -> Option<Vec<T::AccountId>> {
		if let Some(current_era) = Self::current_era() {
			// Initial era has been set.

			let current_era_start_session_index = Self::eras_start_session_index(current_era)
				.unwrap_or_else(|| {
					frame_support::print("Error: start_session_index must be set for current_era");
					0
				});

			let era_length = session_index
				.checked_sub(current_era_start_session_index)
				.unwrap_or(0); // Must never happen.

			match ForceEra::get() {
				Forcing::ForceNew => ForceEra::kill(),
				Forcing::ForceAlways => (),
				Forcing::NotForcing if era_length >= T::SessionsPerEra::get() => (),
				_ => {
					// not forcing, not a new era either. If final, set the flag.
					if era_length + 1 >= T::SessionsPerEra::get() {
						IsCurrentSessionFinal::put(true);
					}
					return None;
				}
			}

			// new era.
			IsCurrentSessionFinal::put(false);
			Self::new_era(session_index)
		} else {
			// Set initial era
			Self::new_era(session_index)
		}
	}

	/// Basic and cheap checks that we perform in validate unsigned, and in the execution.
	pub fn pre_dispatch_checks(score: PhragmenScore, era: EraIndex) -> Result<(), Error<T>> {
		// discard solutions that are not in-time
		// check window open
		ensure!(
			Self::era_election_status().is_open(),
			<Error<T>>::PhragmenEarlySubmission,
		);

		// check current era.
		if let Some(current_era) = Self::current_era() {
			ensure!(current_era == era, <Error<T>>::PhragmenEarlySubmission,)
		}

		// assume the given score is valid. Is it better than what we have on-chain, if we have any?
		if let Some(queued_score) = Self::queued_score() {
			ensure!(
				is_score_better(queued_score, score),
				<Error<T>>::PhragmenWeakSubmission,
			)
		}

		Ok(())
	}

	/// Checks a given solution and if correct and improved, writes it on chain as the queued result
	/// of the next round. This may be called by both a signed and an unsigned transaction.
	pub fn check_and_replace_solution(
		winners: Vec<ValidatorIndex>,
		compact_assignments: CompactAssignments,
		compute: ElectionCompute,
		claimed_score: PhragmenScore,
		era: EraIndex,
	) -> Result<(), Error<T>> {
		// Do the basic checks. era, claimed score and window open.
		Self::pre_dispatch_checks(claimed_score, era)?;

		// Check that the number of presented winners is sane. Most often we have more candidates
		// that we need. Then it should be Self::validator_count(). Else it should be all the
		// candidates.
		let snapshot_length =
			<SnapshotValidators<T>>::decode_len().map_err(|_| <Error<T>>::SnapshotUnavailable)?;
		let desired_winners = Self::validator_count().min(snapshot_length as u32);
		ensure!(
			winners.len() as u32 == desired_winners,
			<Error<T>>::PhragmenBogusWinnerCount
		);

		// decode snapshot validators.
		let snapshot_validators =
			Self::snapshot_validators().ok_or(<Error<T>>::SnapshotUnavailable)?;

		// check if all winners were legit; this is rather cheap. Replace with accountId.
		let winners = winners
			.into_iter()
			.map(|widx| {
				// NOTE: at the moment, since staking is explicitly blocking any offence until election
				// is closed, we don't check here if the account id at `snapshot_validators[widx]` is
				// actually a validator. If this ever changes, this loop needs to also check this.
				snapshot_validators
					.get(widx as usize)
					.cloned()
					.ok_or(<Error<T>>::PhragmenBogusWinner)
			})
			.collect::<Result<Vec<T::AccountId>, Error<T>>>()?;

		// decode the rest of the snapshot.
		let snapshot_nominators =
			<Module<T>>::snapshot_nominators().ok_or(<Error<T>>::SnapshotUnavailable)?;

		// helpers
		let nominator_at = |i: NominatorIndex| -> Option<T::AccountId> {
			snapshot_nominators.get(i as usize).cloned()
		};
		let validator_at = |i: ValidatorIndex| -> Option<T::AccountId> {
			snapshot_validators.get(i as usize).cloned()
		};

		// un-compact.
		let assignments = compact_assignments
			.into_assignment(nominator_at, validator_at)
			.map_err(|e| {
				// log the error since it is not propagated into the runtime error.
				log!(warn, "💸 un-compacting solution failed due to {:?}", e);
				<Error<T>>::PhragmenBogusCompact
			})?;

		// check all nominators actually including the claimed vote. Also check correct self votes.
		// Note that we assume all validators and nominators in `assignments` are properly bonded,
		// because they are coming from the snapshot via a given index.
		for Assignment { who, distribution } in assignments.iter() {
			let is_validator = <Validators<T>>::contains_key(&who);
			let maybe_nomination = Self::nominators(&who);

			if !(maybe_nomination.is_some() ^ is_validator) {
				// all of the indices must map to either a validator or a nominator. If this is ever
				// not the case, then the locking system of staking is most likely faulty, or we
				// have bigger problems.
				log!(
					error,
					"💸 detected an error in the staking locking and snapshot."
				);
				// abort.
				return Err(<Error<T>>::PhragmenBogusNominator);
			}

			if !is_validator {
				// a normal vote
				let nomination = maybe_nomination.expect(
					"exactly one of `maybe_validator` and `maybe_nomination.is_some` is true. \
				is_validator is false; maybe_nomination is some; qed",
				);

				// NOTE: we don't really have to check here if the sum of all edges are the
				// nominator correct. Un-compacting assures this by definition.

				for (t, _) in distribution {
					// each target in the provided distribution must be actually nominated by the
					// nominator after the last non-zero slash.
					if nomination.targets.iter().find(|&tt| tt == t).is_none() {
						return Err(<Error<T>>::PhragmenBogusNomination);
					}

					if <Self as Store>::SlashingSpans::get(&t).map_or(false, |spans| {
						nomination.submitted_in < spans.last_nonzero_slash()
					}) {
						return Err(<Error<T>>::PhragmenSlashedNomination);
					}
				}
			} else {
				// a self vote
				ensure!(distribution.len() == 1, <Error<T>>::PhragmenBogusSelfVote);
				ensure!(distribution[0].0 == *who, <Error<T>>::PhragmenBogusSelfVote);
				// defensive only. A compact assignment of length one does NOT encode the weight and
				// it is always created to be 100%.
				ensure!(
					distribution[0].1 == OffchainAccuracy::one(),
					<Error<T>>::PhragmenBogusSelfVote,
				);
			}
		}

		// convert into staked assignments.
		let staked_assignments =
			sp_phragmen::assignment_ratio_to_staked(assignments, |s| Self::power_of(s) as _);

		// build the support map thereof in order to evaluate.
		// OPTIMIZATION: loop to create the staked assignments but it would bloat the code. Okay for
		// now as it does not add to the complexity order.
		let (supports, num_error) =
			build_support_map::<T::AccountId>(&winners, &staked_assignments);
		// This technically checks that all targets in all nominators were among the winners.
		ensure!(num_error == 0, <Error<T>>::PhragmenBogusEdge);

		// Check if the score is the same as the claimed one.
		let submitted_score = evaluate_support(&supports);
		ensure!(
			submitted_score == claimed_score,
			<Error<T>>::PhragmenBogusScore
		);

		// At last, alles Ok. Exposures and store the result.
		let exposures = Self::collect_exposure(supports);
		log!(
			info,
			"💸 A better solution (with compute {:?}) has been validated and stored on chain.",
			compute,
		);

		// write new results.
		<QueuedElected<T>>::put(ElectionResult {
			elected_stashes: winners,
			compute,
			exposures,
		});
		QueuedScore::put(submitted_score);

		Ok(())
	}

	/// Start a session potentially starting an era.
	fn start_session(start_session: SessionIndex) {
		let next_active_era = Self::active_era().map(|e| e.index + 1).unwrap_or(0);
		if let Some(next_active_era_start_session_index) =
			Self::eras_start_session_index(next_active_era)
		{
			if next_active_era_start_session_index == start_session {
				Self::start_era(start_session);
			} else if next_active_era_start_session_index < start_session {
				// This arm should never happen, but better handle it than to stall the
				// staking pallet.
				frame_support::print("Warning: A session appears to have been skipped.");
				Self::start_era(start_session);
			}
		}
	}

	/// End a session potentially ending an era.
	fn end_session(session_index: SessionIndex) {
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
	fn start_era(start_session: SessionIndex) {
		let active_era = ActiveEra::mutate(|active_era| {
			let new_index = active_era.as_ref().map(|info| info.index + 1).unwrap_or(0);
			*active_era = Some(ActiveEraInfo {
				index: new_index,
				// Set new active era start in next `on_finalize`. To guarantee usage of `Time`
				start: None,
			});
			new_index
		});

		let bonding_duration = T::BondingDurationInEra::get();

		BondedEras::mutate(|bonded| {
			bonded.push((active_era, start_session));

			if active_era > bonding_duration {
				let first_kept = active_era - bonding_duration;

				// prune out everything that's from before the first-kept index.
				let n_to_prune = bonded
					.iter()
					.take_while(|&&(era_idx, _)| era_idx < first_kept)
					.count();

				// kill slashing metadata.
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
	fn end_era(active_era: ActiveEraInfo, _session_index: SessionIndex) {
		// Note: active_era_start can be None if end era is called during genesis config.
		if let Some(active_era_start) = active_era.start {
			let now = unix_time_now!();
			let living_time = Self::living_time();
			let era_duration = now - active_era_start;

			let (validator_payout, max_payout) = inflation::compute_total_payout::<T>(
				era_duration,
				Self::living_time(),
				T::Cap::get().saturating_sub(T::RingCurrency::total_issuance()),
				PayoutFraction::get(),
			);
			let rest = max_payout.saturating_sub(validator_payout);

			Self::deposit_event(RawEvent::EraPayout(
				active_era.index,
				validator_payout,
				rest,
			));

			LivingTime::put(living_time + era_duration);
			// Set ending era reward.
			<ErasValidatorReward<T>>::insert(&active_era.index, validator_payout);
			T::RingRewardRemainder::on_unbalanced(T::RingCurrency::issue(rest));
		}
	}

	/// Plan a new era. Return the potential new staking set.
	fn new_era(start_session_index: SessionIndex) -> Option<Vec<T::AccountId>> {
		// Increment or set current era.
		let current_era = CurrentEra::mutate(|s| {
			*s = Some(s.map(|s| s + 1).unwrap_or(0));
			s.unwrap()
		});
		ErasStartSessionIndex::insert(&current_era, &start_session_index);

		// Clean old era information.
		if let Some(old_era) = current_era.checked_sub(Self::history_depth() + 1) {
			Self::clear_era_information(old_era);
		}

		// Set staking information for new era.
		let maybe_new_validators = Self::select_and_update_validators(current_era);

		maybe_new_validators
	}

	/// Select the new validator set at the end of the era.
	///
	/// Runs [`try_do_phragmen`] and updates the following storage items:
	/// - [`EraElectionStatus`]: with `None`.
	/// - [`ErasStakers`]: with the new staker set.
	/// - [`ErasStakersClipped`].
	/// - [`ErasValidatorPrefs`].
	/// - [`ErasTotalStake`]: with the new total stake.
	/// - [`SnapshotValidators`] and [`SnapshotNominators`] are both removed.
	///
	/// Internally, [`QueuedElected`], snapshots and [`QueuedScore`] are also consumed.
	///
	/// If the election has been successful, It passes the new set upwards.
	///
	/// This should only be called at the end of an era.
	fn select_and_update_validators(current_era: EraIndex) -> Option<Vec<T::AccountId>> {
		if let Some(ElectionResultT::<T> {
			elected_stashes,
			exposures,
			compute,
		}) = Self::try_do_phragmen()
		{
			// We have chosen the new validator set. Submission is no longer allowed.
			<EraElectionStatus<T>>::put(ElectionStatus::Closed);

			// Kill the snapshots.
			Self::kill_stakers_snapshot();

			// Populate Stakers and write slot stake.
			let mut total_stake = 0;
			exposures.into_iter().for_each(|(stash, exposure)| {
				// Total `Power` is `1_000_000_000`, never get overflow; qed
				total_stake += exposure.total_power;
				<ErasStakers<T>>::insert(current_era, &stash, &exposure);

				let mut exposure_clipped = exposure;
				let clipped_max_len = T::MaxNominatorRewardedPerValidator::get() as usize;
				if exposure_clipped.others.len() > clipped_max_len {
					exposure_clipped
						.others
						.sort_unstable_by(|a, b| a.power.cmp(&b.power).reverse());
					exposure_clipped.others.truncate(clipped_max_len);
				}
				<ErasStakersClipped<T>>::insert(&current_era, &stash, exposure_clipped);
			});
			// Insert current era staking information
			ErasTotalStake::insert(&current_era, total_stake);

			// collect the pref of all winners
			for stash in &elected_stashes {
				let pref = Self::validators(stash);
				<ErasValidatorPrefs<T>>::insert(&current_era, stash, pref);
			}

			// emit event
			Self::deposit_event(RawEvent::StakingElection(compute));

			log!(
				info,
				"💸 new validator set of size {:?} has been elected via {:?} for era {:?}",
				elected_stashes.len(),
				compute,
				current_era,
			);

			Some(elected_stashes)
		} else {
			None
		}
	}

	/// Select a new validator set from the assembled stakers and their role preferences. It tries
	/// first to peek into [`QueuedElected`]. Otherwise, it runs a new phragmen.
	///
	/// If [`QueuedElected`] and [`QueuedScore`] exists, they are both removed. No further storage
	/// is updated.
	fn try_do_phragmen() -> Option<ElectionResultT<T>> {
		// a phragmen result from either a stored submission or locally executed one.
		let next_result = <QueuedElected<T>>::take().or_else(|| {
			Self::do_phragmen_with_post_processing::<ChainAccuracy>(ElectionCompute::OnChain)
		});

		// either way, kill this. We remove it here to make sure it always has the exact same
		// lifetime as `QueuedElected`.
		QueuedScore::kill();

		next_result
	}

	/// Execute phragmen and return the new results. The edge weights are processed into support
	/// values.
	///
	/// This is basically a wrapper around [`do_phragmen`] which translates `PhragmenResult` into
	/// `ElectionResult`.
	///
	/// No storage item is updated.
	fn do_phragmen_with_post_processing<Accuracy: PerThing>(
		compute: ElectionCompute,
	) -> Option<ElectionResultT<T>>
	where
		Accuracy: sp_std::ops::Mul<ExtendedBalance, Output = ExtendedBalance>,
		ExtendedBalance: From<<Accuracy as PerThing>::Inner>,
	{
		if let Some(phragmen_result) = Self::do_phragmen::<Accuracy>() {
			let elected_stashes = phragmen_result
				.winners
				.iter()
				.map(|(s, _)| s.clone())
				.collect::<Vec<T::AccountId>>();
			let assignments = phragmen_result.assignments;

			let staked_assignments =
				sp_phragmen::assignment_ratio_to_staked(assignments, |s| Self::power_of(s) as _);

			let (supports, _) =
				build_support_map::<T::AccountId>(&elected_stashes, &staked_assignments);

			// collect exposures
			let exposures = Self::collect_exposure(supports);

			// In order to keep the property required by `on_session_ending` that we must return the
			// new validator set even if it's the same as the old, as long as any underlying
			// economic conditions have changed, we don't attempt to do any optimization where we
			// compare against the prior set.
			Some(ElectionResultT::<T> {
				elected_stashes,
				exposures,
				compute,
			})
		} else {
			// There were not enough candidates for even our minimal level of functionality. This is
			// bad. We should probably disable all functionality except for block production and let
			// the chain keep producing blocks until we can decide on a sufficiently substantial
			// set. TODO: #2494
			None
		}
	}

	// Execute phragmen and return the new results. No post-processing is applied and the raw edge
	/// weights are returned.
	///
	/// Self votes are added and nominations before the most recent slashing span are reaped.
	///
	/// No storage item is updated.
	fn do_phragmen<Accuracy: PerThing>() -> Option<PhragmenResult<T::AccountId, Accuracy>> {
		let mut all_nominators: Vec<(T::AccountId, VoteWeight, Vec<T::AccountId>)> = vec![];
		let mut all_validators = vec![];
		for (validator, _) in <Validators<T>>::iter() {
			// append self vote
			let self_vote = (
				validator.clone(),
				Self::power_of(&validator) as _,
				vec![validator.clone()],
			);
			all_nominators.push(self_vote);
			all_validators.push(validator);
		}

		let nominator_votes = <Nominators<T>>::iter().map(|(nominator, nominations)| {
			let Nominations {
				submitted_in,
				mut targets,
				suppressed: _,
			} = nominations;

			// Filter out nomination targets which were nominated before the most recent
			// slashing span.
			targets.retain(|stash| {
				<Self as Store>::SlashingSpans::get(&stash)
					.map_or(true, |spans| submitted_in >= spans.last_nonzero_slash())
			});

			(nominator, targets)
		});
		all_nominators.extend(nominator_votes.map(|(n, ns)| {
			let s = Self::power_of(&n) as _;
			(n, s, ns)
		}));

		elect::<_, Accuracy>(
			Self::validator_count() as usize,
			Self::minimum_validator_count().max(1) as usize,
			all_validators,
			all_nominators,
		)
	}

	/// Consume a set of [`Supports`] from [`sp_phragmen`] and collect them into a [`Exposure`]
	fn collect_exposure(supports: SupportMap<T::AccountId>) -> Vec<(T::AccountId, ExposureT<T>)> {
		supports
			.into_iter()
			.map(|(validator, support)| {
				// build `struct exposure` from `support`
				let mut own_ring_balance: RingBalance<T> = Zero::zero();
				let mut own_kton_balance: KtonBalance<T> = Zero::zero();
				let mut own_power = 0;
				let mut total_power = 0;
				let mut others = vec![];
				support.voters.into_iter().for_each(|(nominator, weight)| {
					let origin_weight = Self::power_of(&nominator) as ExtendedBalance;
					let (origin_ring_balance, origin_kton_balance) = Self::stake_of(&nominator);

					let ring_balance = if let Ok(ring_balance) = multiply_by_rational(
						origin_ring_balance.saturated_into(),
						weight,
						origin_weight,
					) {
						ring_balance.saturated_into()
					} else {
						debug::error!(
							target: "staking",
							"[staking] Origin RING: {:?}, Weight: {:?}, Origin Weight: {:?}",
							origin_ring_balance,
							weight,
							origin_weight
						);
						Zero::zero()
					};
					let kton_balance = if let Ok(kton_balance) = multiply_by_rational(
						origin_kton_balance.saturated_into(),
						weight,
						origin_weight,
					) {
						kton_balance.saturated_into()
					} else {
						debug::error!(
							target: "staking",
							"[staking] Origin KTON: {:?}, Weight: {:?}, Origin Weight: {:?}",
							origin_kton_balance,
							weight,
							origin_weight
						);
						Zero::zero()
					};
					let power = weight as Power;

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
			.collect::<Vec<(T::AccountId, ExposureT<T>)>>()
	}

	/// Remove all associated data of a stash account from the staking system.
	///
	/// Assumes storage is upgraded before calling.
	///
	/// This is called:
	/// - after a `withdraw_unbond()` call that frees all of a stash's bonded balance.
	/// - through `reap_stash()` if the balance has fallen to zero (through slashing).
	fn kill_stash(stash: &T::AccountId) -> DispatchResult {
		let controller = <Bonded<T>>::take(stash).ok_or(<Error<T>>::NotStash)?;
		<Ledger<T>>::remove(&controller);

		<Payee<T>>::remove(stash);
		<Validators<T>>::remove(stash);
		<Nominators<T>>::remove(stash);

		slashing::clear_stash_metadata::<T>(stash);

		<frame_system::Module<T>>::dec_ref(stash);

		Ok(())
	}

	/// Clear all era information for given era.
	fn clear_era_information(era_index: EraIndex) {
		<ErasStakers<T>>::remove_prefix(era_index);
		<ErasStakersClipped<T>>::remove_prefix(era_index);
		<ErasValidatorPrefs<T>>::remove_prefix(era_index);
		<ErasValidatorReward<T>>::remove(era_index);
		<ErasRewardPoints<T>>::remove(era_index);
		<ErasTotalStake>::remove(era_index);
		ErasStartSessionIndex::remove(era_index);
	}

	/// Apply previously-unapplied slashes on the beginning of a new era, after a delay.
	fn apply_unapplied_slashes(active_era: EraIndex) {
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
	pub fn reward_by_ids(validators_points: impl IntoIterator<Item = (T::AccountId, u32)>) {
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
	fn ensure_new_era() {
		match ForceEra::get() {
			Forcing::ForceAlways | Forcing::ForceNew => (),
			_ => ForceEra::put(Forcing::ForceNew),
		}
	}

	fn will_era_be_forced() -> bool {
		match ForceEra::get() {
			Forcing::ForceAlways | Forcing::ForceNew => true,
			Forcing::ForceNone | Forcing::NotForcing => false,
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	pub fn add_era_stakers(
		current_era: EraIndex,
		controller: T::AccountId,
		exposure: ExposureT<T>,
	) {
		<ErasStakers<T>>::insert(&current_era, &controller, &exposure);
	}

	#[cfg(feature = "runtime-benchmarks")]
	pub fn put_election_status(status: ElectionStatus<T::BlockNumber>) {
		<EraElectionStatus<T>>::put(status);
	}
}

impl<T: Trait> pallet_session::SessionManager<T::AccountId> for Module<T> {
	fn new_session(new_index: SessionIndex) -> Option<Vec<T::AccountId>> {
		Self::new_session(new_index)
	}
	fn end_session(end_index: SessionIndex) {
		Self::end_session(end_index)
	}
	fn start_session(start_index: SessionIndex) {
		Self::start_session(start_index)
	}
}

impl<T: Trait> pallet_session::historical::SessionManager<T::AccountId, ExposureT<T>>
	for Module<T>
{
	fn new_session(new_index: SessionIndex) -> Option<Vec<(T::AccountId, ExposureT<T>)>> {
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
	fn start_session(start_index: SessionIndex) {
		<Self as pallet_session::SessionManager<_>>::start_session(start_index)
	}
	fn end_session(end_index: SessionIndex) {
		<Self as pallet_session::SessionManager<_>>::end_session(end_index)
	}
}

/// Add reward points to block authors:
/// * 20 points to the block producer for producing a (non-uncle) block in the relay chain,
/// * 2 points to the block producer for each reference to a previously unreferenced uncle, and
/// * 1 point to the producer of each referenced uncle block.
impl<T> pallet_authorship::EventHandler<T::AccountId, T::BlockNumber> for Module<T>
where
	T: Trait + pallet_authorship::Trait + pallet_session::Trait,
{
	fn note_author(author: T::AccountId) {
		Self::reward_by_ids(vec![(author, 20)]);
	}
	fn note_uncle(author: T::AccountId, _age: T::BlockNumber) {
		Self::reward_by_ids(vec![
			(<pallet_authorship::Module<T>>::author(), 2),
			(author, 1),
		]);
	}
}

/// A `Convert` implementation that finds the stash of the given controller account,
/// if any.
pub struct StashOf<T>(PhantomData<T>);

impl<T: Trait> Convert<T::AccountId, Option<T::AccountId>> for StashOf<T> {
	fn convert(controller: T::AccountId) -> Option<T::AccountId> {
		<Module<T>>::ledger(&controller).map(|l| l.stash)
	}
}

/// A typed conversion from stash account ID to the active exposure of nominators
/// on that account.
///
/// Active exposure is the exposure of the validator set currently validating, i.e. in
/// `active_era`. It can differ from the latest planned exposure in `current_era`.
pub struct ExposureOf<T>(PhantomData<T>);

impl<T: Trait> Convert<T::AccountId, Option<ExposureT<T>>> for ExposureOf<T> {
	fn convert(validator: T::AccountId) -> Option<ExposureT<T>> {
		if let Some(active_era) = <Module<T>>::active_era() {
			Some(<Module<T>>::eras_stakers(active_era.index, &validator))
		} else {
			None
		}
	}
}

/// This is intended to be used with `FilterHistoricalOffences`.
impl<T: Trait> OnOffenceHandler<T::AccountId, pallet_session::historical::IdentificationTuple<T>>
	for Module<T>
where
	T: pallet_session::Trait<ValidatorId = AccountId<T>>,
	T: pallet_session::historical::Trait<
		FullIdentification = ExposureT<T>,
		FullIdentificationOf = ExposureOf<T>,
	>,
	T::SessionHandler: pallet_session::SessionHandler<AccountId<T>>,
	T::SessionManager: pallet_session::SessionManager<AccountId<T>>,
	T::ValidatorIdOf: Convert<AccountId<T>, Option<AccountId<T>>>,
{
	fn on_offence(
		offenders: &[OffenceDetails<
			T::AccountId,
			pallet_session::historical::IdentificationTuple<T>,
		>],
		slash_fraction: &[Perbill],
		slash_session: SessionIndex,
	) -> Result<(), ()> {
		if !Self::can_report() {
			return Err(());
		}

		let reward_proportion = SlashRewardFraction::get();

		let active_era = {
			let active_era = Self::active_era();
			if active_era.is_none() {
				// this offence need not be re-submitted.
				return Ok(());
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

		let window_start = active_era.saturating_sub(T::BondingDurationInEra::get());

		// fast path for active-era report - most likely.
		// `slash_session` cannot be in a future active era. It must be in `active_era` or before.
		let slash_era = if slash_session >= active_era_start_session_index {
			active_era
		} else {
			let eras = BondedEras::get();

			// reverse because it's more likely to find reports from recent eras.
			match eras
				.iter()
				.rev()
				.filter(|&&(_, ref sesh)| sesh <= &slash_session)
				.next()
			{
				None => return Ok(()), // before bonding period. defensive - should be filtered out.
				Some(&(ref slash_era, _)) => *slash_era,
			}
		};

		<Self as Store>::EarliestUnappliedSlash::mutate(|earliest| {
			if earliest.is_none() {
				*earliest = Some(active_era)
			}
		});

		let slash_defer_duration = T::SlashDeferDuration::get();

		for (details, slash_fraction) in offenders.iter().zip(slash_fraction) {
			let (stash, exposure) = &details.offender;

			// Skip if the validator is invulnerable.
			if Self::invulnerables().contains(stash) {
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
				unapplied.reporters = details.reporters.clone();
				if slash_defer_duration == 0 {
					// apply right away.
					slashing::apply_slash::<T>(unapplied);
				} else {
					// defer to end of some `slash_defer_duration` from now.
					<Self as Store>::UnappliedSlashes::mutate(active_era, move |for_later| {
						for_later.push(unapplied)
					});
				}
			}
		}

		Ok(())
	}

	fn can_report() -> bool {
		Self::era_election_status().is_closed()
	}
}

/// Filter historical offences out and only allow those from the bonding period.
pub struct FilterHistoricalOffences<T, R> {
	_inner: PhantomData<(T, R)>,
}
impl<T, Reporter, Offender, R, O> ReportOffence<Reporter, Offender, O>
	for FilterHistoricalOffences<Module<T>, R>
where
	T: Trait,
	R: ReportOffence<Reporter, Offender, O>,
	O: Offence<Offender>,
{
	fn report_offence(reporters: Vec<Reporter>, offence: O) -> Result<(), OffenceError> {
		// disallow any slashing from before the current bonding period.
		let offence_session = offence.session_index();
		let bonded_eras = BondedEras::get();

		if bonded_eras
			.first()
			.filter(|(_, start)| offence_session >= *start)
			.is_some()
		{
			R::report_offence(reporters, offence)
		} else {
			<Module<T>>::deposit_event(RawEvent::OldSlashingReportDiscarded(offence_session));
			Ok(())
		}
	}
}

impl<T: Trait> OnDepositRedeem<T::AccountId> for Module<T> {
	type Balance = RingBalance<T>;

	fn on_deposit_redeem(
		backing: &T::AccountId,
		start_time: TsInMs,
		months: u8,
		amount: Self::Balance,
		stash: &T::AccountId,
	) -> DispatchResult {
		let controller = Self::bonded(&stash).ok_or(<Error<T>>::NotStash)?;
		let ledger = Self::ledger(&controller).ok_or(<Error<T>>::NotController)?;

		// The timestamp unit is different between Ethereum and Darwinia, converting from seconds to milliseconds
		let start_time = start_time * 1000;
		let promise_month = months.min(36);

		//		let stash_balance = T::Ring::free_balance(&stash);

		// TODO: Lock but no kton reward because this is a deposit redeem
		//		let extra = extra.min(r);

		T::RingCurrency::transfer(&backing, &stash, amount, KeepAlive)?;

		let (start_time, expire_time) = Self::bond_ring_for_deposit_redeem(
			&controller,
			amount,
			start_time,
			promise_month,
			ledger,
		);

		<RingPool<T>>::mutate(|r| *r += amount);
		// TODO: Should we deposit an different event?
		<Module<T>>::deposit_event(RawEvent::BondRing(amount, start_time, expire_time));

		Ok(())
	}
}

impl<T: Trait> From<Error<T>> for InvalidTransaction {
	fn from(e: Error<T>) -> Self {
		InvalidTransaction::Custom(e.as_u8())
	}
}

#[allow(deprecated)]
impl<T: Trait> frame_support::unsigned::ValidateUnsigned for Module<T> {
	type Call = Call<T>;
	fn validate_unsigned(source: TransactionSource, call: &Self::Call) -> TransactionValidity {
		if let Call::submit_election_solution_unsigned(_, _, score, era) = call {
			use offchain_election::DEFAULT_LONGEVITY;

			// discard solution not coming from the local OCW.
			match source {
				TransactionSource::Local | TransactionSource::InBlock => { /* allowed */ }
				_ => {
					log!(
						debug,
						"rejecting unsigned transaction because it is not local/in-block."
					);
					return InvalidTransaction::Call.into();
				}
			}

			if let Err(e) = Self::pre_dispatch_checks(*score, *era) {
				log!(
					debug,
					"validate unsigned pre dispatch checks failed due to {:?}.",
					e
				);
				return InvalidTransaction::from(e).into();
			}

			log!(
				debug,
				"validateUnsigned succeeded for a solution at era {}.",
				era
			);

			ValidTransaction::with_tag_prefix("StakingOffchain")
				// The higher the score[0], the better a solution is.
				.priority(T::UnsignedPriority::get().saturating_add(score[0].saturated_into()))
				// Defensive only. A single solution can exist in the pool per era. Each validator
				// will run OCW at most once per era, hence there should never exist more than one
				// transaction anyhow.
				.and_provides(era)
				// Note: this can be more accurate in the future. We do something like
				// `era_end_block - current_block` but that is not needed now as we eagerly run
				// offchain workers now and the above should be same as `T::ElectionLookahead`
				// without the need to query more storage in the validation phase. If we randomize
				// offchain worker, then we might re-consider this.
				.longevity(
					TryInto::<u64>::try_into(T::ElectionLookahead::get())
						.unwrap_or(DEFAULT_LONGEVITY),
				)
				// We don't propagate this. This can never the validated at a remote node.
				.propagate(false)
				.build()
		} else {
			InvalidTransaction::Call.into()
		}
	}

	fn pre_dispatch(_: &Self::Call) -> Result<(), TransactionValidityError> {
		// IMPORTANT NOTE: By default, a sane `pre-dispatch` should always do the same checks as
		// `validate_unsigned` and overriding this should be done with care. this module has only
		// one unsigned entry point, in which we call into `<Module<T>>::pre_dispatch_checks()`
		// which is all the important checks that we do in `validate_unsigned`. Hence, we can safely
		// override this to save some time.
		Ok(())
	}
}

/// Check that list is sorted and has no duplicates.
fn is_sorted_and_unique(list: &Vec<u32>) -> bool {
	list.windows(2).all(|w| w[0] < w[1])
}
