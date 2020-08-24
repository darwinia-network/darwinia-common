//! A slashing implementation for NPoS systems.
//!
//! For the purposes of the economic model, it is easiest to think of each validator as a nominator
//! which nominates only its own identity.
//!
//! The act of nomination signals intent to unify economic identity with the validator - to take
//! part in the rewards of a job well done, and to take part in the punishment of a job done badly.
//!
//! There are 3 main difficulties to account for with slashing in NPoS:
//!   - A nominator can nominate multiple validators and be slashed via any of them.
//!   - Until slashed, stake is reused from era to era. Nominating with N coins for E eras in a row
//!     does not mean you have N*E coins to be slashed - you've only ever had N.
//!   - Slashable offences can be found after the fact and out of order.
//!
//! The algorithm implemented in this module tries to balance these 3 difficulties.
//!
//! First, we only slash participants for the _maximum_ slash they receive in some time period,
//! rather than the sum. This ensures a protection from overslashing.
//!
//! Second, we do not want the time period (or "span") that the maximum is computed
//! over to last indefinitely. That would allow participants to begin acting with
//! impunity after some point, fearing no further repercussions. For that reason, we
//! automatically "chill" validators and withdraw a nominator's nomination after a slashing event,
//! requiring them to re-enlist voluntarily (acknowledging the slash) and begin a new
//! slashing span.
//!
//! Typically, you will have a single slashing event per slashing span. Only in the case
//! where a validator releases many misbehaviors at once, or goes "back in time" to misbehave in
//! eras that have already passed, would you encounter situations where a slashing span
//! has multiple misbehaviors. However, accounting for such cases is necessary
//! to deter a class of "rage-quit" attacks.
//!
//! Based on research at https://research.web3.foundation/en/latest/polkadot/slashing/npos/

// --- crates ---
use codec::{Decode, Encode};
// --- substrate ---
use frame_support::{
	debug::error,
	ensure,
	traits::{Currency, Imbalance, OnUnbalanced},
	StorageDoubleMap, StorageMap,
};
use sp_runtime::{
	traits::{Saturating, Zero},
	DispatchResult, Perbill, RuntimeDebug,
};
use sp_std::{
	ops::{Add, AddAssign, Sub},
	prelude::*,
};
// --- darwinia ---
use crate::*;

/// The proportion of the slashing reward to be paid out on the first slashing detection.
/// This is f_1 in the paper.
const REWARD_F1: Perbill = Perbill::from_percent(50);

/// The index of a slashing span - unique to each stash.
pub type SpanIndex = u32;

// TODO doc
pub(crate) type RKT<T> = RK<RingBalance<T>, KtonBalance<T>>;

// TODO doc
#[derive(Clone, Copy, Default, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, RuntimeDebug)]
pub struct RK<R, K> {
	pub(crate) r: R,
	pub(crate) k: K,
}

impl<R, K> Zero for RK<R, K>
where
	R: Zero,
	K: Zero,
{
	fn zero() -> Self {
		Self {
			r: Zero::zero(),
			k: Zero::zero(),
		}
	}

	fn set_zero(&mut self) {
		self.r = Zero::zero();
		self.k = Zero::zero();
	}

	fn is_zero(&self) -> bool {
		self.r.is_zero() && self.k.is_zero()
	}
}

impl<R, K> Add for RK<R, K>
where
	R: Add<Output = R>,
	K: Add<Output = K>,
{
	type Output = Self;

	fn add(self, rhs: Self) -> Self::Output {
		Self {
			r: self.r + rhs.r,
			k: self.k + rhs.k,
		}
	}
}

impl<R, K> AddAssign for RK<R, K>
where
	R: AddAssign,
	K: AddAssign,
{
	fn add_assign(&mut self, rhs: Self) {
		self.r += rhs.r;
		self.k += rhs.k;
	}
}

impl<R, K> Sub for RK<R, K>
where
	R: Sub<Output = R>,
	K: Sub<Output = K>,
{
	type Output = Self;

	fn sub(self, rhs: Self) -> Self::Output {
		Self {
			r: self.r - rhs.r,
			k: self.k - rhs.k,
		}
	}
}

impl<R, K> Saturating for RK<R, K>
where
	R: Copy + Saturating,
	K: Copy + Saturating,
{
	fn saturating_add(self, o: Self) -> Self {
		Self {
			r: self.r.saturating_add(o.r),
			k: self.k.saturating_add(o.k),
		}
	}

	fn saturating_sub(self, o: Self) -> Self {
		Self {
			r: self.r.saturating_sub(o.r),
			k: self.k.saturating_sub(o.k),
		}
	}

	fn saturating_mul(self, o: Self) -> Self {
		Self {
			r: self.r.saturating_mul(o.r),
			k: self.k.saturating_mul(o.k),
		}
	}

	fn saturating_pow(self, exp: usize) -> Self {
		Self {
			r: self.r.saturating_pow(exp),
			k: self.k.saturating_pow(exp),
		}
	}
}

// A range of start..end eras for a slashing span.
#[derive(Encode, Decode)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub(crate) struct SlashingSpan {
	pub(crate) index: SpanIndex,
	pub(crate) start: EraIndex,
	pub(crate) length: Option<EraIndex>, // the ongoing slashing span has indeterminate length.
}

impl SlashingSpan {
	fn contains_era(&self, era: EraIndex) -> bool {
		self.start <= era && self.length.map_or(true, |l| self.start + l > era)
	}
}

/// An encoding of all of a nominator's slashing spans.
#[derive(Encode, Decode, RuntimeDebug)]
pub struct SlashingSpans {
	// the index of the current slashing span of the nominator. different for
	// every stash, resets when the account hits free balance 0.
	span_index: SpanIndex,
	// the start era of the most recent (ongoing) slashing span.
	last_start: EraIndex,
	// the last era at which a non-zero slash occurred.
	last_nonzero_slash: EraIndex,
	// all prior slashing spans' start indices, in reverse order (most recent first)
	// encoded as offsets relative to the slashing span after it.
	prior: Vec<EraIndex>,
}

impl SlashingSpans {
	// creates a new record of slashing spans for a stash, starting at the beginning
	// of the bonding period, relative to now.
	pub(crate) fn new(window_start: EraIndex) -> Self {
		SlashingSpans {
			span_index: 0,
			last_start: window_start,
			// initialize to zero, as this structure is lazily created until
			// the first slash is applied. setting equal to `window_start` would
			// put a time limit on nominations.
			last_nonzero_slash: 0,
			prior: vec![],
		}
	}

	// update the slashing spans to reflect the start of a new span at the era after `now`
	// returns `true` if a new span was started, `false` otherwise. `false` indicates
	// that internal state is unchanged.
	pub(crate) fn end_span(&mut self, now: EraIndex) -> bool {
		let next_start = now + 1;
		if next_start <= self.last_start {
			return false;
		}

		let last_length = next_start - self.last_start;
		self.prior.insert(0, last_length);
		self.last_start = next_start;
		self.span_index += 1;
		true
	}

	// an iterator over all slashing spans in _reverse_ order - most recent first.
	pub(crate) fn iter(&'_ self) -> impl Iterator<Item = SlashingSpan> + '_ {
		let mut last_start = self.last_start;
		let mut index = self.span_index;
		let last = SlashingSpan {
			index,
			start: last_start,
			length: None,
		};
		let prior = self.prior.iter().cloned().map(move |length| {
			let start = last_start - length;
			last_start = start;
			index -= 1;

			SlashingSpan {
				index,
				start,
				length: Some(length),
			}
		});

		sp_std::iter::once(last).chain(prior)
	}

	/// Yields the era index where the most recent non-zero slash occurred.
	pub fn last_nonzero_slash(&self) -> EraIndex {
		self.last_nonzero_slash
	}

	// prune the slashing spans against a window, whose start era index is given.
	//
	// If this returns `Some`, then it includes a range start..end of all the span
	// indices which were pruned.
	fn prune(&mut self, window_start: EraIndex) -> Option<(SpanIndex, SpanIndex)> {
		let old_idx = self
			.iter()
			.skip(1) // skip ongoing span.
			.position(|span| {
				span.length
					.map_or(false, |len| span.start + len <= window_start)
			});

		let earliest_span_index = self.span_index - self.prior.len() as SpanIndex;
		let pruned = match old_idx {
			Some(o) => {
				self.prior.truncate(o);
				let new_earliest = self.span_index - self.prior.len() as SpanIndex;
				Some((earliest_span_index, new_earliest))
			}
			None => None,
		};

		// readjust the ongoing span, if it started before the beginning of the window.
		self.last_start = sp_std::cmp::max(self.last_start, window_start);
		pruned
	}
}

/// A slashing-span record for a particular stash.
#[derive(Encode, Decode, Default)]
pub(crate) struct SpanRecord<RingBalance, KtonBalance> {
	slashed: RK<RingBalance, KtonBalance>,
	paid_out: RK<RingBalance, KtonBalance>,
}

impl<RingBalance, KtonBalance> SpanRecord<RingBalance, KtonBalance> {
	/// The value of stash balance slashed in this span.
	#[cfg(test)]
	pub(crate) fn amount_slashed(&self) -> &RK<RingBalance, KtonBalance> {
		&self.slashed
	}
}

/// Parameters for performing a slash.
#[derive(Clone)]
pub(crate) struct SlashParams<'a, T: 'a + Trait> {
	/// The stash account being slashed.
	pub(crate) stash: &'a T::AccountId,
	/// The proportion of the slash.
	pub(crate) slash: Perbill,
	/// The exposure of the stash and all nominators.
	pub(crate) exposure: &'a Exposure<T::AccountId, RingBalance<T>, KtonBalance<T>>,
	/// The era where the offence occurred.
	pub(crate) slash_era: EraIndex,
	/// The first era in the current bonding period.
	pub(crate) window_start: EraIndex,
	/// The current era.
	pub(crate) now: EraIndex,
	/// The maximum percentage of a slash that ever gets paid out.
	/// This is f_inf in the paper.
	pub(crate) reward_proportion: Perbill,
}

/// Computes a slash of a validator and nominators. It returns an unapplied
/// record to be applied at some later point. Slashing metadata is updated in storage,
/// since unapplied records are only rarely intended to be dropped.
///
/// The pending slash record returned does not have initialized reporters. Those have
/// to be set at a higher level, if any.
pub(crate) fn compute_slash<T: Trait>(
	params: SlashParams<T>,
) -> Option<UnappliedSlash<T::AccountId, RingBalance<T>, KtonBalance<T>>> {
	let SlashParams {
		stash,
		slash,
		exposure,
		slash_era,
		window_start,
		now,
		reward_proportion,
	} = params.clone();

	let mut reward_payout = Zero::zero();
	let mut val_slashed = Zero::zero();

	// is the slash amount here a maximum for the era?
	let own_slash = RK {
		r: slash * exposure.own_ring_balance,
		k: slash * exposure.own_kton_balance,
	};
	if (slash * exposure.total_power).is_zero() {
		// kick out the validator even if they won't be slashed,
		// as long as the misbehavior is from their most recent slashing span.
		kick_out_if_recent::<T>(params);
		return None;
	}

	let (prior_slash_p, _era_slash) =
		<Module<T> as Store>::ValidatorSlashInEra::get(&slash_era, stash)
			.unwrap_or((Perbill::zero(), Zero::zero()));

	// compare slash proportions rather than slash values to avoid issues due to rounding
	// error.
	if slash.deconstruct() > prior_slash_p.deconstruct() {
		<Module<T> as Store>::ValidatorSlashInEra::insert(&slash_era, stash, &(slash, own_slash));
	} else {
		// we slash based on the max in era - this new event is not the max,
		// so neither the validator or any nominators will need an update.
		//
		// this does lead to a divergence of our system from the paper, which
		// pays out some reward even if the latest report is not max-in-era.
		// we opt to avoid the nominator lookups and edits and leave more rewards
		// for more drastic misbehavior.
		return None;
	}

	// apply slash to validator.
	{
		let mut spans = fetch_spans::<T>(
			stash,
			window_start,
			&mut reward_payout,
			&mut val_slashed,
			reward_proportion,
		);

		let target_span = spans.compare_and_update_span_slash(slash_era, own_slash);

		if target_span == Some(spans.span_index()) {
			// misbehavior occurred within the current slashing span - take appropriate
			// actions.

			// chill the validator - it misbehaved in the current span and should
			// not continue in the next election. also end the slashing span.
			spans.end_span(now);
			<Module<T>>::chill_stash(stash);

			// make sure to disable validator till the end of this session
			if T::SessionInterface::disable_validator(stash).unwrap_or(false) {
				// force a new era, to select a new validator set
				<Module<T>>::ensure_new_era()
			}
		}
	}

	let mut nominators_slashed = vec![];
	reward_payout += slash_nominators::<T>(params, prior_slash_p, &mut nominators_slashed);

	Some(UnappliedSlash {
		validator: stash.clone(),
		own: val_slashed,
		others: nominators_slashed,
		reporters: vec![],
		payout: reward_payout,
	})
}

// doesn't apply any slash, but kicks out the validator if the misbehavior is from
// the most recent slashing span.
fn kick_out_if_recent<T: Trait>(params: SlashParams<T>) {
	// these are not updated by era-span or end-span.
	let mut reward_payout = RK::zero();
	let mut val_slashed = RK::zero();
	let mut spans = fetch_spans::<T>(
		params.stash,
		params.window_start,
		&mut reward_payout,
		&mut val_slashed,
		params.reward_proportion,
	);

	if spans.era_span(params.slash_era).map(|s| s.index) == Some(spans.span_index()) {
		spans.end_span(params.now);
		<Module<T>>::chill_stash(params.stash);

		// make sure to disable validator till the end of this session
		if T::SessionInterface::disable_validator(params.stash).unwrap_or(false) {
			// force a new era, to select a new validator set
			<Module<T>>::ensure_new_era()
		}
	}
}

/// Slash nominators. Accepts general parameters and the prior slash percentage of the validator.
///
/// Returns the amount of reward to pay out.
fn slash_nominators<T: Trait>(
	params: SlashParams<T>,
	prior_slash_p: Perbill,
	nominators_slashed: &mut Vec<(T::AccountId, RKT<T>)>,
) -> RKT<T> {
	let SlashParams {
		stash: _,
		slash,
		exposure,
		slash_era,
		window_start,
		now,
		reward_proportion,
	} = params;

	let mut reward_payout = Zero::zero();

	nominators_slashed.reserve(exposure.others.len());
	for nominator in &exposure.others {
		let stash = &nominator.who;
		let mut nom_slashed = Zero::zero();

		// the era slash of a nominator always grows, if the validator
		// had a new max slash for the era.
		let era_slash = {
			let own_slash_prior = RK {
				r: prior_slash_p * nominator.ring_balance,
				k: prior_slash_p * nominator.kton_balance,
			};
			let own_slash_by_validator = RK {
				r: slash * nominator.ring_balance,
				k: slash * nominator.kton_balance,
			};
			let own_slash_difference = own_slash_by_validator.saturating_sub(own_slash_prior);

			let mut era_slash = <Module<T> as Store>::NominatorSlashInEra::get(&slash_era, stash)
				.unwrap_or_else(|| Zero::zero());

			era_slash += own_slash_difference;

			<Module<T> as Store>::NominatorSlashInEra::insert(&slash_era, stash, &era_slash);

			era_slash
		};

		// compare the era slash against other eras in the same span.
		{
			let mut spans = fetch_spans::<T>(
				stash,
				window_start,
				&mut reward_payout,
				&mut nom_slashed,
				reward_proportion,
			);

			let target_span = spans.compare_and_update_span_slash(slash_era, era_slash);

			if target_span == Some(spans.span_index()) {
				// End the span, but don't chill the nominator. its nomination
				// on this validator will be ignored in the future.
				spans.end_span(now);
			}
		}

		nominators_slashed.push((stash.clone(), nom_slashed));
	}

	reward_payout
}

// helper struct for managing a set of spans we are currently inspecting.
// writes alterations to disk on drop, but only if a slash has been carried out.
//
// NOTE: alterations to slashing metadata should not be done after this is dropped.
// dropping this struct applies any necessary slashes, which can lead to free balance
// being 0, and the account being garbage-collected -- a dead account should get no new
// metadata.
struct InspectingSpans<'a, T: Trait + 'a> {
	dirty: bool,
	window_start: EraIndex,
	stash: &'a T::AccountId,
	spans: SlashingSpans,
	paid_out: &'a mut RKT<T>,
	slash_of: &'a mut RKT<T>,
	reward_proportion: Perbill,
}

// fetches the slashing spans record for a stash account, initializing it if necessary.
fn fetch_spans<'a, T: Trait + 'a>(
	stash: &'a T::AccountId,
	window_start: EraIndex,
	paid_out: &'a mut RKT<T>,
	slash_of: &'a mut RKT<T>,
	reward_proportion: Perbill,
) -> InspectingSpans<'a, T> {
	let spans = <Module<T> as Store>::SlashingSpans::get(stash).unwrap_or_else(|| {
		let spans = SlashingSpans::new(window_start);
		<Module<T> as Store>::SlashingSpans::insert(stash, &spans);
		spans
	});

	InspectingSpans {
		dirty: false,
		window_start,
		stash,
		spans,
		slash_of,
		paid_out,
		reward_proportion,
	}
}

impl<'a, T: 'a + Trait> InspectingSpans<'a, T> {
	fn span_index(&self) -> SpanIndex {
		self.spans.span_index
	}

	fn end_span(&mut self, now: EraIndex) {
		self.dirty = self.spans.end_span(now) || self.dirty;
	}

	// add some value to the slash of the staker.
	// invariant: the staker is being slashed for non-zero value here
	// although `amount` may be zero, as it is only a difference.
	fn add_slash(&mut self, amount: RKT<T>, slash_era: EraIndex) {
		*self.slash_of += amount;
		self.spans.last_nonzero_slash = sp_std::cmp::max(self.spans.last_nonzero_slash, slash_era);
	}

	// find the span index of the given era, if covered.
	fn era_span(&self, era: EraIndex) -> Option<SlashingSpan> {
		self.spans.iter().find(|span| span.contains_era(era))
	}

	// compares the slash in an era to the overall current span slash.
	// if it's higher, applies the difference of the slashes and then updates the span on disk.
	//
	// returns the span index of the era where the slash occurred, if any.
	fn compare_and_update_span_slash(
		&mut self,
		slash_era: EraIndex,
		slash: RKT<T>,
	) -> Option<SpanIndex> {
		let target_span = self.era_span(slash_era)?;
		let span_slash_key = (self.stash.clone(), target_span.index);
		let mut span_record = <Module<T> as Store>::SpanSlash::get(&span_slash_key);
		let mut changed = false;

		let reward = if span_record.slashed < slash {
			// new maximum span slash. apply the difference.
			let difference = slash - span_record.slashed;
			span_record.slashed = slash;

			// compute reward.
			let slash = RK {
				r: self.reward_proportion * slash.r,
				k: self.reward_proportion * slash.k,
			};
			let slash = slash.saturating_sub(span_record.paid_out);
			let reward = RK {
				r: REWARD_F1 * slash.r,
				k: REWARD_F1 * slash.k,
			};

			self.add_slash(difference, slash_era);
			changed = true;

			reward
		} else if span_record.slashed == slash {
			// compute reward. no slash difference to apply.
			let slash = RK {
				r: self.reward_proportion * slash.r,
				k: self.reward_proportion * slash.k,
			};
			let slash = slash.saturating_sub(span_record.paid_out);
			RK {
				r: REWARD_F1 * slash.r,
				k: REWARD_F1 * slash.k,
			}
		} else {
			Zero::zero()
		};

		if !reward.is_zero() {
			changed = true;
			span_record.paid_out += reward;
			*self.paid_out += reward;
		}

		if changed {
			self.dirty = true;
			<Module<T> as Store>::SpanSlash::insert(&span_slash_key, &span_record);
		}

		Some(target_span.index)
	}
}

impl<'a, T: 'a + Trait> Drop for InspectingSpans<'a, T> {
	fn drop(&mut self) {
		// only update on disk if we slashed this account.
		if !self.dirty {
			return;
		}

		if let Some((start, end)) = self.spans.prune(self.window_start) {
			for span_index in start..end {
				<Module<T> as Store>::SpanSlash::remove(&(self.stash.clone(), span_index));
			}
		}

		<Module<T> as Store>::SlashingSpans::insert(self.stash, &self.spans);
	}
}

/// Clear slashing metadata for an obsolete era.
pub(crate) fn clear_era_metadata<T: Trait>(obsolete_era: EraIndex) {
	<Module<T> as Store>::ValidatorSlashInEra::remove_prefix(&obsolete_era);
	<Module<T> as Store>::NominatorSlashInEra::remove_prefix(&obsolete_era);
}

/// Clear slashing metadata for a dead account.
pub(crate) fn clear_stash_metadata<T: Trait>(
	stash: &T::AccountId,
	num_slashing_spans: u32,
) -> DispatchResult {
	let spans = match <Module<T> as Store>::SlashingSpans::get(stash) {
		None => return Ok(()),
		Some(s) => s,
	};

	ensure!(
		num_slashing_spans as usize >= spans.iter().count(),
		<Error<T>>::IncorrectSlashingSpans
	);

	<Module<T> as Store>::SlashingSpans::remove(stash);

	// kill slashing-span metadata for account.
	//
	// this can only happen while the account is staked _if_ they are completely slashed.
	// in that case, they may re-bond, but it would count again as span 0. Further ancient
	// slashes would slash into this new bond, since metadata has now been cleared.
	for span in spans.iter() {
		<Module<T> as Store>::SpanSlash::remove(&(stash.clone(), span.index));
	}

	Ok(())
}

// apply the slash to a stash account, deducting any missing funds from the reward
// payout, saturating at 0. this is mildly unfair but also an edge-case that
// can only occur when overlapping locked funds have been slashed.
pub fn do_slash<T: Trait>(
	stash: &T::AccountId,
	value: RKT<T>,
	reward_payout: &mut RKT<T>,
	slashed_ring: &mut RingNegativeImbalance<T>,
	slashed_kton: &mut KtonNegativeImbalance<T>,
) {
	let controller = match <Module<T>>::bonded(stash) {
		None => return, // defensive: should always exist.
		Some(c) => c,
	};

	let mut ledger = match <Module<T>>::ledger(&controller) {
		Some(ledger) => ledger,
		None => return, // nothing to do.
	};

	let (pre_active_ring, pre_active_kton) = (ledger.active_ring, ledger.active_kton);

	let (slash_ring, slash_kton) = ledger.slash(
		value.r,
		value.k,
		<frame_system::Module<T>>::block_number(),
		T::UnixTime::now().as_millis().saturated_into::<TsInMs>(),
	);
	let mut slashed = false;

	if !slash_ring.is_zero() {
		slashed = true;

		let (imbalance, missing) = T::RingCurrency::slash(stash, slash_ring);
		slashed_ring.subsume(imbalance);

		if !missing.is_zero() {
			// deduct overslash from the reward payout
			reward_payout.r = reward_payout.r.saturating_sub(missing);
		}

		<RingPool<T>>::mutate(|p| {
			let slashed_active_ring = pre_active_ring.saturating_sub(ledger.active_ring);

			if *p > slashed_active_ring {
				*p -= slashed_active_ring;
			} else {
				error!("Slash on {} Underflow the RING Pool", stash);

				*p = Zero::zero();
			}
		});
	}
	if !slash_kton.is_zero() {
		slashed = true;

		let (imbalance, missing) = T::KtonCurrency::slash(stash, slash_kton);
		slashed_kton.subsume(imbalance);

		if !missing.is_zero() {
			// deduct overslash from the reward payout
			reward_payout.k = reward_payout.k.saturating_sub(missing);
		}

		<KtonPool<T>>::mutate(|p| {
			let slashed_active_kton = pre_active_kton.saturating_sub(ledger.active_kton);

			if *p > slashed_active_kton {
				*p -= slashed_active_kton;
			} else {
				error!("Slash on {} Underflow the RING Pool", stash);

				*p = Zero::zero();
			}
		});
	}

	if slashed {
		<Module<T>>::update_ledger(&controller, &mut ledger);
		<Module<T>>::deposit_event(RawEvent::Slash(stash.clone(), value.r, value.k));
	}
}

/// Apply a previously-unapplied slash.
pub(crate) fn apply_slash<T: Trait>(
	unapplied_slash: UnappliedSlash<T::AccountId, RingBalance<T>, KtonBalance<T>>,
) {
	let mut slashed_ring = <RingNegativeImbalance<T>>::zero();
	let mut slashed_kton = <KtonNegativeImbalance<T>>::zero();
	let mut reward_payout = unapplied_slash.payout;

	do_slash::<T>(
		&unapplied_slash.validator,
		unapplied_slash.own,
		&mut reward_payout,
		&mut slashed_ring,
		&mut slashed_kton,
	);

	for &(ref nominator, nominator_slash) in &unapplied_slash.others {
		do_slash::<T>(
			&nominator,
			nominator_slash,
			&mut reward_payout,
			&mut slashed_ring,
			&mut slashed_kton,
		);
	}

	pay_reporters::<T>(
		reward_payout,
		slashed_ring,
		slashed_kton,
		&unapplied_slash.reporters,
	);
}

/// Apply a reward payout to some reporters, paying the rewards out of the slashed imbalance.
fn pay_reporters<T: Trait>(
	reward_payout: RKT<T>,
	slashed_ring: RingNegativeImbalance<T>,
	slashed_kton: KtonNegativeImbalance<T>,
	reporters: &[T::AccountId],
) {
	if reporters.is_empty() {
		return;
	}

	let reward_ring_is_zero = reward_payout.r.is_zero();
	let reward_kton_is_zero = reward_payout.k.is_zero();

	if reward_ring_is_zero && reward_kton_is_zero {
		return;
	}

	if !reward_ring_is_zero && reward_kton_is_zero {
		// take rewards out of the slashed imbalance.
		let ring_reward_payout = reward_payout.r.min(slashed_ring.peek());
		let (mut ring_reward_payout, mut ring_slashed) = slashed_ring.split(ring_reward_payout);

		let ring_per_reporter = ring_reward_payout.peek() / (reporters.len() as u32).into();
		for reporter in reporters {
			let (ring_reporter_reward, ring_rest) = ring_reward_payout.split(ring_per_reporter);
			ring_reward_payout = ring_rest;

			// this cancels out the reporter reward imbalance internally, leading
			// to no change in total issuance.
			T::RingCurrency::resolve_creating(reporter, ring_reporter_reward);
		}

		// the rest goes to the on-slash imbalance handler (e.g. treasury)
		ring_slashed.subsume(ring_reward_payout); // remainder of reward division remains.
		T::RingSlash::on_unbalanced(ring_slashed);
		// nobody to pay out to or nothing to pay;
		// just treat the whole value as slashed.
		T::KtonSlash::on_unbalanced(slashed_kton);
	} else if reward_ring_is_zero && !reward_kton_is_zero {
		let kton_reward_payout = reward_payout.k.min(slashed_kton.peek());
		let (mut kton_reward_payout, mut kton_slashed) = slashed_kton.split(kton_reward_payout);

		let kton_per_reporter = kton_reward_payout.peek() / (reporters.len() as u32).into();
		for reporter in reporters {
			let (kton_reporter_reward, kton_rest) = kton_reward_payout.split(kton_per_reporter);
			kton_reward_payout = kton_rest;

			// this cancels out the reporter reward imbalance internally, leading
			// to no change in total issuance.
			T::KtonCurrency::resolve_creating(reporter, kton_reporter_reward);
		}

		// the rest goes to the on-slash imbalance handler (e.g. treasury)
		kton_slashed.subsume(kton_reward_payout); // remainder of reward division remains.
		T::KtonSlash::on_unbalanced(kton_slashed);
		// nobody to pay out to or nothing to pay;
		// just treat the whole value as slashed.
		T::RingSlash::on_unbalanced(slashed_ring);
	} else if !reward_ring_is_zero && !reward_kton_is_zero {
		// take rewards out of the slashed imbalance.
		let ring_reward_payout = reward_payout.r.min(slashed_ring.peek());
		let (mut ring_reward_payout, mut ring_slashed) = slashed_ring.split(ring_reward_payout);
		let kton_reward_payout = reward_payout.k.min(slashed_kton.peek());
		let (mut kton_reward_payout, mut kton_slashed) = slashed_kton.split(kton_reward_payout);

		let ring_per_reporter = ring_reward_payout.peek() / (reporters.len() as u32).into();
		let kton_per_reporter = kton_reward_payout.peek() / (reporters.len() as u32).into();
		for reporter in reporters {
			let (ring_reporter_reward, ring_rest) = ring_reward_payout.split(ring_per_reporter);
			ring_reward_payout = ring_rest;

			// this cancels out the reporter reward imbalance internally, leading
			// to no change in total issuance.
			T::RingCurrency::resolve_creating(reporter, ring_reporter_reward);

			let (kton_reporter_reward, kton_rest) = kton_reward_payout.split(kton_per_reporter);
			kton_reward_payout = kton_rest;

			// this cancels out the reporter reward imbalance internally, leading
			// to no change in total issuance.
			T::KtonCurrency::resolve_creating(reporter, kton_reporter_reward);
		}

		// the rest goes to the on-slash imbalance handler (e.g. treasury)
		ring_slashed.subsume(ring_reward_payout); // remainder of reward division remains.
		T::RingSlash::on_unbalanced(ring_slashed);

		// the rest goes to the on-slash imbalance handler (e.g. treasury)
		kton_slashed.subsume(kton_reward_payout); // remainder of reward division remains.
		T::KtonSlash::on_unbalanced(kton_slashed);
	}
}

// #[cfg(test)]
// mod tests {
// 	use super::*;
//
// 	#[test]
// 	fn span_contains_era() {
// 		// unbounded end
// 		let span = SlashingSpan {
// 			index: 0,
// 			start: 1000,
// 			length: None,
// 		};
// 		assert!(!span.contains_era(0));
// 		assert!(!span.contains_era(999));
//
// 		assert!(span.contains_era(1000));
// 		assert!(span.contains_era(1001));
// 		assert!(span.contains_era(10000));
//
// 		// bounded end - non-inclusive range.
// 		let span = SlashingSpan {
// 			index: 0,
// 			start: 1000,
// 			length: Some(10),
// 		};
// 		assert!(!span.contains_era(0));
// 		assert!(!span.contains_era(999));
//
// 		assert!(span.contains_era(1000));
// 		assert!(span.contains_era(1001));
// 		assert!(span.contains_era(1009));
// 		assert!(!span.contains_era(1010));
// 		assert!(!span.contains_era(1011));
// 	}
//
// 	#[test]
// 	fn single_slashing_span() {
// 		let spans = SlashingSpans {
// 			span_index: 0,
// 			last_start: 1000,
// 			last_nonzero_slash: 0,
// 			prior: Vec::new(),
// 		};
//
// 		assert_eq!(
// 			spans.iter().collect::<Vec<_>>(),
// 			vec![SlashingSpan {
// 				index: 0,
// 				start: 1000,
// 				length: None
// 			}],
// 		);
// 	}
//
// 	#[test]
// 	fn many_prior_spans() {
// 		let spans = SlashingSpans {
// 			span_index: 10,
// 			last_start: 1000,
// 			last_nonzero_slash: 0,
// 			prior: vec![10, 9, 8, 10],
// 		};
//
// 		assert_eq!(
// 			spans.iter().collect::<Vec<_>>(),
// 			vec![
// 				SlashingSpan {
// 					index: 10,
// 					start: 1000,
// 					length: None
// 				},
// 				SlashingSpan {
// 					index: 9,
// 					start: 990,
// 					length: Some(10)
// 				},
// 				SlashingSpan {
// 					index: 8,
// 					start: 981,
// 					length: Some(9)
// 				},
// 				SlashingSpan {
// 					index: 7,
// 					start: 973,
// 					length: Some(8)
// 				},
// 				SlashingSpan {
// 					index: 6,
// 					start: 963,
// 					length: Some(10)
// 				},
// 			],
// 		)
// 	}
//
// 	#[test]
// 	fn pruning_spans() {
// 		let mut spans = SlashingSpans {
// 			span_index: 10,
// 			last_start: 1000,
// 			last_nonzero_slash: 0,
// 			prior: vec![10, 9, 8, 10],
// 		};
//
// 		assert_eq!(spans.prune(981), Some((6, 8)));
// 		assert_eq!(
// 			spans.iter().collect::<Vec<_>>(),
// 			vec![
// 				SlashingSpan {
// 					index: 10,
// 					start: 1000,
// 					length: None
// 				},
// 				SlashingSpan {
// 					index: 9,
// 					start: 990,
// 					length: Some(10)
// 				},
// 				SlashingSpan {
// 					index: 8,
// 					start: 981,
// 					length: Some(9)
// 				},
// 			],
// 		);
//
// 		assert_eq!(spans.prune(982), None);
// 		assert_eq!(
// 			spans.iter().collect::<Vec<_>>(),
// 			vec![
// 				SlashingSpan {
// 					index: 10,
// 					start: 1000,
// 					length: None
// 				},
// 				SlashingSpan {
// 					index: 9,
// 					start: 990,
// 					length: Some(10)
// 				},
// 				SlashingSpan {
// 					index: 8,
// 					start: 981,
// 					length: Some(9)
// 				},
// 			],
// 		);
//
// 		assert_eq!(spans.prune(989), None);
// 		assert_eq!(
// 			spans.iter().collect::<Vec<_>>(),
// 			vec![
// 				SlashingSpan {
// 					index: 10,
// 					start: 1000,
// 					length: None
// 				},
// 				SlashingSpan {
// 					index: 9,
// 					start: 990,
// 					length: Some(10)
// 				},
// 				SlashingSpan {
// 					index: 8,
// 					start: 981,
// 					length: Some(9)
// 				},
// 			],
// 		);
//
// 		assert_eq!(spans.prune(1000), Some((8, 10)));
// 		assert_eq!(
// 			spans.iter().collect::<Vec<_>>(),
// 			vec![SlashingSpan {
// 				index: 10,
// 				start: 1000,
// 				length: None
// 			},],
// 		);
//
// 		assert_eq!(spans.prune(2000), None);
// 		assert_eq!(
// 			spans.iter().collect::<Vec<_>>(),
// 			vec![SlashingSpan {
// 				index: 10,
// 				start: 2000,
// 				length: None
// 			},],
// 		);
//
// 		// now all in one shot.
// 		let mut spans = SlashingSpans {
// 			span_index: 10,
// 			last_start: 1000,
// 			last_nonzero_slash: 0,
// 			prior: vec![10, 9, 8, 10],
// 		};
// 		assert_eq!(spans.prune(2000), Some((6, 10)));
// 		assert_eq!(
// 			spans.iter().collect::<Vec<_>>(),
// 			vec![SlashingSpan {
// 				index: 10,
// 				start: 2000,
// 				length: None
// 			},],
// 		);
// 	}
//
// 	#[test]
// 	fn ending_span() {
// 		let mut spans = SlashingSpans {
// 			span_index: 1,
// 			last_start: 10,
// 			last_nonzero_slash: 0,
// 			prior: Vec::new(),
// 		};
//
// 		assert!(spans.end_span(10));
//
// 		assert_eq!(
// 			spans.iter().collect::<Vec<_>>(),
// 			vec![
// 				SlashingSpan {
// 					index: 2,
// 					start: 11,
// 					length: None
// 				},
// 				SlashingSpan {
// 					index: 1,
// 					start: 10,
// 					length: Some(1)
// 				},
// 			],
// 		);
//
// 		assert!(spans.end_span(15));
// 		assert_eq!(
// 			spans.iter().collect::<Vec<_>>(),
// 			vec![
// 				SlashingSpan {
// 					index: 3,
// 					start: 16,
// 					length: None
// 				},
// 				SlashingSpan {
// 					index: 2,
// 					start: 11,
// 					length: Some(5)
// 				},
// 				SlashingSpan {
// 					index: 1,
// 					start: 10,
// 					length: Some(1)
// 				},
// 			],
// 		);
//
// 		// does nothing if not a valid end.
// 		assert!(!spans.end_span(15));
// 		assert_eq!(
// 			spans.iter().collect::<Vec<_>>(),
// 			vec![
// 				SlashingSpan {
// 					index: 3,
// 					start: 16,
// 					length: None
// 				},
// 				SlashingSpan {
// 					index: 2,
// 					start: 11,
// 					length: Some(5)
// 				},
// 				SlashingSpan {
// 					index: 1,
// 					start: 10,
// 					length: Some(1)
// 				},
// 			],
// 		);
// 	}
// }
