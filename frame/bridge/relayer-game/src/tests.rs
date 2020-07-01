// --- substrate ---
use frame_support::{assert_err, assert_ok};
// --- darwinia ---
use crate::mock::*;

#[test]
fn empty_proposal_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		assert_err!(
			RelayerGame::submit_proposal(Origin::signed(1), vec![]),
			RelayerGameError::ProposalI
		);
	});
}
