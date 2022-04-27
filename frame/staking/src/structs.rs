// --- core ---
use core::{marker::PhantomData, mem};
use scale_info::TypeInfo;
// --- crates.io ---
use codec::{Decode, Encode, HasCompact};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
// --- paritytech ---
use frame_election_provider_support::*;
use frame_support::WeakBoundedVec;
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, Convert, Saturating, Zero},
	Perbill, RuntimeDebug,
};
use sp_std::{collections::btree_map::BTreeMap, prelude::*};
// --- darwinia-network ---
use crate::*;
use darwinia_support::balance::*;

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
#[derive(
	Clone, Default, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, RuntimeDebug, TypeInfo,
)]
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
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, RuntimeDebug, TypeInfo)]
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
#[derive(Encode, Decode, RuntimeDebug, TypeInfo)]
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
#[derive(Clone, Default, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
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
	pub active: RingBalance,
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
	pub fn consolidate_unbondings(&mut self, now: BlockNumber) {
		self.ring_staking_lock.refresh(now);
		self.kton_staking_lock.refresh(now);
	}

	/// Re-bond funds that were scheduled for unlocking.
	///
	/// Returns the amount actually rebonded.
	pub fn rebond(
		&mut self,
		plan_to_rebond_ring: RingBalance,
		plan_to_rebond_kton: KtonBalance,
	) -> (RingBalance, KtonBalance) {
		fn update<Balance, _M>(
			bonded: &mut Balance,
			lock: &mut StakingLock<Balance, _M>,
			plan_to_rebond: Balance,
		) -> Balance
		where
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

			rebonded
		}

		(
			update(&mut self.active, &mut self.ring_staking_lock, plan_to_rebond_ring),
			update(&mut self.active_kton, &mut self.kton_staking_lock, plan_to_rebond_kton),
		)
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
		let slash_out_of = |active: &mut RingBalance,
		                    active_deposit_ring: &mut RingBalance,
		                    deposit_item: &mut Vec<TimeDepositItem<RingBalance>>,
		                    active_kton: &mut KtonBalance,
		                    slash_ring: &mut RingBalance,
		                    slash_kton: &mut KtonBalance| {
			let slashable_active_ring = (*slash_ring).min(*active);
			let slashable_active_kton = (*slash_kton).min(*active_kton);

			if !slashable_active_ring.is_zero() {
				let slashable_normal_ring = *active - *active_deposit_ring;
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

				*active -= slashable_active_ring;
				*slash_ring -= slashable_active_ring;
			}

			if !slashable_active_kton.is_zero() {
				*active_kton -= slashable_active_kton;
				*slash_kton -= slashable_active_kton;
			}
		};

		let (mut apply_slash_ring, mut apply_slash_kton) = (slash_ring, slash_kton);
		let StakingLedger {
			active,
			active_deposit_ring,
			deposit_items,
			active_kton,
			ring_staking_lock,
			kton_staking_lock,
			..
		} = self;

		slash_out_of(
			active,
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
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct TimeDepositItem<RingBalance: HasCompact> {
	#[codec(compact)]
	pub value: RingBalance,
	#[codec(compact)]
	pub start_time: TsInMs,
	#[codec(compact)]
	pub expire_time: TsInMs,
}

/// A destination account for payment.
#[derive(Copy, Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
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
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
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
		ValidatorPrefs { commission: Perbill::zero(), blocked: false }
	}
}

/// A record of the nominations made by a specific account.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
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
#[derive(Debug, Default, PartialEq, Encode, Decode, TypeInfo)]
pub struct EraRewardPoints<AccountId: Ord> {
	/// Total number of points. Equals the sum of reward points for each validator.
	pub total: RewardPoint,
	/// The reward points earned by a given validator.
	pub individual: BTreeMap<AccountId, RewardPoint>,
}

/// Mode of era-forcing.
#[derive(Copy, Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
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
#[derive(Default, Encode, Decode, RuntimeDebug, TypeInfo)]
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
#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
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
#[derive(RuntimeDebug, TypeInfo)]
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
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
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

impl<T: Config> VoteWeightProvider<T::AccountId> for Pallet<T> {
	fn vote_weight(who: &T::AccountId) -> VoteWeight {
		Self::weight_of(who)
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn set_vote_weight_of(who: &T::AccountId, weight: VoteWeight) {
		// this will clearly results in an inconsistent state, but it should not matter for a
		// benchmark.
		use sp_std::convert::TryInto;
		let active: BalanceOf<T> = weight.try_into().map_err(|_| ()).unwrap();
		let mut ledger = Self::ledger(who).unwrap_or_default();
		ledger.active = active;
		<Ledger<T>>::insert(who, ledger);
		<Bonded<T>>::insert(who, who);

		// also, we play a trick to make sure that a issuance based-`CurrencyToVote` behaves well:
		// This will make sure that total issuance is zero, thus the currency to vote will be a 1-1
		// conversion.
		let imbalance = T::Currency::burn(T::Currency::total_issuance());
		// kinda ugly, but gets the job done. The fact that this works here is a HUGE exception.
		// Don't try this pattern in other places.
		sp_std::mem::forget(imbalance);
	}
}

/// A simple voter list implementation that does not require any additional pallets. Note, this
/// does not provided nominators in sorted ordered. If you desire nominators in a sorted order take
/// a look at [`pallet-bags-list].
pub struct UseNominatorsMap<T>(sp_std::marker::PhantomData<T>);
impl<T: Config> SortedListProvider<T::AccountId> for UseNominatorsMap<T> {
	type Error = ();

	/// Returns iterator over voter list, which can have `take` called on it.
	fn iter() -> Box<dyn Iterator<Item = T::AccountId>> {
		Box::new(<Nominators<T>>::iter().map(|(n, _)| n))
	}
	fn count() -> u32 {
		<CounterForNominators<T>>::get()
	}
	fn contains(id: &T::AccountId) -> bool {
		<Nominators<T>>::contains_key(id)
	}
	fn on_insert(_: T::AccountId, _weight: VoteWeight) -> Result<(), Self::Error> {
		// nothing to do on insert.
		Ok(())
	}
	fn on_update(_: &T::AccountId, _weight: VoteWeight) {
		// nothing to do on update.
	}
	fn on_remove(_: &T::AccountId) {
		// nothing to do on remove.
	}
	fn regenerate(
		_: impl IntoIterator<Item = T::AccountId>,
		_: Box<dyn Fn(&T::AccountId) -> VoteWeight>,
	) -> u32 {
		// nothing to do upon regenerate.
		0
	}
	fn sanity_check() -> Result<(), &'static str> {
		Ok(())
	}
	fn clear(maybe_count: Option<u32>) -> u32 {
		<Nominators<T>>::remove_all(maybe_count);
		if let Some(count) = maybe_count {
			<CounterForNominators<T>>::mutate(|noms| *noms - count);
			count
		} else {
			<CounterForNominators<T>>::take()
		}
	}
}
