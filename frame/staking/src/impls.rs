// --- core ---
use core::marker::PhantomData;
// --- paritytech ---
use frame_election_provider_support::*;
use frame_support::{
	dispatch::{DispatchError, DispatchResult, DispatchResultWithPostInfo, WithPostDispatchInfo},
	ensure,
	traits::{
		Currency, EstimateNextNewSession, ExistenceRequirement, Get, Imbalance, OnUnbalanced,
		UnixTime, WithdrawReasons,
	},
	weights::{DispatchClass, Weight},
};
use sp_runtime::{
	helpers_128bit,
	traits::{AccountIdConversion, Bounded, Convert, Saturating, Zero},
	Perbill, Perquintill, SaturatedConversion,
};
use sp_staking::{offence::*, *};
use sp_std::{borrow::ToOwned, collections::btree_map::BTreeMap, prelude::*};
// --- darwinia-network ---
use crate::*;
use darwinia_staking_rpc_runtime_api::RuntimeDispatchInfo;
use darwinia_support::{balance::*, traits::OnDepositRedeem};

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
			let kton_positive_imbalance = T::KtonCurrency::deposit_creating(&stash, kton_return);

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
				// dbg!(Self::currency_to_power::<_>(
				// 	l.active_ring,
				// 	Self::ring_pool()
				// ));

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
		let current_era = <CurrentEra<T>>::get().ok_or_else(|| {
			<Error<T>>::InvalidEraToReward
				.with_weight(T::WeightInfo::payout_stakers_alive_staked(0))
		})?;
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

		let controller = Self::bonded(&validator_stash).ok_or_else(|| {
			<Error<T>>::NotStash.with_weight(T::WeightInfo::payout_stakers_alive_staked(0))
		})?;
		let mut ledger = <Ledger<T>>::get(&controller).ok_or_else(|| <Error<T>>::NotController)?;

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

		Self::deposit_event(Event::<T>::PayoutStarted(era, ledger.stash.clone()));

		// Due to the `payout * percent` there might be some losses
		let mut actual_payout = <RingPositiveImbalance<T>>::zero();

		// We can now make total validator payout:
		if let Some(imbalance) = Self::make_payout(
			&ledger.stash,
			validator_staking_payout + validator_commission_payout,
		) {
			let payout = imbalance.peek();

			actual_payout.subsume(imbalance);

			Self::deposit_event(Event::Rewarded(ledger.stash, payout));
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

				let e = <Event<T>>::Rewarded(nominator.who.clone(), payout);

				Self::deposit_event(e);
			}
		}

		T::RingCurrency::settle(
			&module_account,
			actual_payout,
			WithdrawReasons::all(),
			ExistenceRequirement::KeepAlive,
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
			RewardDestination::Stash => T::RingCurrency::deposit_into_existing(stash, amount).ok(),
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
	pub fn new_session(session_index: SessionIndex, is_genesis: bool) -> Option<Vec<AccountId<T>>> {
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

			Self::deposit_event(Event::EraPaid(active_era.index, validator_payout, rest));

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
		let election_result = if is_genesis {
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

		Self::deposit_event(Event::StakersElected);

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

						let (origin_ring_balance, origin_kton_balance) = Self::stake_of(&nominator);
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
	/// This function is self-weighing as [`DispatchClass::Mandatory`].
	///
	/// ### Slashing
	///
	/// All nominations that have been submitted before the last non-zero slash of the validator are
	/// auto-chilled.
	pub fn get_npos_voters() -> Vec<(AccountId<T>, VoteWeight, Vec<AccountId<T>>)> {
		let weight_of =
			|account_id: &AccountId<T>| -> VoteWeight { Self::power_of(account_id) as _ };
		let mut all_voters = Vec::new();
		let mut validator_count = 0u32;

		for (validator, _) in <Validators<T>>::iter() {
			// Append self vote
			let self_vote = (
				validator.clone(),
				weight_of(&validator),
				vec![validator.clone()],
			);

			all_voters.push(self_vote);
			validator_count.saturating_inc();
		}

		// Collect all slashing spans into a BTreeMap for further queries.
		let slashing_spans = <SlashingSpans<T>>::iter().collect::<BTreeMap<_, _>>();
		let mut nominator_count = 0u32;

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

				all_voters.push((nominator, vote_weight, targets));
				nominator_count.saturating_inc();
			}
		}

		Self::register_weight(T::WeightInfo::get_npos_voters(
			validator_count,
			nominator_count,
			slashing_spans.len() as u32,
		));

		all_voters
	}

	/// Get the targets for an upcoming npos election.
	///
	/// This function is self-weighing as [`DispatchClass::Mandatory`].
	pub fn get_npos_targets() -> Vec<AccountId<T>> {
		let mut validator_count = 0u32;
		let targets = Validators::<T>::iter()
			.map(|(v, _)| {
				validator_count.saturating_inc();

				v
			})
			.collect::<Vec<_>>();

		Self::register_weight(T::WeightInfo::get_npos_targets(validator_count));

		targets
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

	/// Register some amount of weight directly with the system pallet.
	///
	/// This is always mandatory weight.
	fn register_weight(weight: Weight) {
		<frame_system::Pallet<T>>::register_extra_weight_unchecked(
			weight,
			DispatchClass::Mandatory,
		);
	}
}

impl<T: Config> ElectionDataProvider<AccountId<T>, T::BlockNumber> for Pallet<T> {
	const MAXIMUM_VOTES_PER_VOTER: u32 = T::MAX_NOMINATIONS;

	fn desired_targets() -> data_provider::Result<u32> {
		Self::register_weight(T::DbWeight::get().reads(1));

		Ok(Self::validator_count())
	}

	fn voters(
		maybe_max_len: Option<usize>,
	) -> data_provider::Result<Vec<(AccountId<T>, VoteWeight, Vec<AccountId<T>>)>> {
		let nominator_count = <CounterForNominators<T>>::get();
		let validator_count = <CounterForValidators<T>>::get();
		let voter_count = nominator_count.saturating_add(validator_count) as usize;

		debug_assert!(<Nominators<T>>::iter().count() as u32 == <CounterForNominators<T>>::get());
		debug_assert!(<Validators<T>>::iter().count() as u32 == <CounterForValidators<T>>::get());

		// register the extra 2 reads
		Self::register_weight(T::DbWeight::get().reads(2));

		if maybe_max_len.map_or(false, |max_len| voter_count > max_len) {
			return Err("Voter snapshot too big");
		}

		Ok(Self::get_npos_voters())
	}

	fn targets(maybe_max_len: Option<usize>) -> data_provider::Result<Vec<AccountId<T>>> {
		let target_count = <CounterForValidators<T>>::get() as usize;

		// register the extra 1 read
		Self::register_weight(T::DbWeight::get().reads(1));

		if maybe_max_len.map_or(false, |max_len| target_count > max_len) {
			return Err("Target snapshot too big");
		}

		Ok(Self::get_npos_targets())
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
	fn add_voter(voter: T::AccountId, weight: VoteWeight, targets: Vec<T::AccountId>) {
		let stake = <RingBalance<T>>::try_from(weight).unwrap_or_else(|_| {
			panic!("cannot convert a VoteWeight into BalanceOf, benchmark needs reconfiguring.")
		});
		<Bonded<T>>::insert(voter.clone(), voter.clone());
		<Ledger<T>>::insert(
			voter.clone(),
			StakingLedger {
				stash: voter.clone(),
				active_ring: stake,
				ring_staking_lock: StakingLock {
					staking_amount: stake,
					..Default::default()
				},
				..Default::default()
			},
		);
		Self::do_add_nominator(
			&voter,
			Nominations {
				targets,
				submitted_in: 0,
				suppressed: false,
			},
		);
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn add_target(target: T::AccountId) {
		let stake = <MinValidatorBond<T>>::get() * 100u32.into();
		<Bonded<T>>::insert(target.clone(), target.clone());
		<Ledger<T>>::insert(
			target.clone(),
			StakingLedger {
				stash: target.clone(),
				active_ring: stake,
				ring_staking_lock: StakingLock {
					staking_amount: stake,
					..Default::default()
				},
				..Default::default()
			},
		);
		Self::do_add_validator(
			&target,
			ValidatorPrefs {
				commission: Perbill::zero(),
				blocked: false,
			},
		);
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn clear() {
		<Bonded<T>>::remove_all(None);
		<Ledger<T>>::remove_all(None);
		<Validators<T>>::remove_all(None);
		<Nominators<T>>::remove_all(None);
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
	fn new_session_genesis(new_index: SessionIndex) -> Option<Vec<(AccountId<T>, ExposureT<T>)>> {
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
impl<T> OnOffenceHandler<AccountId<T>, pallet_session::historical::IdentificationTuple<T>, Weight>
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

			T::RingCurrency::transfer(&backing, &stash, amount, ExistenceRequirement::KeepAlive)?;

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

			T::RingCurrency::transfer(&backing, &stash, amount, ExistenceRequirement::KeepAlive)?;

			<Bonded<T>>::insert(&stash, controller);
			<Payee<T>>::insert(&stash, RewardDestination::Stash);

			<frame_system::Pallet<T>>::inc_consumers(&stash).map_err(|_| <Error<T>>::BadState)?;

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
