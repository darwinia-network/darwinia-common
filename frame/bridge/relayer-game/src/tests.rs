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
			let relay_header_parcels = MockRelayHeader::gen_continous(1, vec![1, 1], true);

			{
				let poor_man = 0;

				assert_err!(
					RelayerGame::affirm(poor_man, relay_header_parcels[0].clone(), None),
					RelayerGameError::BondIns
				);
			}

			assert_err!(
				RelayerGame::affirm(1, relay_header_parcels[0].clone(), None),
				RelayerGameError::BondIns
			);
			assert_ok!(RelayerGame::affirm(
				2,
				MockRelayHeader::gen(2, 0, 1),
				Some(())
			));
			assert_ok!(RelayerGame::dispute_and_affirm(
				3,
				relay_header_parcels[0].clone(),
				Some(())
			));

			run_to_block(4);

			assert_err!(
				RelayerGame::affirm(2, relay_header_parcels[1].clone(), None),
				RelayerGameError::BondIns
			);
			assert_ok!(RelayerGame::affirm(
				3,
				relay_header_parcels[1].clone(),
				None
			));
		});
}

#[test]
fn some_affirm_cases_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		let relay_header_parcel_a = MockRelayHeader::gen(1, 0, 1);
		let relay_header_parcel_b = MockRelayHeader::gen(1, 0, 1);

		assert_err!(
			RelayerGame::dispute_and_affirm(1, relay_header_parcel_a.clone(), None),
			RelayerGameError::NothingToAgainstAffirmationE
		);
		assert_ok!(RelayerGame::affirm(1, relay_header_parcel_a, None));
		assert_err!(
			RelayerGame::affirm(1, relay_header_parcel_b, None),
			RelayerGameError::ExistedAffirmationsFoundC
		);
	});
}

#[test]
fn already_confirmed_should_fail() {
	let relay_header_parcels = MockRelayHeader::gen_continous(1, vec![1, 1, 1, 1, 1], true);

	ExtBuilder::default()
		.headers(relay_header_parcels.clone())
		.build()
		.execute_with(|| {
			for relay_header_parcel in relay_header_parcels {
				assert_err!(
					RelayerGame::affirm(1, relay_header_parcel, None),
					RelayerGameError::RelayParcelAR
				);
			}
		});
}

#[test]
fn duplicate_game_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		let relay_header_parcel = MockRelayHeader::gen(1, 0, 1);

		assert_ok!(RelayerGame::affirm(1, relay_header_parcel.clone(), None));
		assert_err!(
			RelayerGame::dispute_and_affirm(2, relay_header_parcel, None),
			RelayerGameError::RelayAffirmationDup
		);
	});
}

// #[test]
// fn jump_round_should_fail() {
// 	ExtBuilder::default().build().execute_with(|| {
// 		let proposal = MockTcHeader::mock_proposal(vec![1, 1, 1, 1, 1], true);

// 		assert_ok!(RelayerGame::affirm(
// 			1,
// 			proposal[..1].to_vec()
// 		));

// 		for i in 2..=5 {
// 			assert_err!(
// 				RelayerGame::affirm(1, proposal[..i].to_vec()),
// 				RelayerGameError::RoundMis
// 			);
// 		}
// 	});
// }

#[test]
fn challenge_time_should_work() {
	for &challenge_time in [4, 6, 8].iter() {
		ExtBuilder::default()
			.challenge_time(challenge_time)
			.build()
			.execute_with(|| {
				let relay_header_parcel = MockRelayHeader::gen(1, 0, 1);

				assert_ok!(RelayerGame::affirm(1, relay_header_parcel.clone(), None));

				for block in 0..=challenge_time {
					run_to_block(block);

					assert_eq!(
						RelayerGame::affirmations_of_game_at(relay_header_parcel.number, 0).len(),
						1
					);
					assert!(Relay::confirmed_header_of(relay_header_parcel.number).is_none());
				}

				run_to_block(challenge_time + 1);

				assert!(
					RelayerGame::affirmations_of_game_at(relay_header_parcel.number, 1).is_empty()
				);
				assert_eq!(
					Relay::confirmed_header_of(relay_header_parcel.number),
					Some(relay_header_parcel)
				);
			});
	}
}

#[test]
fn extend_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let relay_header_parcels_a = MockRelayHeader::gen_continous(1, vec![1, 1, 1], true);
		let relay_header_parcels_b = MockRelayHeader::gen_continous(1, vec![1, 1, 1], true);

		assert_ok!(RelayerGame::affirm(
			1,
			relay_header_parcels_a[0].clone(),
			Some(())
		));
		assert_ok!(RelayerGame::dispute_and_affirm(
			2,
			relay_header_parcels_b[0].clone(),
			Some(())
		));

		run_to_block(6 * 1 + 1);

		// println_game(3);

		assert_ok!(RelayerGame::extend_affirmation(
			1,
			vec![relay_header_parcels_a[1].clone()],
			RelayAffirmationId {
				relay_header_id: 3,
				round: 0,
				index: 0
			},
			Some(vec![()])
		));
		assert_ok!(RelayerGame::extend_affirmation(
			2,
			vec![relay_header_parcels_b[1].clone()],
			RelayAffirmationId {
				relay_header_id: 3,
				round: 0,
				index: 1
			},
			Some(vec![()])
		));

		run_to_block(6 * 2 + 1);

		assert_ok!(RelayerGame::extend_affirmation(
			1,
			vec![relay_header_parcels_a[2].clone()],
			RelayAffirmationId {
				relay_header_id: 3,
				round: 1,
				index: 0
			},
			Some(vec![()])
		));
		assert_ok!(RelayerGame::extend_affirmation(
			2,
			vec![relay_header_parcels_b[2].clone()],
			RelayAffirmationId {
				relay_header_id: 3,
				round: 1,
				index: 1
			},
			Some(vec![()])
		));
	});
}

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
// 					assert_ok!(RelayerGame::affirm(
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
// 					assert_ok!(RelayerGame::affirm(
// 						10,
// 						proposal_a[..i as usize].to_vec()
// 					));
// 					assert_ok!(RelayerGame::affirm(
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

#[test]
fn settle_without_challenge_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		for (relay_header_parcel, i) in MockRelayHeader::gen_continous(1, vec![1, 1, 1, 1, 1], true)
			.into_iter()
			.rev()
			.zip(1..)
		{
			assert_ok!(RelayerGame::affirm(1, relay_header_parcel.clone(), None));
			assert!(Ring::usable_balance(&1) < 100);
			assert!(!Ring::locks(1).is_empty());

			run_to_block(7 * i);

			assert_eq!(
				Relay::confirmed_header_of(relay_header_parcel.number),
				Some(relay_header_parcel)
			);
			assert_eq!(Ring::usable_balance(&1), 100);
			assert!(Ring::locks(1).is_empty());
		}
	})
}

// #[test]
// fn settle_with_challenge_should_work() {
// 	ExtBuilder::default().build().execute_with(|| {
// 		let proposal_a = MockTcHeader::mock_proposal(vec![1, 1, 1, 1, 1], true);
// 		let proposal_b = MockTcHeader::mock_proposal(vec![1, 1, 1, 1, 1], false);

// 		for i in 1..=3 {
// 			assert_ok!(RelayerGame::affirm(
// 				1,
// 				proposal_a[..i as usize].to_vec()
// 			));
// 			assert_ok!(RelayerGame::affirm(
// 				2,
// 				proposal_b[..i as usize].to_vec()
// 			));

// 			run_to_block(3 * i + 1);
// 		}

// 		assert_ok!(RelayerGame::affirm(
// 			1,
// 			proposal_a[..4 as usize].to_vec()
// 		));

// 		run_to_block(3 * 4 + 1);

// 		let relay_header_parcel = proposal_a[0].clone();

// 		assert_eq!(
// 			Relay::confirmed_header_of(relay_header_parcel.number),
// 			Some(relay_header_parcel)
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
// 			assert_ok!(RelayerGame::affirm(
// 				1,
// 				proposal_a[..i as usize].to_vec()
// 			));
// 			assert_ok!(RelayerGame::affirm(
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
// 			assert_ok!(RelayerGame::affirm(
// 				1,
// 				proposal_a[..i as usize].to_vec()
// 			));
// 			assert_ok!(RelayerGame::affirm(
// 				2,
// 				proposal_b[..i as usize].to_vec()
// 			));

// 			run_to_block(3 * i + 1);
// 		}

// 		let relay_header_parcel = proposal_a[0].clone();

// 		assert_eq!(
// 			Relay::confirmed_header_of(relay_header_parcel.number),
// 			Some(relay_header_parcel)
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
// 			assert_ok!(RelayerGame::affirm(
// 				1,
// 				proposal_a[..i as usize].to_vec()
// 			));
// 			assert_ok!(RelayerGame::affirm(
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

// TODO: more cases
#[test]
fn auto_confirm_period_should_work() {
	ExtBuilder::default()
		.confirmed_period(3)
		.build()
		.execute_with(|| {
			let relay_header_parcel = MockRelayHeader::gen(1, 0, 1);

			assert_ok!(RelayerGame::affirm(1, relay_header_parcel.clone(), None));

			run_to_block(7);

			assert!(Relay::confirmed_header_of(relay_header_parcel.number).is_none());
			assert_eq!(
				RelayerGame::pending_relay_header_parcels(),
				vec![(9, relay_header_parcel.number, relay_header_parcel.clone())]
			);

			run_to_block(9);

			assert_eq!(
				Relay::confirmed_header_of(relay_header_parcel.number),
				Some(relay_header_parcel)
			);
			assert!(RelayerGame::pending_relay_header_parcels().is_empty());
		});
}

// TODO: more cases
#[test]
fn approve_pending_parcels_should_work() {
	ExtBuilder::default()
		.confirmed_period(3)
		.build()
		.execute_with(|| {
			let relay_header_parcel = MockRelayHeader::gen(1, 0, 1);

			assert_ok!(RelayerGame::affirm(1, relay_header_parcel.clone(), None));

			run_to_block(7);

			assert!(Relay::confirmed_header_of(relay_header_parcel.number).is_none());
			assert_eq!(
				RelayerGame::pending_relay_header_parcels(),
				vec![(9, relay_header_parcel.number, relay_header_parcel.clone())]
			);

			assert_ok!(RelayerGame::approve_pending_relay_header_parcel(
				relay_header_parcel.number
			));

			assert_eq!(
				Relay::confirmed_header_of(relay_header_parcel.number),
				Some(relay_header_parcel)
			);
			assert!(RelayerGame::pending_relay_header_parcels().is_empty());
		});
}

// TODO: more cases
#[test]
fn reject_pending_parcels_should_work() {
	ExtBuilder::default()
		.confirmed_period(3)
		.build()
		.execute_with(|| {
			let relay_header_parcel = MockRelayHeader::gen(1, 0, 1);

			assert_ok!(RelayerGame::affirm(1, relay_header_parcel.clone(), None));

			run_to_block(7);

			assert!(Relay::confirmed_header_of(relay_header_parcel.number).is_none());
			assert_eq!(
				RelayerGame::pending_relay_header_parcels(),
				vec![(9, relay_header_parcel.number, relay_header_parcel.clone())]
			);

			assert_ok!(RelayerGame::reject_pending_relay_header_parcel(
				relay_header_parcel.number
			));

			assert!(Relay::confirmed_header_of(relay_header_parcel.number).is_none());
			assert!(RelayerGame::pending_relay_header_parcels().is_empty());
		});
}
