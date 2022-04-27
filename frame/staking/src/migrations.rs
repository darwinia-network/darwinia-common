// --- paritytech ---
use frame_support::{traits::Get, weights::Weight};
use sp_runtime::traits::Zero;
// --- darwinia-network ---
use crate::*;

pub fn pre_migrate<T: Config>() -> Result<(), &'static str> {
	assert!(<CounterForValidators<T>>::get().is_zero(), "CounterForValidators already set.");
	assert!(<CounterForNominators<T>>::get().is_zero(), "CounterForNominators already set.");
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
