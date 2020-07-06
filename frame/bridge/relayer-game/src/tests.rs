// --- crates ---
use codec::Encode;
// --- substrate ---
use frame_support::{assert_err, assert_ok};
// --- darwinia ---
use crate::{
	mock::{mock_relay::*, *},
	*,
};

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
fn insufficient_bond_should_fail() {
	ExtBuilder::default()
		.estimate_bond(100)
		.build()
		.execute_with(|| {
			let mut proposal_chain = vec![MockTcHeader::new_raw(5, 0)];

			assert_err!(
				RelayerGame::submit_proposal(Origin::signed(1), proposal_chain.clone()),
				RelayerGameError::InsufficientBond
			);
			assert_ok!(RelayerGame::submit_proposal(
				Origin::signed(2),
				vec![MockTcHeader::new_raw(5, 2)]
			));
			assert_ok!(RelayerGame::submit_proposal(
				Origin::signed(3),
				proposal_chain.clone()
			));

			run_to_block(4);

			proposal_chain.push(MockTcHeader::new_raw(4, 0));

			assert_err!(
				RelayerGame::submit_proposal(Origin::signed(2), proposal_chain.clone()),
				RelayerGameError::InsufficientBond
			);
			assert_ok!(RelayerGame::submit_proposal(
				Origin::signed(3),
				proposal_chain.clone()
			));
		});
}

#[test]
fn already_confirmed_should_fail() {
	let mut confirmed_headers = vec![];

	for block_number in 5..10 {
		confirmed_headers.push(MockTcHeader::new(block_number, 0));
	}

	ExtBuilder::default()
		.headers(confirmed_headers.clone())
		.build()
		.execute_with(|| {
			for confirmed_header in confirmed_headers {
				assert_err!(
					RelayerGame::submit_proposal(
						Origin::signed(1),
						vec![confirmed_header.encode()]
					),
					RelayerGameError::TargetHeaderAC
				);
			}
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
fn lock_should_work() {
	for estimate_bond in 1..5 {
		ExtBuilder::default()
			.estimate_bond(estimate_bond)
			.build()
			.execute_with(|| {
				let mut bonds = estimate_bond;

				let mut proposal_chain_a = vec![MockTcHeader::new_raw(5, 0)];
				let mut proposal_chain_b = vec![MockTcHeader::new_raw(5, 2)];

				assert_ok!(RelayerGame::submit_proposal(
					Origin::signed(1),
					proposal_chain_a.clone()
				));
				assert_eq!(RelayerGame::bonds_of_relayer(1), bonds);
				assert_eq!(
					Ring::locks(1),
					vec![BalanceLock {
						id: RELAYER_GAME_ID,
						lock_for: LockFor::Common { amount: bonds },
						lock_reasons: LockReasons::All
					}]
				);

				assert_ok!(RelayerGame::submit_proposal(
					Origin::signed(2),
					proposal_chain_b.clone()
				));
				assert_eq!(RelayerGame::bonds_of_relayer(2), bonds);
				assert_eq!(
					Ring::locks(2),
					vec![BalanceLock {
						id: RELAYER_GAME_ID,
						lock_for: LockFor::Common { amount: bonds },
						lock_reasons: LockReasons::All
					}]
				);

				for (block_number, closed_at) in (2..5).rev().zip((1..).map(|n| 4 * n)) {
					run_to_block(closed_at);

					bonds += estimate_bond;

					proposal_chain_a.push(MockTcHeader::new_raw(block_number, 0));
					proposal_chain_b.push(MockTcHeader::new_raw(block_number, 2));

					assert_ok!(RelayerGame::submit_proposal(
						Origin::signed(1),
						proposal_chain_a.clone()
					));
					assert_eq!(RelayerGame::bonds_of_relayer(1), bonds);
					assert_eq!(
						Ring::locks(1),
						vec![BalanceLock {
							id: RELAYER_GAME_ID,
							lock_for: LockFor::Common { amount: bonds },
							lock_reasons: LockReasons::All
						}]
					);

					assert_ok!(RelayerGame::submit_proposal(
						Origin::signed(2),
						proposal_chain_b.clone()
					));
					assert_eq!(RelayerGame::bonds_of_relayer(2), bonds);
					assert_eq!(
						Ring::locks(2),
						vec![BalanceLock {
							id: RELAYER_GAME_ID,
							lock_for: LockFor::Common { amount: bonds },
							lock_reasons: LockReasons::All
						}]
					);
				}
			});
	}
}

#[test]
fn jump_round_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		let mut proposal_chain = vec![MockTcHeader::new_raw(5, 0)];

		assert_ok!(RelayerGame::submit_proposal(
			Origin::signed(1),
			proposal_chain.clone()
		));

		for block_number in (2..5).rev() {
			proposal_chain.push(MockTcHeader::new_raw(block_number, 0));

			assert_err!(
				RelayerGame::submit_proposal(Origin::signed(1), proposal_chain.clone()),
				RelayerGameError::RoundMis
			);
		}
	});
}

#[test]
fn challenge_time_should_work() {
	for challenge_time in 3..10 {
		ExtBuilder::default()
			.challenge_time(challenge_time)
			.build()
			.execute_with(|| {
				let header = MockTcHeader::new(1, 0);

				assert_ok!(RelayerGame::submit_proposal(
					Origin::signed(1),
					vec![header.encode()]
				));

				for block in 0..=challenge_time {
					run_to_block(block);

					assert_eq!(RelayerGame::proposals_of_game(header.number).len(), 1);
					assert_eq!(Relay::header_of_block_number(header.number), None);
				}

				run_to_block(challenge_time + 1);

				assert_eq!(RelayerGame::proposals_of_game(header.number).len(), 0);
				assert_eq!(Relay::header_of_block_number(header.number), Some(header));
			});
	}
}

#[test]
fn no_challenge_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		for (block_number, closed_at) in (1..10).rev().zip((1..).map(|n| 4 * n)) {
			let header = MockTcHeader::new(block_number, 0);

			assert_ok!(RelayerGame::submit_proposal(
				Origin::signed(1),
				vec![header.encode()]
			));

			run_to_block(closed_at);

			assert_eq!(Relay::header_of_block_number(block_number), Some(header));
		}
	})
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

		for (block_number, closed_at) in (2..5).rev().zip((1..).map(|n| 4 * n)) {
			run_to_block(closed_at);

			proposal_chain_a.push(MockTcHeader::new_raw(block_number, 0));
			proposal_chain_b.push(MockTcHeader::new_raw(block_number, 2));

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
