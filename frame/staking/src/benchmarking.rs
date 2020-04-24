//! Staking pallet benchmarking.

// --- crates ---
use rand_chacha::{
	rand_core::{RngCore, SeedableRng},
	ChaChaRng,
};
// --- darwinia ---
use super::*;
use crate::Module as Staking;
// --- substrate ---
use frame_benchmarking::{account, benchmarks};
use frame_system::RawOrigin;
use sp_io::hashing::blake2_256;
use sp_runtime::traits::{Dispatchable, One};

const SEED: u32 = 0;

fn create_funded_user<T: Trait>(string: &'static str, n: u32) -> T::AccountId {
	let user = account(string, n, SEED);
	let balance = T::RingCurrency::minimum_balance() * 100.into();
	T::RingCurrency::make_free_balance_be(&user, balance);
	user
}

pub fn create_stash_controller<T: Trait>(
	n: u32,
) -> Result<(T::AccountId, T::AccountId), &'static str> {
	let stash = create_funded_user::<T>("stash", n);
	let controller = create_funded_user::<T>("controller", n);
	let controller_lookup: <T::Lookup as StaticLookup>::Source =
		T::Lookup::unlookup(controller.clone());
	let reward_destination = RewardDestination::Staked { promise_month: 0 };
	let amount = StakingBalance::RingBalance(T::RingCurrency::minimum_balance() * 10.into());
	<Staking<T>>::bond(
		RawOrigin::Signed(stash.clone()).into(),
		controller_lookup,
		amount,
		reward_destination,
		0,
	)?;
	return Ok((stash, controller));
}

fn create_validators<T: Trait>(
	max: u32,
) -> Result<Vec<<T::Lookup as StaticLookup>::Source>, &'static str> {
	let mut validators: Vec<<T::Lookup as StaticLookup>::Source> = Vec::with_capacity(max as usize);
	for i in 0..max {
		let (stash, controller) = create_stash_controller::<T>(i)?;
		let validator_prefs = ValidatorPrefs {
			commission: Perbill::from_percent(50),
		};
		<Staking<T>>::validate(RawOrigin::Signed(controller).into(), validator_prefs)?;
		let stash_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(stash);
		validators.push(stash_lookup);
	}
	Ok(validators)
}

// This function generates one validator being nominated by n nominators.
// It starts an era and creates pending payouts.
pub fn create_validator_with_nominators<T: Trait>(
	n: u32,
	upper_bound: u32,
) -> Result<T::AccountId, &'static str> {
	let mut points_total = 0;
	let mut points_individual = Vec::new();

	MinimumValidatorCount::put(0);

	let (v_stash, v_controller) = create_stash_controller::<T>(0)?;
	let validator_prefs = ValidatorPrefs {
		commission: Perbill::from_percent(50),
	};
	<Staking<T>>::validate(
		RawOrigin::Signed(v_controller.clone()).into(),
		validator_prefs,
	)?;
	let stash_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(v_stash.clone());

	points_total += 10;
	points_individual.push((v_stash, 10));

	// Give the validator n nominators, but keep total users in the system the same.
	for i in 0..upper_bound {
		let (_n_stash, n_controller) = create_stash_controller::<T>(u32::max_value() - i)?;
		if i < n {
			<Staking<T>>::nominate(
				RawOrigin::Signed(n_controller.clone()).into(),
				vec![stash_lookup.clone()],
			)?;
		}
	}

	ValidatorCount::put(1);

	// Start a new Era
	let new_validators = <Staking<T>>::new_era(SessionIndex::one()).unwrap();

	assert!(new_validators.len() == 1);

	// Give Era Points
	let reward = EraRewardPoints::<T::AccountId> {
		total: points_total,
		individual: points_individual.into_iter().collect(),
	};

	let current_era = CurrentEra::get().unwrap();
	<ErasRewardPoints<T>>::insert(current_era, reward);

	// Create reward pool
	let total_payout = T::RingCurrency::minimum_balance() * 1000.into();
	<ErasValidatorReward<T>>::insert(current_era, total_payout);

	Ok(v_controller)
}

// This function generates one nominator nominating v validators.
// It starts an era and creates pending payouts.
pub fn create_nominator_with_validators<T: Trait>(
	v: u32,
) -> Result<(T::AccountId, Vec<T::AccountId>), &'static str> {
	let mut validators = Vec::new();
	let mut points_total = 0;
	let mut points_individual = Vec::new();

	MinimumValidatorCount::put(0);

	// Create v validators
	let mut validator_lookups = Vec::new();
	for i in 0..v {
		let (v_stash, v_controller) = create_stash_controller::<T>(i)?;
		let validator_prefs = ValidatorPrefs {
			commission: Perbill::from_percent(50),
		};
		<Staking<T>>::validate(
			RawOrigin::Signed(v_controller.clone()).into(),
			validator_prefs,
		)?;
		let stash_lookup: <T::Lookup as StaticLookup>::Source =
			T::Lookup::unlookup(v_stash.clone());

		points_total += 10;
		points_individual.push((v_stash.clone(), 10));
		validator_lookups.push(stash_lookup);
		// Add to the list if it is less than the number we want the nominator to have
		if validators.len() < v as usize {
			validators.push(v_stash.clone())
		}
	}

	// Create a nominator
	let (_n_stash, n_controller) = create_stash_controller::<T>(u32::max_value())?;
	<Staking<T>>::nominate(
		RawOrigin::Signed(n_controller.clone()).into(),
		validator_lookups,
	)?;

	ValidatorCount::put(v);

	// Start a new Era
	let new_validators = <Staking<T>>::new_era(SessionIndex::one()).unwrap();

	assert!(new_validators.len() == v as usize);

	// Give Era Points
	let reward = EraRewardPoints::<T::AccountId> {
		total: points_total,
		individual: points_individual.into_iter().collect(),
	};

	let current_era = CurrentEra::get().unwrap();
	ErasRewardPoints::<T>::insert(current_era, reward);

	// Create reward pool
	let total_payout = T::RingCurrency::minimum_balance() * 1000.into();
	<ErasValidatorReward<T>>::insert(current_era, total_payout);

	Ok((n_controller, validators))
}

benchmarks! {
	_{
		// User account seed
		let u in 0 .. 1000 => ();
	}

	bond {
		let u in ...;
		let stash = create_funded_user::<T>("stash", u);
		let controller = create_funded_user::<T>("controller", u);
		let controller_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(controller);
		let reward_destination = RewardDestination::Staked { promise_month: 0 };
		let amount = StakingBalance::RingBalance(T::RingCurrency::minimum_balance() * 10.into());
		let promise_month = 0;
	}: _(RawOrigin::Signed(stash), controller_lookup, amount, reward_destination, promise_month)

	bond_extra {
		let u in ...;
		let (stash, _) = create_stash_controller::<T>(u)?;
		let max_additional = StakingBalance::RingBalance(T::RingCurrency::minimum_balance() * 10.into());
		let promise_month = 0;
	}: _(RawOrigin::Signed(stash), max_additional, promise_month)

	// TODO: deposit_extra

	unbond {
		let u in ...;
		let (_, controller) = create_stash_controller::<T>(u)?;
		let amount = StakingBalance::RingBalance(T::RingCurrency::minimum_balance() * 10.into());
	}: _(RawOrigin::Signed(controller), amount)

	validate {
		let u in ...;
		let (_, controller) = create_stash_controller::<T>(u)?;
		let prefs = ValidatorPrefs::default();
	}: _(RawOrigin::Signed(controller), prefs)

	// Worst case scenario, MAX_NOMINATIONS
	nominate {
		let n in 1 .. MAX_NOMINATIONS as u32;
		let (_, controller) = create_stash_controller::<T>(n + 1)?;
		let validators = create_validators::<T>(n)?;
	}: _(RawOrigin::Signed(controller), validators)

	chill {
		let u in ...;
		let (_, controller) = create_stash_controller::<T>(u)?;
	}: _(RawOrigin::Signed(controller))

	set_payee {
		let u in ...;
		let (_, controller) = create_stash_controller::<T>(u)?;
	}: _(RawOrigin::Signed(controller), RewardDestination::Controller)

	set_controller {
		let u in ...;
		let (stash, _) = create_stash_controller::<T>(u)?;
		let new_controller = create_funded_user::<T>("new_controller", u);
		let new_controller_lookup = T::Lookup::unlookup(new_controller);
	}: _(RawOrigin::Signed(stash), new_controller_lookup)

	set_validator_count {
		let c in 0 .. 1000;
	}: _(RawOrigin::Root, c)

	force_no_eras { let i in 1 .. 1; }: _(RawOrigin::Root)

	force_new_era {let i in 1 .. 1; }: _(RawOrigin::Root)

	force_new_era_always { let i in 1 .. 1; }: _(RawOrigin::Root)

	// Worst case scenario, the list of invulnerables is very long.
	set_invulnerables {
		let v in 0 .. 1000;
		let mut invulnerables = Vec::new();
		for i in 0 .. v {
			invulnerables.push(account("invulnerable", i, SEED));
		}
	}: _(RawOrigin::Root, invulnerables)

	force_unstake {
		let u in ...;
		let (stash, _) = create_stash_controller::<T>(u)?;
	}: _(RawOrigin::Root, stash)

	cancel_deferred_slash {
		let s in 1 .. 1000;
		let mut unapplied_slashes = Vec::new();
		let era = EraIndex::one();
		for _ in 0 .. 1000 {
			unapplied_slashes.push(<UnappliedSlash<
				T::AccountId,
				RingBalance<T>,
				KtonBalance<T>,
			>>::default());
		}
		<UnappliedSlashes<T>>::insert(era, &unapplied_slashes);

		let slash_indices: Vec<u32> = (0 .. s).collect();
	}: _(RawOrigin::Root, era, slash_indices)

	payout_stakers {
		let n in 1 .. MAX_NOMINATIONS as u32;
		let validator = create_validator_with_nominators::<T>(n, MAX_NOMINATIONS as u32)?;
		let current_era = CurrentEra::get().unwrap();
		let caller = account("caller", n, SEED);
	}: _(RawOrigin::Signed(caller), validator, current_era)

	set_history_depth {
		let e in 1 .. 100;
		HistoryDepth::put(e);
		CurrentEra::put(e);
		for i in 0 .. e {
			<ErasStakers<T>>::insert(
				i,
				T::AccountId::default(),
				Exposure::<T::AccountId, RingBalance<T>, KtonBalance<T>>::default(),
			);
			<ErasStakersClipped<T>>::insert(
				i,
				T::AccountId::default(),
				Exposure::<T::AccountId, RingBalance<T>, KtonBalance<T>>::default(),
			);
			<ErasValidatorPrefs<T>>::insert(i, T::AccountId::default(), ValidatorPrefs::default());
			<ErasValidatorReward<T>>::insert(i, RingBalance::<T>::one());
			<ErasRewardPoints<T>>::insert(i, EraRewardPoints::<T::AccountId>::default());
			ErasTotalStake::insert(i, Power::one());
			ErasStartSessionIndex::insert(i, i);
		}
	}: _(RawOrigin::Root, EraIndex::zero())

	reap_stash {
		let u in 1 .. 1000;
		let (stash, controller) = create_stash_controller::<T>(u)?;
		T::RingCurrency::make_free_balance_be(&stash, 0.into());
	}: _(RawOrigin::Signed(controller), stash)

	new_era {
		let v in 1 .. 10;
		let n in 1 .. 100;
		MinimumValidatorCount::put(0);
		create_validators_with_nominators_for_era::<T>(v, n)?;
		let session_index = SessionIndex::one();
	}: {
		let validators = <Staking<T>>::new_era(session_index).ok_or("`new_era` failed")?;
		assert!(validators.len() == v as usize);
	}

	// TODO: do_slash
	// do_slash {
	// 	let l in 1 .. 1000;
	// 	let (stash, controller) = create_stash_controller::<T>(0)?;
	// 	let mut staking_ledger = <Ledger<T>>::get(controller.clone()).unwrap();
	// 	let unlock_chunk = UnlockChunk::<BalanceOf<T>> {
	// 		value: 1.into(),
	// 		era: EraIndex::zero(),
	// 	};
	// 	for _ in 0 .. l {
	// 		staking_ledger.unlocking.push(unlock_chunk.clone())
	// 	}
	// 	<Ledger<T>>::insert(controller.clone(), staking_ledger.clone());
	// 	let slash_amount = T::Currency::minimum_balance() * 10.into();
	// }: {
	// 	crate::slashing::do_slash::<T>(
	// 		&stash,
	// 		slash_amount,
	// 		&mut BalanceOf::<T>::zero(),
	// 		&mut NegativeImbalanceOf::<T>::zero()
	// 	);
	// }

	payout_all {
		let v in 1 .. 10;
		let n in 1 .. 100;
		MinimumValidatorCount::put(0);
		create_validators_with_nominators_for_era::<T>(v, n)?;
		// Start a new Era
		let new_validators = <Staking<T>>::new_era(SessionIndex::one()).unwrap();
		assert!(new_validators.len() == v as usize);

		let current_era = CurrentEra::get().unwrap();
		let mut points_total = 0;
		let mut points_individual = Vec::new();
		let mut payout_calls = Vec::new();

		for validator in new_validators.iter() {
			points_total += 10;
			points_individual.push((validator.clone(), 10));
			payout_calls.push(<Call<T>>::payout_stakers(validator.clone(), current_era))
		}

		// Give Era Points
		let reward = EraRewardPoints::<T::AccountId> {
			total: points_total,
			individual: points_individual.into_iter().collect(),
		};

		<ErasRewardPoints<T>>::insert(current_era, reward);

		// Create reward pool
		let total_payout = T::Currency::minimum_balance() * 1000.into();
		<ErasValidatorReward<T>>::insert(current_era, total_payout);

		let caller: T::AccountId = account("caller", 0, SEED);
	}: {
		for call in payout_calls {
			call.dispatch(RawOrigin::Signed(caller.clone()).into())?;
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::mock::*;
	use crate::*;
	use frame_support::assert_ok;

	use crate::benchmarking::{
		create_validator_with_nominators, create_validators_with_nominators_for_era,
		SelectedBenchmark,
	};

	#[test]
	fn create_validators_with_nominators_for_era_works() {
		ExtBuilder::default()
			.stakers(false)
			.build()
			.execute_with(|| {
				let v = 10;
				let n = 100;

				create_validators_with_nominators_for_era::<Test>(v, n).unwrap();

				let count_validators = Validators::<Test>::iter().count();
				let count_nominators = Nominators::<Test>::iter().count();

				assert_eq!(count_validators, v as usize);
				assert_eq!(count_nominators, n as usize);
			});
	}

	#[test]
	fn create_validator_with_nominators_works() {
		ExtBuilder::default()
			.stakers(false)
			.build()
			.execute_with(|| {
				let n = 10;

				let validator =
					create_validator_with_nominators::<Test>(n, MAX_NOMINATIONS as u32).unwrap();

				let current_era = CurrentEra::get().unwrap();
				let controller = validator;
				let ledger = Staking::ledger(&controller).unwrap();
				let stash = ledger.stash;

				let original_free_balance = Ring::free_balance(&stash);
				assert_ok!(Staking::payout_stakers(
					Origin::signed(1337),
					stash,
					current_era
				));
				let new_free_balance = Ring::free_balance(&stash);

				assert!(original_free_balance < new_free_balance);
			});
	}

	#[test]
	fn test_payout_all() {
		ExtBuilder::default()
			.has_stakers(false)
			.build()
			.execute_with(|| {
				let v = 10;
				let n = 100;

				let selected_benchmark = SelectedBenchmark::payout_all;
				let c = vec![
					(frame_benchmarking::BenchmarkParameter::v, v),
					(frame_benchmarking::BenchmarkParameter::n, n),
				];
				let closure_to_benchmark =
					<SelectedBenchmark as frame_benchmarking::BenchmarkingSetup<Test>>::instance(
						&selected_benchmark,
						&c,
					)
					.unwrap();

				assert_ok!(closure_to_benchmark());
			});
	}
}
