// --- crates ---
use num_integer::Roots;
// --- substrate ---
use sp_core::U256;
use sp_runtime::{Perbill, Perquintill};
use sp_std::convert::TryInto;
// --- darwinia ---
use crate::*;

/// The total payout to all validators (and their nominators) per era and maximum payout.
///
/// Defined as such:
/// `staker-payout = yearly_inflation(npos_token_staked / total_tokens) * total_tokens / era_per_year`
/// `maximum-payout = max_yearly_inflation * total_tokens / era_per_year`
///
/// `era_duration` is expressed in millisecond.
///
/// Formula:
///.  1 - (99 / 100) ^ sqrt(year)
pub fn compute_total_payout<T: Trait>(
	era_duration: TsInMs,
	living_time: TsInMs,
	total_left: RingBalance<T>,
	payout_fraction: Perbill,
) -> (RingBalance<T>, RingBalance<T>) {
	// Milliseconds per year for the Julian year (365.25 days).
	const MILLISECONDS_PER_YEAR: TsInMs = ((36525 * 24 * 60 * 60) / 100) * 1000;

	let maximum = {
		let maximum = {
			let portion =
				Perquintill::from_rational_approximation(era_duration, MILLISECONDS_PER_YEAR);
			let total_left = total_left.saturated_into::<Balance>();

			portion * total_left
		};
		let year = {
			let year = living_time / MILLISECONDS_PER_YEAR + 1;

			year as u32
		};

		maximum
			- maximum * 99_u128.saturating_pow(year.sqrt()) / 100_u128.saturating_pow(year.sqrt())
	};
	let payout = payout_fraction * maximum;

	(
		<RingBalance<T>>::saturated_from::<Balance>(payout),
		<RingBalance<T>>::saturated_from::<Balance>(maximum),
	)
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
