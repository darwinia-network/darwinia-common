// --- substrate ---
use sp_runtime::assert_eq_error_rate;
// --- darwinia ---
use crate::{
	inflation::*,
	mock::{Balance, *},
	*,
};

#[test]
fn compute_total_payout_should_work() {
	const MILLISECONDS_PER_YEAR: TsInMs = ((36525 * 24 * 60 * 60) / 100) * 1000;

	let initial_issuance = 200_000;
	let hard_cap = 1_000_000;
	// year, expect inflation, expect inflation rate, exp payout fraction
	let inflation_spec = [
		(1_u32, 8000 as Balance, 4_f64, 0_u32),
		(2, 11177, 5.37, 35),
		(3, 13474, 6.15, 50),
		(4, 15270, 6.56, 77),
		(5, 16713, 6.74, 33),
		(6, 17882, 6.76, 81),
		(7, 18826, 6.66, 100),
	];
	let mut total_left: RingBalance<Test> = hard_cap - initial_issuance;

	for (i, &(year, exp_inflation, exp_inflation_rate, exp_payout_fraction)) in
		inflation_spec.iter().enumerate()
	{
		let (payout, inflation) = compute_total_payout::<Test>(
			MILLISECONDS_PER_YEAR,
			((year - 1) as TsInMs) * MILLISECONDS_PER_YEAR,
			total_left,
			Perbill::from_percent(exp_payout_fraction),
		);

		assert_eq_error_rate!(
			payout * 100,
			exp_inflation * exp_payout_fraction as Balance,
			if exp_payout_fraction == 0 {
				0
			} else {
				(i * 10) as Balance
			}
		);
		assert_eq!(inflation, exp_inflation);
		assert_eq!(
			inflation * 100 / (hard_cap - total_left),
			exp_inflation_rate as Balance
		);

		total_left = total_left - inflation;
	}
}
