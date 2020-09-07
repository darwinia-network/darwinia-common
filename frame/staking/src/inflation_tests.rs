use crate::{mock::*, *};

#[test]
fn compute_total_payout_should_work_correctly() {
	const MILLISECONDS_PER_YEAR: TsInMs = ((36525 * 24 * 60 * 60) / 100) * 1000;

	let initial_issuance = 200_000;
	let hard_cap = 1_000_000;

	// year, inflated tokens, inflation rate
	let inflation_spec = [
		(1, 8000 , 4.00),
		(2, 11177, 5.37),
		(3, 13474, 6.15),
		(4, 15270, 6.56),
		(5, 16713, 6.74),
		(6, 17882, 6.76),
		(7, 18826, 6.66),
	];

	let mut total_left: RingBalance<Test> = hard_cap - initial_issuance;
	for spec_item in inflation_spec.iter() {
		let year = spec_item.0;
		let expected_inflated = spec_item.1;
		let expected_inflation_rate = spec_item.2;
		// println!("{} - {}", year, total_left);

		let era_duration = MILLISECONDS_PER_YEAR;
		let living_time = (year - 1) * MILLISECONDS_PER_YEAR;
		let payout_fraction = Perbill::from_percent(0);

		let (payout, inflated) = inflation::compute_total_payout::<Test>(era_duration, living_time, total_left, payout_fraction);

		assert_eq!(payout, 0);
		assert_eq!(inflated, expected_inflated);
		assert_eq!(inflated*100/total_left, (expected_inflation_rate*100.0) as u128);

		total_left = total_left - inflated;
	}
}

// #[test]
// fn compute_total_payout_with_payout_fraction_should_work_corrently() {

// }


