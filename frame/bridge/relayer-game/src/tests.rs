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
		let proposal_chain = vec![MockTcHeader::new_raw(1, 0)];

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
		let mut proposal_chain = vec![MockTcHeader::new_raw(5, 0)];

		assert_ok!(RelayerGame::submit_proposal(
			Origin::signed(1),
			proposal_chain.clone()
		));

		for i in (2..5).rev() {
			proposal_chain.push(MockTcHeader::new_raw(i, 0));

			assert_err!(
				RelayerGame::submit_proposal(Origin::signed(1), proposal_chain.clone()),
				RelayerGameError::RoundMis
			);
		}
	});
}

#[test]
fn extend_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let mut proposal_chain_a = vec![MockTcHeader::new_raw(5, 0)];
		let mut proposal_chain_b = vec![MockTcHeader::new_raw(5, 2)];

		assert_ok!(RelayerGame::submit_proposal(
			Origin::signed(1),
			proposal_chain_a.clone()
		));
		assert_ok!(RelayerGame::submit_proposal(
			Origin::signed(2),
			proposal_chain_b.clone()
		));

		for i in (4..5).rev() {
			run_to_block((i - 1) * 3 + 1);

			proposal_chain_a.push(MockTcHeader::new_raw(i, 0));
			proposal_chain_b.push(MockTcHeader::new_raw(i, 2));

			assert_ok!(RelayerGame::submit_proposal(
				Origin::signed(1),
				proposal_chain_a.clone()
			));
			assert_ok!(RelayerGame::submit_proposal(
				Origin::signed(2),
				proposal_chain_b.clone()
			));
		}
	});
}
