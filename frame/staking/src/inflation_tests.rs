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
	let initial_issuance = 2_000_000_000;
	let hard_cap = 10_000_000_000;

	// year, expect inflation, expect inflation rate, payout fraction
	let inflation_spec = [
		(1_u32, 80_000_000 as Balance, 4_f64, 0_u64),
		(2, 111_773_288, 5.37, 35),
		(3, 134_746_988, 6.15, 50),
		(4, 152_702_246, 6.56, 77),
		(5, 167_131_170, 6.74, 33),
		(6, 178_823_310, 6.76, 81),
		(7, 188_269_290, 6.66, 100),
		(8, 195_807_997, 6.50, 50),
		(9, 201_691_938, 6.28, 50),
		(10, 206_120_193, 6.04, 50),
		(11, 209_256_586, 5.79, 50),
		(12, 211_240_394, 5.52, 50),
		(13, 212_192_984, 5.26, 50),
		(14, 212_222_107, 4.99, 50),
		(15, 211_424_761, 4.74, 50),
		(16, 209_889_164, 4.49, 50),
		(17, 207_696_141, 4.25, 50),
		(18, 204_920_129, 4.03, 50),
		(19, 201_629_917, 3.81, 50),
		(20, 197_889_214, 3.60, 50),
	];
	let mut total_left: RingBalance<Test> = hard_cap - initial_issuance;

	for &(year, exp_inflation, exp_inflation_rate, payout_fraction) in inflation_spec.iter().skip(1)
	{
		let payout_fraction = Perquintill::from_percent(payout_fraction);
		let (payout, inflation) = compute_total_payout::<Test>(
			MILLISECONDS_PER_YEAR,
			((year - 1) as TsInMs) * MILLISECONDS_PER_YEAR,
			total_left,
			payout_fraction,
		);

		assert_eq!(payout, payout_fraction * inflation);

		eprintln!("{}\n{}\n", inflation, exp_inflation);

		// assert_eq_error_rate!(inflation as i128, exp_inflation as i128, 1300000);
		// assert_eq_error_rate!(
		// (inflation * 10000 / (hard_cap - total_left)) as i128,
		// (exp_inflation_rate * 100.0) as i128,
		// 3
		// );

		total_left -= inflation;
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
	for year in 1_u32..=100 {
		let (_, inflation) = compute_total_payout::<Test>(
			MILLISECONDS_PER_YEAR,
			((year - 1) as TsInMs) * MILLISECONDS_PER_YEAR,
			total_left,
			Perquintill::from_percent(0),
		);

		let inflation_rate = inflation * 10_000 / (hard_cap - total_left);

		println!(
			"year {:3}, inflation {:9}, rate {:3}",
			year, inflation, inflation_rate
		);

		total_inflation += inflation;
		total_left = total_left - inflation;
	}

	println!("total inflation: {}", total_inflation);
	println!("total left: {}", total_left);
}
