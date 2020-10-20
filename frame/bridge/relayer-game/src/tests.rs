// --- substrate ---
use frame_support::{assert_err, assert_ok};
// --- darwinia ---
use crate::{
	mock::{mock_relay::*, *},
	*,
};
use darwinia_support::balance::lock::*;

#[test]
fn insufficient_bond_should_fail() {
	ExtBuilder::default()
		.estimate_bond(101)
		.build()
		.execute_with(|| {
			let relay_parcels = MockRelayHeader::gen_continous(1, vec![1, 1], true);

			{
				let poor_man = 0;

				assert_err!(
					RelayerGame::propose(poor_man, relay_parcels[0].clone(), None),
					RelayerGameError::BondIns
				);
			}

			assert_err!(
				RelayerGame::propose(1, relay_parcels[0].clone(), None),
				RelayerGameError::BondIns
			);
			assert_ok!(RelayerGame::propose(
				2,
				MockRelayHeader::gen(2, 0, 1),
				Some(())
			));
			assert_ok!(RelayerGame::propose(3, relay_parcels[0].clone(), Some(())));

			run_to_block(4);

			assert_err!(
				RelayerGame::propose(2, relay_parcels[1].clone(), None),
				RelayerGameError::BondIns
			);
			assert_ok!(RelayerGame::propose(3, relay_parcels[1].clone(), None));
		});
}

#[test]
fn already_confirmed_should_fail() {
	let relay_parcels = MockRelayHeader::gen_continous(1, vec![1, 1, 1, 1, 1], true);

	ExtBuilder::default()
		.headers(relay_parcels.clone())
		.build()
		.execute_with(|| {
			for relay_parcel in relay_parcels {
				assert_err!(
					RelayerGame::propose(1, relay_parcel, None),
					RelayerGameError::RelayParcelAR
				);
			}
		});
}

#[test]
fn duplicate_game_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		let relay_parcel = MockRelayHeader::gen(1, 0, 1);

		assert_ok!(RelayerGame::propose(1, relay_parcel.clone(), None));
		assert_err!(
			RelayerGame::propose(2, relay_parcel, None),
			RelayerGameError::RelayProposalDup
		);
	});
}

// #[test]
// fn jump_round_should_fail() {
// 	ExtBuilder::default().build().execute_with(|| {
// 		let proposal = MockTcHeader::mock_proposal(vec![1, 1, 1, 1, 1], true);

// 		assert_ok!(RelayerGame::submit_proposal(
// 			1,
// 			proposal[..1].to_vec()
// 		));

// 		for i in 2..=5 {
// 			assert_err!(
// 				RelayerGame::submit_proposal(1, proposal[..i].to_vec()),
// 				RelayerGameError::RoundMis
// 			);
// 		}
// 	});
// }

// #[test]
// fn challenge_time_should_work() {
// 	for challenge_time in 3..10 {
// 		ExtBuilder::default()
// 			.challenge_time(challenge_time)
// 			.build()
// 			.execute_with(|| {
// 				let header_thing = MockTcHeader::mock(1, 0, 1);

// 				assert_ok!(RelayerGame::submit_proposal(
// 					1,
// 					vec![header_thing.clone()]
// 				));

// 				for block in 0..=challenge_time {
// 					run_to_block(block);

// 					assert_eq!(RelayerGame::proposals_of_game(header_thing.number).len(), 1);
// 					assert_eq!(Relay::header_of_block_number(header_thing.number), None);
// 				}

// 				run_to_block(challenge_time + 1);

// 				assert_eq!(RelayerGame::proposals_of_game(header_thing.number).len(), 0);
// 				assert_eq!(
// 					Relay::header_of_block_number(header_thing.number),
// 					Some(header_thing)
// 				);
// 			});
// 	}
// }

// #[test]
// fn extend_should_work() {
// 	ExtBuilder::default().build().execute_with(|| {
// 		let proposal_a = MockTcHeader::mock_proposal(vec![1, 1, 1, 1, 1], true);
// 		let proposal_b = MockTcHeader::mock_proposal(vec![1, 1, 1, 1, 1], false);

// 		for i in 1..=5 {
// 			assert_ok!(RelayerGame::submit_proposal(
// 				1,
// 				proposal_a[..i as usize].to_vec()
// 			));
// 			assert_ok!(RelayerGame::submit_proposal(
// 				2,
// 				proposal_b[..i as usize].to_vec()
// 			));

// 			run_to_block(3 * i + 1);
// 		}
// 	});
// }

// #[test]
// fn lock_should_work() {
// 	for estimate_bond in 1..2 {
// 		ExtBuilder::default()
// 			.estimate_bond(estimate_bond)
// 			.build()
// 			.execute_with(|| {
// 				let mut bonds = 0;
// 				let proposal_a = MockTcHeader::mock_proposal(vec![1, 1, 1, 1, 1], true);
// 				let proposal_b = MockTcHeader::mock_proposal(vec![1, 1, 1, 1, 1], false);
// 				let submit_then_assert = |account_id, chain, bonds| {
// 					assert_ok!(RelayerGame::submit_proposal(
// 						account_id, chain
// 					));
// 					assert_eq!(RelayerGame::bonds_of_relayer(account_id), bonds);
// 					assert_eq!(
// 						Ring::locks(account_id),
// 						vec![BalanceLock {
// 							id: RELAYER_GAME_ID,
// 							lock_for: LockFor::Common { amount: bonds },
// 							lock_reasons: LockReasons::All
// 						}]
// 					);
// 				};

// 				for i in 1..=5 {
// 					bonds += estimate_bond;

// 					submit_then_assert(1, proposal_a[..i as usize].to_vec(), bonds);
// 					submit_then_assert(2, proposal_b[..i as usize].to_vec(), bonds);

// 					run_to_block(3 * i + 1);
// 				}

// 				assert_eq!(RelayerGame::bonds_of_relayer(1), 0);
// 				assert!(Ring::locks(1).is_empty());

// 				assert_eq!(RelayerGame::bonds_of_relayer(2), 0);
// 				assert!(Ring::locks(2).is_empty());
// 			});
// 	}
// }

// #[test]
// fn slash_and_reward_should_work() {
// 	for estimate_bond in vec![1, 5, 10, 20, 50, 100] {
// 		ExtBuilder::default()
// 			.estimate_bond(estimate_bond)
// 			.build()
// 			.execute_with(|| {
// 				let proposal_a = MockTcHeader::mock_proposal(vec![1, 1, 1, 1, 1], true);
// 				let proposal_b = MockTcHeader::mock_proposal(vec![1, 1, 1, 1, 1], false);
// 				let mut bonds = estimate_bond;

// 				assert_eq!(Ring::usable_balance(&10), 1000);
// 				assert_eq!(Ring::usable_balance(&20), 2000);

// 				for i in 1..=5 {
// 					assert_ok!(RelayerGame::submit_proposal(
// 						10,
// 						proposal_a[..i as usize].to_vec()
// 					));
// 					assert_ok!(RelayerGame::submit_proposal(
// 						20,
// 						proposal_b[..i as usize].to_vec()
// 					));

// 					run_to_block(3 * i + 1);

// 					bonds += estimate_bond;
// 				}

// 				assert_eq!(Ring::usable_balance(&10), 1000 + bonds);
// 				assert!(Ring::locks(10).is_empty());

// 				assert_eq!(Ring::usable_balance(&20), 2000 - bonds);
// 				assert!(Ring::locks(20).is_empty());
// 			});
// 	}
// }

// #[test]
// fn settle_without_challenge_should_work() {
// 	ExtBuilder::default().build().execute_with(|| {
// 		for (header_thing, i) in MockTcHeader::mock_proposal(vec![1, 1, 1, 1, 1], true)
// 			.into_iter()
// 			.rev()
// 			.zip(1..)
// 		{
// 			assert_ok!(RelayerGame::submit_proposal(
// 				1,
// 				vec![header_thing.clone()]
// 			));

// 			run_to_block(4 * i);

// 			assert_eq!(Ring::usable_balance(&1), 100);
// 			assert!(Ring::locks(1).is_empty());

// 			assert_eq!(
// 				Relay::header_of_block_number(header_thing.number),
// 				Some(header_thing)
// 			);
// 		}
// 	})
// }

// #[test]
// fn settle_with_challenge_should_work() {
// 	ExtBuilder::default().build().execute_with(|| {
// 		let proposal_a = MockTcHeader::mock_proposal(vec![1, 1, 1, 1, 1], true);
// 		let proposal_b = MockTcHeader::mock_proposal(vec![1, 1, 1, 1, 1], false);

// 		for i in 1..=3 {
// 			assert_ok!(RelayerGame::submit_proposal(
// 				1,
// 				proposal_a[..i as usize].to_vec()
// 			));
// 			assert_ok!(RelayerGame::submit_proposal(
// 				2,
// 				proposal_b[..i as usize].to_vec()
// 			));

// 			run_to_block(3 * i + 1);
// 		}

// 		assert_ok!(RelayerGame::submit_proposal(
// 			1,
// 			proposal_a[..4 as usize].to_vec()
// 		));

// 		run_to_block(3 * 4 + 1);

// 		let header_thing = proposal_a[0].clone();

// 		assert_eq!(
// 			Relay::header_of_block_number(header_thing.number),
// 			Some(header_thing)
// 		);
// 	});
// }

// #[test]
// fn settle_abandon_should_work() {
// 	ExtBuilder::default().build().execute_with(|| {
// 		let proposal_a = MockTcHeader::mock_proposal(vec![1, 1, 1, 1, 1], true);
// 		let proposal_b = MockTcHeader::mock_proposal(vec![1, 1, 1, 1, 1], false);

// 		assert_eq!(Ring::usable_balance(&1), 100);
// 		assert_eq!(Ring::usable_balance(&2), 200);

// 		for i in 1..=3 {
// 			assert_ok!(RelayerGame::submit_proposal(
// 				1,
// 				proposal_a[..i as usize].to_vec()
// 			));
// 			assert_ok!(RelayerGame::submit_proposal(
// 				2,
// 				proposal_b[..i as usize].to_vec()
// 			));

// 			run_to_block(3 * i + 1);
// 		}

// 		run_to_block(4 * 3 + 1);

// 		assert_eq!(Ring::usable_balance(&1), 100 - 3);
// 		assert!(Ring::locks(1).is_empty());

// 		assert_eq!(Ring::usable_balance(&2), 200 - 3);
// 		assert!(Ring::locks(2).is_empty());
// 	});
// }

// #[test]
// fn on_chain_arbitrate_should_work() {
// 	ExtBuilder::default().build().execute_with(|| {
// 		let proposal_a = MockTcHeader::mock_proposal(vec![1, 1, 1, 1, 1], true);
// 		let proposal_b = MockTcHeader::mock_proposal(vec![1, 1, 1, 1, 1], false);

// 		for i in 1..=5 {
// 			assert_ok!(RelayerGame::submit_proposal(
// 				1,
// 				proposal_a[..i as usize].to_vec()
// 			));
// 			assert_ok!(RelayerGame::submit_proposal(
// 				2,
// 				proposal_b[..i as usize].to_vec()
// 			));

// 			run_to_block(3 * i + 1);
// 		}

// 		let header_thing = proposal_a[0].clone();

// 		assert_eq!(
// 			Relay::header_of_block_number(header_thing.number),
// 			Some(header_thing)
// 		);
// 	});
// }

// #[test]
// fn no_honesty_should_work() {
// 	ExtBuilder::default().build().execute_with(|| {
// 		let proposal_a = MockTcHeader::mock_proposal(vec![1, 1, 1, 1, 1], false);
// 		let proposal_b = MockTcHeader::mock_proposal(vec![1, 1, 1, 1, 1], false);

// 		assert_eq!(Ring::usable_balance(&1), 100);
// 		assert_eq!(Ring::usable_balance(&2), 200);

// 		for i in 1..=5 {
// 			assert_ok!(RelayerGame::submit_proposal(
// 				1,
// 				proposal_a[..i as usize].to_vec()
// 			));
// 			assert_ok!(RelayerGame::submit_proposal(
// 				2,
// 				proposal_b[..i as usize].to_vec()
// 			));

// 			run_to_block(3 * i + 1);
// 		}

// 		assert_eq!(Ring::usable_balance(&1), 100 - 5);
// 		assert!(Ring::locks(1).is_empty());

// 		assert_eq!(Ring::usable_balance(&2), 200 - 5);
// 		assert!(Ring::locks(2).is_empty());
// 	});
// }

// // TODO: more cases
// #[test]
// fn auto_confirm_period_should_work() {
// 	ExtBuilder::default()
// 		.confirmed_period(3)
// 		.build()
// 		.execute_with(|| {
// 			let header_thing = MockTcHeader::mock(1, 0, 1);

// 			assert_ok!(RelayerGame::submit_proposal(
// 				1,
// 				vec![header_thing.clone()]
// 			));

// 			run_to_block(4);

// 			assert!(Relay::header_of_block_number(header_thing.number).is_none());
// 			assert_eq!(
// 				RelayerGame::pending_headers(),
// 				vec![(6, header_thing.number, header_thing.clone())]
// 			);

// 			run_to_block(6);

// 			assert_eq!(
// 				Relay::header_of_block_number(header_thing.number),
// 				Some(header_thing)
// 			);
// 			assert!(RelayerGame::pending_headers().is_empty());
// 		});
// }

// // TODO: more cases
// #[test]
// fn approve_pending_header_should_work() {
// 	ExtBuilder::default()
// 		.confirmed_period(3)
// 		.build()
// 		.execute_with(|| {
// 			let header_thing = MockTcHeader::mock(1, 0, 1);

// 			assert_ok!(RelayerGame::submit_proposal(
// 				1,
// 				vec![header_thing.clone()],
// 			));

// 			run_to_block(4);

// 			assert!(Relay::header_of_block_number(header_thing.number).is_none());
// 			assert_eq!(
// 				RelayerGame::pending_headers(),
// 				vec![(6, header_thing.number, header_thing.clone())]
// 			);

// 			assert_ok!(
// 				RelayerGame::approve_pending_header(header_thing.number)
// 			);

// 			assert_eq!(
// 				Relay::header_of_block_number(header_thing.number),
// 				Some(header_thing)
// 			);
// 			assert!(RelayerGame::pending_headers().is_empty());
// 		});
// }

// // TODO: more cases
// #[test]
// fn reject_pending_header_should_work() {
// 	ExtBuilder::default()
// 		.confirmed_period(3)
// 		.build()
// 		.execute_with(|| {
// 			let header_thing = MockTcHeader::mock(1, 0, 1);

// 			assert_ok!(RelayerGame::submit_proposal(
// 				1,
// 				vec![header_thing.clone()]
// 			));

// 			run_to_block(4);

// 			assert!(Relay::header_of_block_number(header_thing.number).is_none());
// 			assert_eq!(
// 				RelayerGame::pending_headers(),
// 				vec![(6, header_thing.number, header_thing.clone())]
// 			);

// 			assert_ok!(RelayerGame::reject_pending_header(
// 				header_thing.number
// 			));

// 			assert!(Relay::header_of_block_number(header_thing.number).is_none());
// 			assert!(RelayerGame::pending_headers().is_empty());
// 		});
// }
