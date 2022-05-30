// --- paritytech ---
use frame_support::{assert_err, assert_ok, traits::Currency, WeakBoundedVec};
// --- darwinia-network ---
use crate::{mock::*, Event, *};
use darwinia_support::balance::*;

#[test]
fn bond_zero_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		gen_paired_account!(s(123), c(456));
		assert_err!(
			Staking::bond(
				Origin::signed(s),
				c,
				StakingBalance::RingBalance(0),
				RewardDestination::Stash,
				0,
			),
			StakingError::InsufficientBond
		);

		gen_paired_account!(s(234), c(567));
		assert_err!(
			Staking::bond(
				Origin::signed(s),
				c,
				StakingBalance::KtonBalance(0),
				RewardDestination::Stash,
				0,
			),
			StakingError::InsufficientBond
		);
	});
}

#[test]
fn normal_unbond_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let (stash, controller) = (11, 10);
		let value = 200 * COIN;
		let promise_month: u64 = 12;
		let _ = Ring::deposit_creating(&stash, 1000 * COIN);
		let start = System::block_number();

		{
			let mut ledger = Staking::ledger(controller).unwrap();

			assert_ok!(Staking::bond_extra(
				Origin::signed(stash),
				StakingBalance::RingBalance(value),
				promise_month as u8,
			));

			ledger.active += value;
			ledger.active_deposit_ring += value;
			ledger.deposit_items.push(TimeDepositItem {
				value,
				start_time: INIT_TIMESTAMP,
				expire_time: INIT_TIMESTAMP + promise_month * MONTH_IN_MILLISECONDS,
			});

			assert_eq!(Staking::ledger(controller).unwrap(), ledger);
		}

		{
			let kton_free_balance = Kton::free_balance(&stash);
			let mut ledger = Staking::ledger(controller).unwrap();

			assert_ok!(Staking::bond_extra(
				Origin::signed(stash),
				StakingBalance::KtonBalance(COIN),
				0,
			));
			ledger.active_kton += kton_free_balance;
			assert_eq!(Staking::ledger(controller).unwrap(), ledger);

			assert_ok!(Staking::unbond(
				Origin::signed(controller),
				StakingBalance::KtonBalance(kton_free_balance)
			));

			ledger.active_kton = 0;
			ledger
				.kton_staking_lock
				.unbondings
				.try_push(Unbonding {
					amount: kton_free_balance,
					until: BondingDurationInBlockNumber::get() + start,
				})
				.unwrap();

			assert_eq!(Staking::ledger(controller).unwrap(), ledger);
		}
	});
}

#[test]
fn check_stash_already_bonded_and_controller_already_paired() {
	ExtBuilder::default().build().execute_with(|| {
		gen_paired_account!(unpaired_stash(123), unpaired_controller(456));

		assert_err!(
			Staking::bond(
				Origin::signed(11),
				unpaired_controller,
				StakingBalance::RingBalance(COIN),
				RewardDestination::Stash,
				0,
			),
			StakingError::AlreadyBonded
		);
		assert_err!(
			Staking::bond(
				Origin::signed(unpaired_stash),
				10,
				StakingBalance::RingBalance(COIN),
				RewardDestination::Stash,
				0,
			),
			StakingError::AlreadyPaired
		);
	});
}

#[test]
fn bond_kton_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		{
			let (stash, controller) = (1001, 1000);
			let _ = Kton::deposit_creating(&stash, 10 * COIN);

			assert_ok!(Staking::bond(
				Origin::signed(stash),
				controller,
				StakingBalance::KtonBalance(10 * COIN),
				RewardDestination::Stash,
				0,
			));
			assert_eq!(
				Staking::ledger(controller).unwrap(),
				StakingLedger {
					stash,
					active_kton: 10 * COIN,
					kton_staking_lock: StakingLock { staking_amount: 0, ..Default::default() },
					..Default::default()
				}
			);
			assert_eq!(
				Kton::locks(&stash),
				vec![BalanceLock { id: STAKING_ID, amount: 10 * COIN, reasons: Reasons::All }]
			);
		}

		{
			let (stash, controller) = (2001, 2000);
			// promise_month should not work for kton
			let _ = Kton::deposit_creating(&stash, 10 * COIN);

			assert_ok!(Staking::bond(
				Origin::signed(stash),
				controller,
				StakingBalance::KtonBalance(10 * COIN),
				RewardDestination::Stash,
				12,
			));
			assert_eq!(
				Staking::ledger(controller).unwrap(),
				StakingLedger {
					stash,
					active_kton: 10 * COIN,
					kton_staking_lock: StakingLock { staking_amount: 0, ..Default::default() },
					..Default::default()
				}
			);
		}
	});
}

#[test]
fn unbond_over_max_unbondings_chunks_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		gen_paired_account!(stash(123), controller(456));
		assert_ok!(Staking::bond(
			Origin::signed(stash),
			controller,
			StakingBalance::RingBalance(COIN),
			RewardDestination::Stash,
			0,
		));

		for ts in 0..MAX_UNLOCKING_CHUNKS {
			Timestamp::set_timestamp(ts as u64);

			assert_ok!(Staking::unbond(Origin::signed(controller), StakingBalance::RingBalance(1)));
		}

		assert_err!(
			Staking::unbond(Origin::signed(controller), StakingBalance::RingBalance(1)),
			StakingError::NoMoreChunks
		);
	});
}

#[test]
fn unbond_zero() {
	ExtBuilder::default().build().execute_with(|| {
		gen_paired_account!(stash(123), controller(456), promise_month(12));
		let ledger = Staking::ledger(controller).unwrap();

		Timestamp::set_timestamp(promise_month * MONTH_IN_MILLISECONDS);
		assert_ok!(Staking::unbond(Origin::signed(10), StakingBalance::RingBalance(0)));
		assert_ok!(Staking::unbond(Origin::signed(10), StakingBalance::KtonBalance(0)));
		assert_eq!(Staking::ledger(controller).unwrap(), ledger);
	});
}

// bond 10_000 Ring for 12 months, gain 1 Kton
// bond extra 10_000 Ring for 36 months, gain 3 Kton
// bond extra 1 Kton
// nominate
// unlock the 12 months deposit item with punish
// lost 3 Kton and 10_000 Ring's power for nominate
#[test]
fn two_different_bond_then_unbond_specific_one() {
	ExtBuilder::default().build().execute_with(|| {
		let (stash, controller) = (777, 888);
		let _ = Ring::deposit_creating(&stash, 20_000);

		// Earn 1 Kton with bond 10_000 Ring 12 months
		assert_ok!(Staking::bond(
			Origin::signed(stash),
			controller,
			StakingBalance::RingBalance(10_000),
			RewardDestination::Stash,
			12,
		));

		// Earn 3 Kton with bond 10_000 Ring 36 months
		assert_ok!(Staking::bond_extra(
			Origin::signed(stash),
			StakingBalance::RingBalance(10_000),
			36,
		));

		assert_eq!(Kton::free_balance(&stash), 4);

		// Bond 1 Kton
		assert_ok!(Staking::bond_extra(Origin::signed(stash), StakingBalance::KtonBalance(1), 36));
		assert_eq!(Staking::ledger(controller).unwrap().active_kton, 1);

		// Become a nominator
		assert_ok!(Staking::nominate(Origin::signed(controller), vec![controller]));

		// Then unbond the the first 12 months part,
		// this behavior should be punished 3 times Kton according to the remaining times
		// 3 times * 1 Kton * 12 months(remaining) / 12 months(promised)
		assert_ok!(Staking::try_claim_deposits_with_punish(
			Origin::signed(controller),
			INIT_TIMESTAMP + 12 * MONTH_IN_MILLISECONDS,
		));
		assert_eq!(Kton::free_balance(&stash), 1);

		let ledger = Staking::ledger(controller).unwrap();

		// Please Note:
		// not enough Kton to unbond, but the function will not fail
		assert_ok!(Staking::try_claim_deposits_with_punish(
			Origin::signed(controller),
			INIT_TIMESTAMP + 36 * MONTH_IN_MILLISECONDS,
		));
		assert_eq!(Staking::ledger(controller).unwrap(), ledger);
	});
}

#[test]
fn rebond_event_should_work() {
	ExtBuilder::default().nominate(false).build().execute_with(|| {
		assert_ok!(Staking::set_payee(Origin::signed(10), RewardDestination::Controller));

		let _ = Ring::make_free_balance_be(&11, 1000000);

		run_to_block(5);

		assert_eq!(
			Staking::ledger(&10),
			Some(StakingLedger {
				stash: 11,
				active: 1000,
				ring_staking_lock: StakingLock {
					staking_amount: 0,
					unbondings: WeakBoundedVec::force_from(vec![], None)
				},
				..Default::default()
			})
		);

		run_to_block(6);

		Staking::unbond(Origin::signed(10), StakingBalance::RingBalance(400)).unwrap();
		assert_eq!(
			Staking::ledger(&10),
			Some(StakingLedger {
				stash: 11,
				active: 600,
				ring_staking_lock: StakingLock {
					staking_amount: 0,
					unbondings: WeakBoundedVec::force_from(
						vec![Unbonding { amount: 400, until: 6 + bonding_duration_in_blocks() }],
						None
					)
				},
				..Default::default()
			})
		);

		System::reset_events();

		// Re-bond half of the unbonding funds.
		Staking::rebond(Origin::signed(10), 200, 0).unwrap();
		assert_eq!(
			Staking::ledger(&10),
			Some(StakingLedger {
				stash: 11,
				active: 800,
				ring_staking_lock: StakingLock {
					staking_amount: 0,
					unbondings: WeakBoundedVec::force_from(
						vec![Unbonding {
							amount: 200,
							until: 6 + BondingDurationInBlockNumber::get(),
						}],
						None
					)
				},
				..Default::default()
			})
		);
		System::assert_has_event(Event::RingBonded(11, 200, 36000, 36000).into());
	});
}

#[test]
fn withdraw_unbonded_should_work() {
	ExtBuilder::default().nominate(false).existential_deposit(0).build().execute_with(|| {
		let _ = Ring::make_free_balance_be(&100, 100);

		Staking::bond(
			Origin::signed(100),
			100,
			StakingBalance::RingBalance(100),
			RewardDestination::Stash,
			0,
		)
		.unwrap();
		Staking::unbond(Origin::signed(100), StakingBalance::RingBalance(100)).unwrap();

		run_to_block(60);
		assert_ok!(Staking::withdraw_unbonded(Origin::signed(100), 0));
		// Reaped.
		assert!(Staking::ledger(&100).is_none());
		assert!(Ring::locks(&100).is_empty());
	});
}
