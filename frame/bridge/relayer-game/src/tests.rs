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
			let chain = MockTcHeader::mock_raw_chain(vec![1, 1], true);

			assert_err!(
				RelayerGame::submit_proposal(Origin::signed(1), chain[..1].to_vec()),
				RelayerGameError::InsufficientBond
			);
			assert_ok!(RelayerGame::submit_proposal(
				Origin::signed(2),
				vec![MockTcHeader::mock_raw(2, 0, 1)]
			));
			assert_ok!(RelayerGame::submit_proposal(
				Origin::signed(3),
				chain[..1].to_vec()
			));

			run_to_block(4);

			assert_err!(
				RelayerGame::submit_proposal(Origin::signed(2), chain.clone()),
				RelayerGameError::InsufficientBond
			);
			assert_ok!(RelayerGame::submit_proposal(Origin::signed(3), chain));
		});
}

#[test]
fn already_confirmed_should_fail() {
	let confirmed_headers = MockTcHeader::mock_chain(vec![1, 1, 1, 1, 1], true);

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
		let chain = MockTcHeader::mock_raw_chain(vec![1], true);

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

#[test]
fn jump_round_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		let chain = MockTcHeader::mock_raw_chain(vec![1, 1, 1, 1, 1], true);

		assert_ok!(RelayerGame::submit_proposal(
			Origin::signed(1),
			chain[..1].to_vec()
		));

		for i in 2..=5 {
			assert_err!(
				RelayerGame::submit_proposal(Origin::signed(1), chain[..i].to_vec()),
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
				let header = MockTcHeader::mock(1, 0, 1);

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
fn extend_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let chain_a = MockTcHeader::mock_raw_chain(vec![1, 1, 1, 1, 1], true);
		let chain_b = MockTcHeader::mock_raw_chain(vec![1, 1, 1, 1, 1], false);

		for i in 1..=5 {
			assert_ok!(RelayerGame::submit_proposal(
				Origin::signed(1),
				chain_a[..i as usize].to_vec()
			));
			assert_ok!(RelayerGame::submit_proposal(
				Origin::signed(2),
				chain_b[..i as usize].to_vec()
			));

			run_to_block(4 * i);
		}
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
				let chain_a = MockTcHeader::mock_raw_chain(vec![1, 1, 1, 1, 1], true);
				let chain_b = MockTcHeader::mock_raw_chain(vec![1, 1, 1, 1, 1], false);
				let submit_then_assert = |account_id, chain, bonds| {
					assert_ok!(RelayerGame::submit_proposal(
						Origin::signed(account_id),
						chain
					));
					assert_eq!(RelayerGame::bonds_of_relayer(account_id), bonds);
					assert_eq!(
						Ring::locks(account_id),
						vec![BalanceLock {
							id: RELAYER_GAME_ID,
							lock_for: LockFor::Common { amount: bonds },
							lock_reasons: LockReasons::All
						}]
					);
				};

				submit_then_assert(1, chain_a[..1].to_vec(), bonds);
				submit_then_assert(2, chain_b[..1].to_vec(), bonds);

				run_to_block(4);

				for i in 2..=5 {
					bonds += estimate_bond;

					submit_then_assert(1, chain_a[..i as usize].to_vec(), bonds);
					submit_then_assert(2, chain_b[..i as usize].to_vec(), bonds);

					run_to_block(4 * i);
				}

				run_to_block(4 * 5);

				assert_eq!(RelayerGame::bonds_of_relayer(1), 0);
				assert_eq!(Ring::locks(1), vec![]);

				assert_eq!(RelayerGame::bonds_of_relayer(2), 0);
				assert_eq!(Ring::locks(2), vec![]);
			});
	}
}

// #[test]
// fn reward_should_work() {
// 	for estimate_bond in 1..5 {
// 		ExtBuilder::default()
// 			.estimate_bond(estimate_bond)
// 			.build()
// 			.execute_with(|| {
// 				let mut bonds = estimate_bond;

// 				let mut proposal_chain_a = vec![MockTcHeader::new_raw(5, 0)];
// 				let mut proposal_chain_b = vec![MockTcHeader::new_raw(5, 2)];

// 				assert_ok!(RelayerGame::submit_proposal(
// 					Origin::signed(1),
// 					proposal_chain_a.clone()
// 				));
// 				assert_ok!(RelayerGame::submit_proposal(
// 					Origin::signed(2),
// 					proposal_chain_b.clone()
// 				));

// 				for (block_number, closed_at) in (2..5).rev().zip((1..).map(|n| 4 * n)) {
// 					run_to_block(closed_at);

// 					bonds += estimate_bond;

// 					proposal_chain_a.push(MockTcHeader::new_raw(block_number, 0));
// 					proposal_chain_b.push(MockTcHeader::new_raw(block_number, 2));

// 					assert_ok!(RelayerGame::submit_proposal(
// 						Origin::signed(1),
// 						proposal_chain_a.clone()
// 					));
// 					assert_ok!(RelayerGame::submit_proposal(
// 						Origin::signed(2),
// 						proposal_chain_b.clone()
// 					));
// 				}
// 			});
// 	}
// }

// #[test]
// fn settle_without_challenge_should_work() {
// 	ExtBuilder::default().build().execute_with(|| {
// 		let chain = MockTcHeader::mock_raw_chain(vec![1, 1, 1, 1, 1], true);

// 		for i in 0..5 {
// 			assert_ok!(RelayerGame::submit_proposal(
// 				Origin::signed(1),
// 				chain[..1].to_vec()
// 			));

// 			run_to_block(4 * i);

// 			assert_eq!(Relay::header_of_block_number(block_number), Some(header));
// 		}
// 	})
// }

// #[test]
// fn settle_with_challenge_should_work() {}

// #[test]
// fn on_chain_arbitrate_should_work() {}

// #[test]
// fn handle_give_up_should_work() {}
