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
		(1_u32, 80_000_000 as Balance, 4_f64, 0_u32),
		(2, 111773288, 5.37, 35),
		(3, 134746988, 6.15, 50),
		(4, 152702246, 6.56, 77),
		(5, 167131170, 6.74, 33),
		(6, 178823310, 6.76, 81),
		(7, 188269290, 6.66, 100),
		(8, 195807997, 6.5, 50),
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
		(20, 197889214, 3.6, 50),
		(21, 193757077, 3.4, 50),
		(22, 189288266, 3.21, 50),
		(23, 184533528, 3.04, 50),
		(24, 179539840, 2.87, 50),
		(25, 174350616, 2.71, 50),
		(26, 169005896, 2.55, 50),
		(27, 163542518, 2.41, 50),
		(28, 157994277, 2.27, 50),
		(29, 152392074, 2.14, 50),
		(30, 146764063, 2.02, 50),
		(31, 141135789, 1.91, 50),
		(32, 135530328, 1.8, 50),
		(33, 129968412, 1.69, 50),
		(34, 124468569, 1.59, 50),
		(35, 119047241, 1.5, 50),
		(36, 113718914, 1.41, 50),
		(37, 108496241, 1.33, 50),
		(38, 103390154, 1.25, 50),
		(39, 98409989, 1.17, 50),
		(40, 93563589, 1.1, 50),
		(41, 88857423, 1.04, 50),
		(42, 84296681, 0.97, 50),
		(43, 79885384, 0.91, 50),
		(44, 75626477, 0.86, 50),
		(45, 71521925, 0.8, 50),
		(46, 67572798, 0.75, 50),
		(47, 63779362, 0.71, 50),
		(48, 60141154, 0.66, 50),
		(49, 56657063, 0.62, 50),
		(50, 53325399, 0.58, 50),
		(51, 50143961, 0.54, 50),
		(52, 47110102, 0.51, 50),
		(53, 44220788, 0.47, 50),
		(54, 41472651, 0.44, 50),
		(55, 38862044, 0.41, 50),
		(56, 36385085, 0.38, 50),
		(57, 34037703, 0.36, 50),
		(58, 31815678, 0.33, 50),
		(59, 29714675, 0.31, 50),
		(60, 27730280, 0.29, 50),
		(61, 25858031, 0.27, 50),
		(62, 24093441, 0.25, 50),
		(63, 22432029, 0.23, 50),
		(64, 20869334, 0.21, 50),
		(65, 19400941, 0.2, 50),
		(66, 18022494, 0.18, 50),
		(67, 16729713, 0.17, 50),
		(68, 15518405, 0.16, 50),
		(69, 14384476, 0.15, 50),
		(70, 13323940, 0.14, 50),
		(71, 12332925, 0.13, 50),
		(72, 11407683, 0.12, 50),
		(73, 10544590, 0.11, 50),
		(74, 9740152, 0.1, 50),
		(75, 8991009, 0.09, 50),
		(76, 8293933, 0.08, 50),
		(77, 7645831, 0.08, 50),
		(78, 7043743, 0.07, 50),
		(79, 6484843, 0.07, 50),
		(80, 5966438, 0.06, 50),
		(81, 5485962, 0.06, 50),
		(82, 5040980, 0.05, 50),
		(83, 4629177, 0.05, 50),
		(84, 4248362, 0.04, 50),
		(85, 3896461, 0.04, 50),
		(86, 3571514, 0.04, 50),
		(87, 3271672, 0.03, 50),
		(88, 2995190, 0.03, 50),
		(89, 2740428, 0.03, 50),
		(90, 2505842, 0.03, 50),
		(91, 2289982, 0.02, 50),
		(92, 2091488, 0.02, 50),
		(93, 1909086, 0.02, 50),
		(94, 1741584, 0.02, 50),
		(95, 1587864, 0.02, 50),
		(96, 1446887, 0.01, 50),
		(97, 1317678, 0.01, 50),
		(98, 1199332, 0.01, 50),
		(99, 1091004, 0.01, 50),
		(100, 991910, 0.01, 50),
	];
	let mut total_left: RingBalance<Test> = hard_cap - initial_issuance;

	for &(year, exp_inflation, exp_inflation_rate, payout_fraction) in inflation_spec.iter() {
		let payout_fraction = Perbill::from_percent(payout_fraction);
		let (payout, inflation) = compute_total_payout::<Test>(
			MILLISECONDS_PER_YEAR,
			((year - 1) as TsInMs) * MILLISECONDS_PER_YEAR,
			total_left,
			payout_fraction,
		);

		// eprintln!("{}\n{}\n", inflation, exp_inflation);

		assert_eq!(payout, payout_fraction * inflation);
		assert_eq_error_rate!(inflation, exp_inflation, if inflation == 0 { 0 } else { 3 });
		assert_eq_error_rate!(
			inflation as f64 / (hard_cap - total_left) as f64,
			exp_inflation_rate / 100.00_f64,
			0.01_f64 / 100.00_f64
		);

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
			Perbill::from_percent(0),
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
