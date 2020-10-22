// --- substrate ---
use frame_support::{assert_err, assert_ok};
// --- darwinia ---
use crate::{
	mock::{mock_relay::*, BlockNumber, *},
	*,
};
use darwinia_support::balance::lock::*;

#[test]
fn insufficient_bond_should_fail() {
	ExtBuilder::default()
		.estimate_stake(101)
		.build()
		.execute_with(|| {
			let relay_header_parcels = MockRelayHeader::gen_continous(1, vec![1, 1], true);

			{
				let poor_man = 0;

				assert_err!(
					RelayerGame::affirm(poor_man, relay_header_parcels[0].clone(), None),
					RelayerGameError::StakeIns
				);
			}

			assert_err!(
				RelayerGame::affirm(1, relay_header_parcels[0].clone(), None),
				RelayerGameError::StakeIns
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
				RelayerGameError::StakeIns
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
			let relayer = 1;

			for relay_header_parcel in relay_header_parcels {
				assert_err!(
					RelayerGame::affirm(relayer, relay_header_parcel, None),
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
		let realyer_a = 1;
		let realyer_b = 2;
		let relay_header_parcels_a = MockRelayHeader::gen_continous(1, vec![1, 1, 1, 1, 1], true);
		let relay_header_parcels_b = MockRelayHeader::gen_continous(1, vec![1, 1, 1, 1, 1], true);
		let relay_header_id = relay_header_parcels_a.len() as _;
		let round_count = relay_header_parcels_a.len() as _;

		assert_ok!(RelayerGame::affirm(
			realyer_a,
			relay_header_parcels_a[0].clone(),
			Some(())
		));
		assert_ok!(RelayerGame::dispute_and_affirm(
			realyer_b,
			relay_header_parcels_b[0].clone(),
			Some(())
		));

		// println_game(3);

		for round in 1..round_count {
			run_to_block(6 * round as BlockNumber + 1);

			assert_ok!(RelayerGame::extend_affirmation(
				realyer_a,
				vec![relay_header_parcels_a[round as usize].clone()],
				RelayAffirmationId {
					relay_header_id,
					round: round - 1,
					index: 0
				},
				Some(vec![()])
			));
			assert_ok!(RelayerGame::extend_affirmation(
				realyer_b,
				vec![relay_header_parcels_b[round as usize].clone()],
				RelayAffirmationId {
					relay_header_id,
					round: round - 1,
					index: 1
				},
				Some(vec![()])
			));
		}
	});
}

#[test]
fn lock_should_work() {
	for estimate_stake in 1..=3 {
		ExtBuilder::default()
			.estimate_stake(estimate_stake)
			.build()
			.execute_with(|| {
				let relayer_a = 1;
				let relayer_b = 2;
				let relay_header_parcels_a =
					MockRelayHeader::gen_continous(1, vec![1, 1, 1, 1, 1], true);
				let relay_header_parcels_b =
					MockRelayHeader::gen_continous(1, vec![1, 1, 1, 1, 1], false);
				let relay_header_id = relay_header_parcels_a.len() as _;
				let round_count = relay_header_parcels_a.len() as _;
				let submit_then_assert = |relayer, relay_parcel, round, index, stakes| {
					assert_ok!(RelayerGame::extend_affirmation(
						relayer,
						vec![relay_parcel],
						RelayAffirmationId {
							relay_header_id,
							round,
							index,
						},
						Some(vec![()])
					));
					assert_eq!(RelayerGame::stakes_of(relayer), stakes);
					assert_eq!(
						Ring::locks(relayer),
						vec![BalanceLock {
							id: RELAYER_GAME_ID,
							lock_for: LockFor::Common { amount: stakes },
							lock_reasons: LockReasons::All
						}]
					);
				};

				assert_ok!(RelayerGame::affirm(
					relayer_a,
					relay_header_parcels_a[0].clone(),
					Some(())
				));
				assert_ok!(RelayerGame::dispute_and_affirm(
					relayer_b,
					relay_header_parcels_b[0].clone(),
					Some(())
				));

				run_to_block(7);

				let mut stakes = estimate_stake;

				for round in 1..round_count {
					stakes += estimate_stake;

					submit_then_assert(
						relayer_a,
						relay_header_parcels_a[round as usize].clone(),
						round - 1,
						0,
						stakes,
					);
					submit_then_assert(
						relayer_b,
						relay_header_parcels_b[round as usize].clone(),
						round - 1,
						1,
						stakes,
					);

					run_to_block(6 * (round as BlockNumber + 1) + 1);
				}

				assert_eq!(RelayerGame::stakes_of(relayer_a), 0);
				assert!(Ring::locks(1).is_empty());

				assert_eq!(RelayerGame::stakes_of(relayer_b), 0);
				assert!(Ring::locks(2).is_empty());
			});
	}
}

#[test]
fn slash_and_reward_should_work() {
	for estimate_stake in vec![1, 5, 10, 20, 50, 100] {
		ExtBuilder::default()
			.estimate_stake(estimate_stake)
			.build()
			.execute_with(|| {
				let relayer_a = 10;
				let relayer_b = 20;
				let relay_header_parcels_a =
					MockRelayHeader::gen_continous(1, vec![1, 1, 1, 1, 1], true);
				let relay_header_parcels_b =
					MockRelayHeader::gen_continous(1, vec![1, 1, 1, 1, 1], false);
				let relay_header_id = relay_header_parcels_a.len() as _;
				let round_count = relay_header_parcels_a.len() as _;
				let mut stakes = estimate_stake;

				assert_eq!(Ring::usable_balance(&relayer_a), 1000);
				assert_eq!(Ring::usable_balance(&relayer_b), 2000);

				assert_ok!(RelayerGame::affirm(
					relayer_a,
					relay_header_parcels_a[0].clone(),
					Some(())
				));
				assert_ok!(RelayerGame::dispute_and_affirm(
					relayer_b,
					relay_header_parcels_b[0].clone(),
					Some(())
				));

				run_to_block(7);

				for round in 1..round_count {
					assert_ok!(RelayerGame::extend_affirmation(
						relayer_a,
						vec![relay_header_parcels_a[round as usize].clone()],
						RelayAffirmationId {
							relay_header_id,
							round: round - 1,
							index: 0
						},
						Some(vec![()])
					));
					assert_ok!(RelayerGame::extend_affirmation(
						relayer_b,
						vec![relay_header_parcels_b[round as usize].clone()],
						RelayAffirmationId {
							relay_header_id,
							round: round - 1,
							index: 1
						},
						Some(vec![()])
					));

					run_to_block(6 * (round as BlockNumber + 1) + 1);

					stakes += estimate_stake;
				}

				assert_eq!(Ring::usable_balance(&relayer_a), 1000 + stakes);
				assert!(Ring::locks(relayer_a).is_empty());

				assert_eq!(Ring::usable_balance(&relayer_b), 2000 - stakes);
				assert!(Ring::locks(relayer_b).is_empty());
			});
	}
}

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

#[test]
fn no_honesty_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let relayer_a = 1;
		let relayer_b = 2;
		let relay_header_parcels_a = MockRelayHeader::gen_continous(1, vec![1, 1, 1, 1, 1], false);
		let relay_header_parcels_b = MockRelayHeader::gen_continous(1, vec![1, 1, 1, 1, 1], false);
		let relay_header_id = relay_header_parcels_a.len() as _;
		let round_count = relay_header_parcels_a.len() as _;

		assert_eq!(Ring::usable_balance(&relayer_a), 100);
		assert_eq!(Ring::usable_balance(&relayer_b), 200);

		assert_ok!(RelayerGame::affirm(
			relayer_a,
			relay_header_parcels_a[0].clone(),
			Some(())
		));
		assert_ok!(RelayerGame::dispute_and_affirm(
			relayer_b,
			relay_header_parcels_b[0].clone(),
			Some(())
		));

		run_to_block(7);

		for round in 1..round_count {
			assert_ok!(RelayerGame::extend_affirmation(
				relayer_a,
				vec![relay_header_parcels_a[round as usize].clone()],
				RelayAffirmationId {
					relay_header_id,
					round: round - 1,
					index: 0
				},
				Some(vec![()])
			));
			assert_ok!(RelayerGame::extend_affirmation(
				relayer_b,
				vec![relay_header_parcels_b[round as usize].clone()],
				RelayAffirmationId {
					relay_header_id,
					round: round - 1,
					index: 1
				},
				Some(vec![()])
			));

			run_to_block(6 * (round as BlockNumber + 1) + 1);
		}

		assert_eq!(Ring::usable_balance(&relayer_a), 100 - 5);
		assert!(Ring::locks(relayer_a).is_empty());

		assert_eq!(Ring::usable_balance(&relayer_b), 200 - 5);
		assert!(Ring::locks(relayer_b).is_empty());
	});
}

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
