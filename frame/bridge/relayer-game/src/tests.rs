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

			run_to_block(3 * i + 1);
		}
	});
}

#[test]
fn lock_should_work() {
	for estimate_bond in 1..2 {
		ExtBuilder::default()
			.estimate_bond(estimate_bond)
			.build()
			.execute_with(|| {
				let mut bonds = 0;
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

				for i in 1..=5 {
					bonds += estimate_bond;

					submit_then_assert(1, chain_a[..i as usize].to_vec(), bonds);
					submit_then_assert(2, chain_b[..i as usize].to_vec(), bonds);

					run_to_block(3 * i + 1);
				}

				assert_eq!(RelayerGame::bonds_of_relayer(1), 0);
				assert!(Ring::locks(1).is_empty());

				assert_eq!(RelayerGame::bonds_of_relayer(2), 0);
				assert!(Ring::locks(2).is_empty());
			});
	}
}

#[test]
fn slash_and_reward_should_work() {
	for estimate_bond in vec![1, 5, 10, 20, 50, 100] {
		ExtBuilder::default()
			.estimate_bond(estimate_bond)
			.build()
			.execute_with(|| {
				let chain_a = MockTcHeader::mock_raw_chain(vec![1, 1, 1, 1, 1], true);
				let chain_b = MockTcHeader::mock_raw_chain(vec![1, 1, 1, 1, 1], false);
				let mut bonds = estimate_bond;

				assert_eq!(Ring::usable_balance(&10), 1000);
				assert_eq!(Ring::usable_balance(&20), 2000);

				for i in 1..=5 {
					assert_ok!(RelayerGame::submit_proposal(
						Origin::signed(10),
						chain_a[..i as usize].to_vec()
					));
					assert_ok!(RelayerGame::submit_proposal(
						Origin::signed(20),
						chain_b[..i as usize].to_vec()
					));

					run_to_block(3 * i + 1);

					bonds += estimate_bond;
				}

				assert_eq!(Ring::usable_balance(&10), 1000 + bonds);
				assert!(Ring::locks(10).is_empty());

				assert_eq!(Ring::usable_balance(&20), 2000 - bonds);
				assert!(Ring::locks(20).is_empty());
			});
	}
}

#[test]
fn settle_without_challenge_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		for (header, i) in MockTcHeader::mock_chain(vec![1, 1, 1, 1, 1], true)
			.into_iter()
			.rev()
			.zip(1..)
		{
			assert_ok!(RelayerGame::submit_proposal(
				Origin::signed(1),
				vec![header.encode()]
			));

			run_to_block(4 * i);

			assert_eq!(Ring::usable_balance(&1), 100);
			assert!(Ring::locks(1).is_empty());

			assert_eq!(Relay::header_of_block_number(header.number), Some(header));
		}
	})
}

#[test]
fn settle_with_challenge_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let chain_a = MockTcHeader::mock_raw_chain(vec![1, 1, 1, 1, 1], true);
		let chain_b = MockTcHeader::mock_raw_chain(vec![1, 1, 1, 1, 1], false);

		for i in 1..=3 {
			assert_ok!(RelayerGame::submit_proposal(
				Origin::signed(1),
				chain_a[..i as usize].to_vec()
			));
			assert_ok!(RelayerGame::submit_proposal(
				Origin::signed(2),
				chain_b[..i as usize].to_vec()
			));

			run_to_block(3 * i + 1);
		}

		assert_ok!(RelayerGame::submit_proposal(
			Origin::signed(1),
			chain_a[..4 as usize].to_vec()
		));

		run_to_block(3 * 4 + 1);

		let header: MockTcHeader = Decode::decode(&mut &*chain_a[0]).unwrap();

		assert_eq!(Relay::header_of_block_number(header.number), Some(header));
	});
}

#[test]
fn settle_abandon_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let chain_a = MockTcHeader::mock_raw_chain(vec![1, 1, 1, 1, 1], true);
		let chain_b = MockTcHeader::mock_raw_chain(vec![1, 1, 1, 1, 1], false);

		assert_eq!(Ring::usable_balance(&1), 100);
		assert_eq!(Ring::usable_balance(&2), 200);

		for i in 1..=3 {
			assert_ok!(RelayerGame::submit_proposal(
				Origin::signed(1),
				chain_a[..i as usize].to_vec()
			));
			assert_ok!(RelayerGame::submit_proposal(
				Origin::signed(2),
				chain_b[..i as usize].to_vec()
			));

			run_to_block(3 * i + 1);
		}

		run_to_block(4 * 3 + 1);

		assert_eq!(Ring::usable_balance(&1), 100 - 3);
		assert!(Ring::locks(1).is_empty());

		assert_eq!(Ring::usable_balance(&2), 200 - 3);
		assert!(Ring::locks(2).is_empty());
	});
}

#[test]
fn on_chain_arbitrate_should_work() {
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

			run_to_block(3 * i + 1);
		}

		let header: MockTcHeader = Decode::decode(&mut &*chain_a[0]).unwrap();

		assert_eq!(Relay::header_of_block_number(header.number), Some(header));
	});
}

#[test]
fn no_honesty_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let chain_a = MockTcHeader::mock_raw_chain(vec![1, 1, 1, 1, 1], false);
		let chain_b = MockTcHeader::mock_raw_chain(vec![1, 1, 1, 1, 1], false);

		assert_eq!(Ring::usable_balance(&1), 100);
		assert_eq!(Ring::usable_balance(&2), 200);

		for i in 1..=5 {
			assert_ok!(RelayerGame::submit_proposal(
				Origin::signed(1),
				chain_a[..i as usize].to_vec()
			));
			assert_ok!(RelayerGame::submit_proposal(
				Origin::signed(2),
				chain_b[..i as usize].to_vec()
			));

			run_to_block(3 * i + 1);
		}

		assert_eq!(Ring::usable_balance(&1), 100 - 5);
		assert!(Ring::locks(1).is_empty());

		assert_eq!(Ring::usable_balance(&2), 200 - 5);
		assert!(Ring::locks(2).is_empty());
	});
}

// TODO: more cases
#[test]
fn auto_confirm_period_should_work() {
	ExtBuilder::default()
		.confirmed_period(3)
		.build()
		.execute_with(|| {
			let header = MockTcHeader::mock(1, 0, 1);

			assert_ok!(RelayerGame::submit_proposal(
				Origin::signed(1),
				vec![header.encode()]
			));

			run_to_block(4);

			assert!(Relay::header_of_block_number(header.number).is_none());
			assert_eq!(
				RelayerGame::pending_headers(),
				vec![(6, header.number, header.encode())]
			);

			run_to_block(6);

			assert_eq!(Relay::header_of_block_number(header.number), Some(header));
			assert!(RelayerGame::pending_headers().is_empty());
		});
}
