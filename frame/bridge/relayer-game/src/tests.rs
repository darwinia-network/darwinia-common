// --- substrate ---
use frame_support::{assert_err, assert_ok};
// --- darwinia ---
use crate::mock::{mock_relay::MockTcHeader, *};

#[test]
fn empty_proposal_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		assert_err!(
			RelayerGame::submit_proposal(Origin::signed(1), vec![]),
			RelayerGameError::ProposalI
		);
	});
}

#[test]
fn duplicate_game_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		let chain = vec![MockTcHeader::new_raw(1, true)];

		assert_ok!(RelayerGame::submit_proposal(
			Origin::signed(1),
			chain.clone()
		));
		assert_err!(
			RelayerGame::submit_proposal(Origin::signed(1), chain),
			RelayerGameError::ProposalAE
		);
	});
}
