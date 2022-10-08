// --- paritytech ---
use frame_support::{assert_ok, traits::Currency, WeakBoundedVec};
// --- darwinia-network ---
use crate::{mock::*, Event, *};
use darwinia_balances::{BalanceLock, Reasons};
use darwinia_support::balance::*;

#[test]
fn deposit_zero_should_do_nothing() {
	ExtBuilder::default().build().execute_with(|| {
		let (stash, controller) = (1001, 1000);
		let _ = Ring::deposit_creating(&stash, COIN);

		assert_ok!(Staking::bond(
			Origin::signed(stash),
			controller,
			StakingBalance::RingBalance(COIN),
			RewardDestination::Stash,
			0,
		));

		for m in 0..=36 {
			// NO-OP
			assert_ok!(Staking::deposit_extra(Origin::signed(stash), 0, m));
		}

		assert!(Staking::ledger(&controller).unwrap().deposit_items.is_empty());

		// Deposit succeeded.
		assert_ok!(Staking::deposit_extra(Origin::signed(stash), COIN, 1));
		assert_eq!(Staking::ledger(&controller).unwrap().deposit_items.len(), 1);

		// NO-OP
		assert_ok!(Staking::deposit_extra(Origin::signed(stash), COIN, 1));
		assert_eq!(Staking::ledger(&controller).unwrap().deposit_items.len(), 1);
	});
}

#[test]
fn deposit_extra_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let (stash, controller) = (1001, 1000);
		let _ = Ring::deposit_creating(&stash, COIN);

		assert_ok!(Staking::bond(
			Origin::signed(stash),
			controller,
			StakingBalance::RingBalance(COIN),
			RewardDestination::Stash,
			0,
		));

		let kton_free_balance = Kton::free_balance(&stash);
		let mut ledger = Staking::ledger(controller).unwrap();

		assert_ok!(Staking::deposit_extra(Origin::signed(stash), COIN, 12));

		ledger.active_deposit_ring += COIN;
		ledger.deposit_items.push(TimeDepositItem {
			value: COIN,
			start_time: INIT_TIMESTAMP,
			expire_time: INIT_TIMESTAMP + 12 * MONTH_IN_MILLISECONDS,
		});

		assert_eq!(Staking::ledger(controller).unwrap(), ledger);
		assert_eq!(Kton::free_balance(&stash), kton_free_balance + (COIN / 10000));
	});
}

#[test]
fn deposit_extra_should_not_touch_existed_items() {
	ExtBuilder::default().build().execute_with(|| {
		gen_paired_account!(stash(123), controller(456), 0);

		let promise_month = 12;
		let expired_items_len = 3;
		let expiry_timestamp = INIT_TIMESTAMP + promise_month * MONTH_IN_MILLISECONDS;

		assert_ok!(Staking::bond_extra(
			Origin::signed(stash),
			StakingBalance::RingBalance(5 * COIN),
			0,
		));

		for _ in 0..expired_items_len {
			assert_ok!(Staking::deposit_extra(Origin::signed(stash), COIN, promise_month as u8));
		}

		Timestamp::set_timestamp(expiry_timestamp);

		assert_ok!(Staking::deposit_extra(Origin::signed(stash), 2 * COIN, promise_month as u8,));
		assert_eq!(Staking::ledger(controller).unwrap().deposit_items.len(), expired_items_len + 1,);
	});
}

#[test]
fn claim_deposits_with_punish_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let (stash, controller) = (1001, 1000);
		let promise_month = 36;
		let bond_value = 10;
		let _ = Ring::deposit_creating(&stash, 1000);
		let mut ledger = StakingLedger {
			stash,
			active: bond_value,
			active_deposit_ring: bond_value,
			deposit_items: vec![TimeDepositItem {
				value: bond_value,
				start_time: INIT_TIMESTAMP,
				expire_time: INIT_TIMESTAMP + promise_month * MONTH_IN_MILLISECONDS,
			}],
			ring_staking_lock: StakingLock { staking_amount: 0, ..Default::default() },
			..Default::default()
		};

		assert_ok!(Staking::bond(
			Origin::signed(stash),
			controller,
			StakingBalance::RingBalance(bond_value),
			RewardDestination::Stash,
			promise_month as u8,
		));
		assert_eq!(Staking::ledger(controller).unwrap(), ledger);
		// Kton is 0, skip `unbond_with_punish`.
		assert_ok!(Staking::try_claim_deposits_with_punish(
			Origin::signed(controller),
			INIT_TIMESTAMP + promise_month * MONTH_IN_MILLISECONDS,
		));
		assert_eq!(Staking::ledger(controller).unwrap(), ledger);

		// Set more kton balance to make it work.
		let _ = Kton::deposit_creating(&stash, COIN);

		assert_ok!(Staking::try_claim_deposits_with_punish(
			Origin::signed(controller),
			INIT_TIMESTAMP + promise_month * MONTH_IN_MILLISECONDS,
		));

		ledger.active_deposit_ring -= bond_value;
		ledger.deposit_items.clear();

		assert_eq!(Staking::ledger(controller).unwrap(), ledger);
		assert_eq!(Kton::free_balance(&stash), COIN - 3);
	});

	// punished value for unbond deposit claim after a duration should correct
	ExtBuilder::default().build().execute_with(|| {
		let (stash, controller) = (1001, 1000);
		let promise_month = 36;
		let bond_value = 10 * COIN;
		let deposit_item_expire_time = INIT_TIMESTAMP + promise_month * MONTH_IN_MILLISECONDS;
		let _ = Ring::deposit_creating(&stash, 1000 * COIN);
		let mut ledger = StakingLedger {
			stash,
			active: bond_value,
			active_deposit_ring: bond_value,
			deposit_items: vec![TimeDepositItem {
				value: bond_value,
				start_time: INIT_TIMESTAMP,
				expire_time: deposit_item_expire_time,
			}],
			ring_staking_lock: StakingLock { staking_amount: 0, ..Default::default() },
			..Default::default()
		};

		assert_ok!(Staking::bond(
			Origin::signed(stash),
			controller,
			StakingBalance::RingBalance(bond_value),
			RewardDestination::Stash,
			promise_month as u8,
		));
		assert_eq!(Staking::ledger(controller).unwrap(), ledger);

		Timestamp::set_timestamp(Timestamp::now() + 14 * MONTH_IN_MILLISECONDS);

		assert_ok!(Staking::try_claim_deposits_with_punish(
			Origin::signed(controller),
			deposit_item_expire_time,
		));
		assert_eq!(Staking::ledger(controller).unwrap(), ledger);

		let _ = Kton::deposit_creating(&stash, COIN);
		let free_kton = Kton::free_balance(&stash);

		assert_ok!(Staking::try_claim_deposits_with_punish(
			Origin::signed(controller),
			deposit_item_expire_time,
		));

		let slashed: KtonBalance<Test> = inflation::compute_kton_reward::<Test>(bond_value, 36)
			- inflation::compute_kton_reward::<Test>(bond_value, 14);

		System::assert_has_event(
			Event::DepositsClaimedWithPunish(ledger.stash.clone(), slashed * 3).into(),
		);

		ledger.active_deposit_ring -= bond_value;
		ledger.deposit_items.clear();

		assert_eq!(Staking::ledger(controller).unwrap(), ledger);
		assert_eq!(Kton::free_balance(&stash), free_kton - slashed * 3);
	});
}
