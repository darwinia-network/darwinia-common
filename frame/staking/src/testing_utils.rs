// --- paritytech ---
use frame_benchmarking::account;
use frame_support::traits::Currency;
use frame_system::RawOrigin;
use sp_runtime::traits::StaticLookup;
// use sp_runtime::Perbill;
// --- darwinia-network ---
use crate::{Pallet as Staking, *};

pub const SEED: u32 = 0;

// /// create `max` validators.
// pub fn create_validators<T: Config>(
// 	max: u32,
// 	balance_factor: u32,
// ) -> Result<Vec<<T::Lookup as StaticLookup>::Source>, &'static str> {
// 	let mut validators: Vec<<T::Lookup as StaticLookup>::Source> = Vec::with_capacity(max as usize);
// 	for i in 0..max {
// 		let (stash, controller) =
// 			create_stash_controller::<T>(i, balance_factor, RewardDestination::Staked)?;
// 		let validator_prefs = ValidatorPrefs {
// 			commission: Perbill::from_percent(50),
// 			..Default::default()
// 		};
// 		<Staking<T>>::validate(RawOrigin::Signed(controller).into(), validator_prefs)?;
// 		let stash_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(stash);
// 		validators.push(stash_lookup);
// 	}
// 	Ok(validators)
// }

/// Create a stash and controller pair.
pub fn create_stash_controller<T: Config>(
	n: u32,
	balance_factor: u32,
	destination: RewardDestination<T::AccountId>,
) -> Result<(T::AccountId, T::AccountId), &'static str> {
	let stash = create_funded_user::<T>("stash", n, balance_factor);
	let controller = create_funded_user::<T>("controller", n, balance_factor);
	let controller_lookup: <T::Lookup as StaticLookup>::Source =
		T::Lookup::unlookup(controller.clone());
	let amount = T::RingCurrency::minimum_balance() * (balance_factor / 10).max(1).into();
	<Staking<T>>::bond(
		RawOrigin::Signed(stash.clone()).into(),
		controller_lookup,
		StakingBalance::RingBalance(amount),
		destination,
		0,
	)?;
	return Ok((stash, controller));
}

/// Grab a funded user.
pub fn create_funded_user<T: Config>(
	string: &'static str,
	n: u32,
	balance_factor: u32,
) -> T::AccountId {
	let user = account(string, n, SEED);
	let balance = T::RingCurrency::minimum_balance() * balance_factor.into();
	T::RingCurrency::make_free_balance_be(&user, balance);
	// ensure T::CurrencyToVote will work correctly.
	T::RingCurrency::issue(balance);
	user
}
