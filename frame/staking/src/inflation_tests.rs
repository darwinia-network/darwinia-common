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

	let initial_issuance = 2_000_000_000;
	let hard_cap = 10_000_000_000;

	// year, expect inflation, expect inflation rate, exp payout fraction
	let inflation_spec = [
		(1_u32, 80000000 as Balance, 4_f64, 0_u32),
		(2, 111773288, 5.37, 35),
		(3, 134746988, 6.15, 50),
		(4, 152702246, 6.56, 77),
		(5, 167131170, 6.74, 33),
		(6, 178823310, 6.76, 81),
		(7, 188269290, 6.66, 100),
		(8, 195807997, 6.50, 50),
		(9, 201691938, 6.28, 50),
		(10, 206120193, 6.04, 50),
		(11, 209256586, 5.79, 50),
		(12, 211240394, 5.52, 50),
		(13, 212192984, 5.26, 50),
		(14, 212222107, 4.99, 50),
		(15, 211424761, 4.74, 50),
		(16, 209889164, 4.49, 50),
		(17, 207696141, 4.25, 50),
		(18, 204920129, 4.03, 50),
		(19, 201629917, 3.81, 50),
		(20, 197889214, 3.60, 50),
		(21, 0, 0.0, 50),

	];
	let mut total_left: RingBalance<Test> = hard_cap - initial_issuance;

	for (_i, &(year, exp_inflation, exp_inflation_rate, exp_payout_fraction)) in
		inflation_spec.iter().enumerate()
	{
		let (payout, inflation) = compute_total_payout::<Test>(
			MILLISECONDS_PER_YEAR,
			((year - 1) as TsInMs) * MILLISECONDS_PER_YEAR,
			total_left,
			Perbill::from_percent(exp_payout_fraction),
		);

		assert_eq_error_rate!(
			(payout * 100) as i128,
			(inflation * exp_payout_fraction as u128) as i128,
			100
		);

		assert_eq_error_rate!(
			inflation as i128,
			exp_inflation as i128,
			1300000
		);
		assert_eq_error_rate!(
			(inflation * 10000 / (hard_cap - total_left)) as i128,
			(exp_inflation_rate * 100.0) as i128,
			3
		);

		total_left = total_left - inflation;
	}
}

#[test]
fn calc_error_rate() {
	const MILLISECONDS_PER_YEAR: TsInMs = ((36525 * 24 * 60 * 60) / 100) * 1000;

	let initial_issuance = 2_000_000_000;
	let hard_cap = 10_000_000_000;
	let mut total_left = hard_cap - initial_issuance;
	let mut total_inflation = 0;
	// 100 years
	for year in 1_u32..101 {
		let (payout, inflation) = compute_total_payout::<Test>(
			MILLISECONDS_PER_YEAR,
			((year - 1) as TsInMs) * MILLISECONDS_PER_YEAR,
			total_left,
			Perbill::from_percent(0),
		);

		let inflation_rate = inflation * 10000 / (hard_cap - total_left);

		println!("year {}: {}, {}", year, inflation, inflation_rate);
		total_inflation += inflation;
		total_left = total_left - inflation;
	}

	println!("total inflation: {}", total_inflation);
	println!("total left: {}", total_left);
}
