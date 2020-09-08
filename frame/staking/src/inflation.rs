// --- github ---
use substrate_fixed::{
	transcendental::{pow, sqrt},
	types::I64F64,
};
// --- substrate ---
use frame_support::debug::*;
use sp_arithmetic::helpers_128bit::multiply_by_rational;
use sp_core::U256;
use sp_runtime::Perquintill;
use sp_std::convert::TryInto;
// --- darwinia ---
use crate::*;

// Milliseconds per year for the Julian year (365.25 days).
pub const MILLISECONDS_PER_YEAR: TsInMs = (366 * 24 * 60 * 60) * 1000;

/// The total payout to all validators (and their nominators) per era and maximum payout.
///
/// Defined as such:
/// `staker-payout = yearly_inflation(npos_token_staked / total_tokens) * total_tokens / era_per_year`
/// `maximum-payout = max_yearly_inflation * total_tokens / era_per_year`
///
/// `era_duration` is expressed in millisecond.
pub fn compute_total_payout<T: Trait>(
	era_duration: TsInMs,
	living_time: TsInMs,
	total_left: RingBalance<T>,
	payout_fraction: Perquintill,
) -> (RingBalance<T>, RingBalance<T>) {
	info!(
		target: "darwinia-staking",
		"era_duration: {}, living_time: {}, total_left: {:?}, payout_fraction: {:?}",
		era_duration,
		living_time,
		total_left,
		payout_fraction,
	);

	let inflation = {
		let maximum = {
			let total_left = total_left.saturated_into::<Balance>();

			multiply_by_rational(total_left, era_duration as _, MILLISECONDS_PER_YEAR as _)
				.unwrap_or(0)
		};
		let year = {
			let year = living_time / MILLISECONDS_PER_YEAR + 1;

			year as u32
		};

		compute_inflation(maximum, year).unwrap_or(0)
	};
	let payout = payout_fraction * inflation;

	(
		<RingBalance<T>>::saturated_from::<Balance>(payout),
		<RingBalance<T>>::saturated_from::<Balance>(inflation),
	)
}

/// Formula:
/// 	1 - (99 / 100) ^ sqrt(year)
pub fn compute_inflation(maximum: Balance, year: u32) -> Option<u128> {
	type F64 = I64F64;

	if let Ok(a) = sqrt::<F64, F64>(F64::from_num(year)) {
		let b: F64 = F64::from_num(99) / 100;

		if let Ok(c) = pow::<F64, F64>(b, a) {
			let d: F64 = F64::from_num(1) - c;
			let e: F64 = F64::from_num(maximum) * d;

			#[cfg(test)]
			{
				let a_f64 = (year as f64).sqrt();
				// eprintln!("{}\n{}", a, a_f64);
				let b_f64 = 0.99_f64;
				// eprintln!("{}\n{}", b, b_f64);
				let c_f64 = b_f64.powf(a_f64);
				// eprintln!("{}\n{}", c, c_f64);
				let d_f64 = 1.00_f64 - c_f64;
				// eprintln!("{}\n{}", d, d_f64);
				let e_f64 = maximum as f64 * d_f64;
				// eprintln!("{}\n{}", e, e_f64);

				sp_runtime::assert_eq_error_rate!(
					e.floor(),
					e_f64 as u128,
					if e_f64 == 0.00_f64 { 0 } else { 3 }
				);
			}

			return Some(e.floor().to_num());
		} else {
			error!(target: "darwniia-staking", "Compute Inflation Failed at Step 1");
		}
	} else {
		error!(target: "darwniia-staking", "Compute Inflation Failed at Step 0");
	}

	None
}

// consistent with the formula in smart contract in evolution land which can be found in
// https://github.com/evolutionlandorg/bank/blob/master/contracts/GringottsBank.sol#L280
pub fn compute_kton_return<T: Trait>(value: RingBalance<T>, months: u64) -> KtonBalance<T> {
	let value = value.saturated_into::<u64>();
	let no = U256::from(67).pow(U256::from(months));
	let de = U256::from(66).pow(U256::from(months));

	let quotient = no / de;
	let remainder = no % de;
	let res = U256::from(value)
		* (U256::from(1000) * (quotient - 1) + U256::from(1000) * remainder / de)
		/ U256::from(1_970_000);
	res.as_u128().try_into().unwrap_or_default()
}
