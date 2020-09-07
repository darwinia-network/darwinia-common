use crate::{mock::*, *};

#[test]
fn compute_total_payout_should_work_correctly() {
	const MILLISECONDS_PER_YEAR: TsInMs = ((36525 * 24 * 60 * 60) / 100) * 1000;

	let initial_issuance = 200_000;
	let hard_cap = 1_000_000;

	// year, inflated tokens, inflation rate, payout_fraction
	let inflation_spec = [
		(1, 8000 , 4.00, 0),
		(2, 11177, 5.37, 35),
		(3, 13474, 6.15, 50),
		(4, 15270, 6.56, 77),
		(5, 16713, 6.74, 33),
		(6, 17882, 6.76, 81),
		(7, 18826, 6.66, 100),
	];

	let mut total_left: RingBalance<Test> = hard_cap - initial_issuance;
	for spec_item in inflation_spec.iter() {
		let year = spec_item.0;
		let expected_inflated = spec_item.1;
		let expected_inflation_rate = spec_item.2;
		let expected_payout_fraction = spec_item.3;
		println!("{} - {}", year, total_left);

		let era_duration = MILLISECONDS_PER_YEAR;
		let living_time = (year - 1) * MILLISECONDS_PER_YEAR;
		let payout_fraction = Perbill::from_percent(expected_payout_fraction);

		let (payout, inflated) = inflation::compute_total_payout::<Test>(era_duration, living_time, total_left, payout_fraction);

		assert_eq!(payout * 100, expected_inflated * expected_payout_fraction as u128);
		assert_eq!(inflated, expected_inflated);
		println!("{}", inflated);
		assert_eq!(inflated * 100 / (hard_cap - total_left), expected_inflation_rate as u128);

		total_left = total_left - inflated;
	}
}
