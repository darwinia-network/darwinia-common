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
		let proposal_chain = vec![MockTcHeader::new_raw(1, true)];

		assert_ok!(RelayerGame::submit_proposal(
			Origin::signed(1),
			proposal_chain.clone()
		));
		assert_err!(
			RelayerGame::submit_proposal(Origin::signed(1), proposal_chain),
			RelayerGameError::ProposalAE
		);
	});
}

#[test]
fn jump_round_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		let mut proposal_chain = vec![MockTcHeader::new_raw(1, true)];

		assert_ok!(RelayerGame::submit_proposal(
			Origin::signed(1),
			proposal_chain.clone()
		));

		for i in 2..5 {
			proposal_chain.push(MockTcHeader::new_raw(i, true));

			assert_err!(
				RelayerGame::submit_proposal(Origin::signed(1), proposal_chain.clone()),
				RelayerGameError::RoundMis
			);
		}
	});
}
