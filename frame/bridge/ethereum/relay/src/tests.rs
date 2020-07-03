use crate::mock::*;

#[test]
fn verify_receipt_proof() {
	ExtBuilder::default().build().execute_with(|| {
		let r = EthHeaderRaw::from_file("./src/test-data/3.json");
	})
}
