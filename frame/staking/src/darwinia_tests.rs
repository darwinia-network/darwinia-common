// This file is part of Darwinia.
//
// Copyright (C) 2018-2022 Darwinia Network
// SPDX-License-Identifier: GPL-3.0
//
// Darwinia is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Darwinia is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

//! Tests for the module.

// --- paritytech ---
use frame_support::{assert_err, assert_ok, traits::Currency, WeakBoundedVec};
use sp_runtime::{traits::Zero, Perbill};
use sp_staking::offence::*;
use substrate_test_utils::assert_eq_uvec;
// --- darwinia-network ---
use crate::{mock::*, Event, *};
use darwinia_support::{balance::*, traits::OnDepositRedeem};

/// gen_paired_account!(a(1), b(2), m(12));
/// will create stash `a` and controller `b`
/// `a` has 100 Ring and 100 Kton
/// promise for `m` month with 50 Ring and 50 Kton
///
/// `m` can be ignore, this won't create variable `m`
/// ```rust
/// gen_paired_account!(a(1), b(2), 12);
/// ```
///
/// `m(12)` can be ignore, and it won't perform `bond` action
/// ```rust
/// gen_paired_account!(a(1), b(2));
/// ```
macro_rules! gen_paired_account {
	($stash:ident($stash_id:expr), $controller:ident($controller_id:expr), $promise_month:ident($how_long:expr)) => {
		#[allow(non_snake_case, unused)]
		let $stash = $stash_id;
		let _ = Ring::deposit_creating(&$stash, 100 * COIN);
		let _ = Kton::deposit_creating(&$stash, 100 * COIN);
		#[allow(non_snake_case, unused)]
		let $controller = $controller_id;
		let _ = Ring::deposit_creating(&$controller, COIN);
		#[allow(non_snake_case, unused)]
		let $promise_month = $how_long;
		assert_ok!(Staking::bond(
			Origin::signed($stash),
			$controller,
			StakingBalance::RingBalance(50 * COIN),
			RewardDestination::Stash,
			$how_long,
		));
		assert_ok!(Staking::bond_extra(
			Origin::signed($stash),
			StakingBalance::KtonBalance(50 * COIN),
			$how_long
		));
	};
	($stash:ident($stash_id:expr), $controller:ident($controller_id:expr), $how_long:expr) => {
		#[allow(non_snake_case, unused)]
		let $stash = $stash_id;
		let _ = Ring::deposit_creating(&$stash, 100 * COIN);
		let _ = Kton::deposit_creating(&$stash, 100 * COIN);
		#[allow(non_snake_case, unused)]
		let $controller = $controller_id;
		let _ = Ring::deposit_creating(&$controller, COIN);
		assert_ok!(Staking::bond(
			Origin::signed($stash),
			$controller,
			StakingBalance::RingBalance(50 * COIN),
			RewardDestination::Stash,
			$how_long,
		));
		assert_ok!(Staking::bond_extra(
			Origin::signed($stash),
			StakingBalance::KtonBalance(50 * COIN),
			$how_long,
		));
	};
	($stash:ident($stash_id:expr), $controller:ident($controller_id:expr)) => {
		#[allow(non_snake_case, unused)]
		let $stash = $stash_id;
		let _ = Ring::deposit_creating(&$stash, 100 * COIN);
		let _ = Kton::deposit_creating(&$stash, 100 * COIN);
		#[allow(non_snake_case, unused)]
		let $controller = $controller_id;
		let _ = Ring::deposit_creating(&$controller, COIN);
	};
}

#[test]
fn slash_ledger_should_work() {
	ExtBuilder::default()
		.nominate(false)
		.validator_count(1)
		.build()
		.execute_with(|| {
			start_active_era(0);

			assert_eq_uvec!(validator_controllers(), vec![20]);

			let (account_id, bond) = (777, COIN);
			let _ = Ring::deposit_creating(&account_id, bond);

			assert_ok!(Staking::bond(
				Origin::signed(account_id),
				account_id,
				StakingBalance::RingBalance(bond),
				RewardDestination::Controller,
				0,
			));
			assert_ok!(Staking::deposit_extra(
				Origin::signed(account_id),
				COIN * 80 / 100,
				36
			));
			assert_ok!(Staking::validate(
				Origin::signed(account_id),
				ValidatorPrefs::default()
			));

			start_active_era(1);

			assert_eq_uvec!(validator_controllers(), vec![777]);

			on_offence_now(
				&[OffenceDetails {
					offender: (account_id, Staking::eras_stakers(active_era(), account_id)),
					reporters: vec![],
				}],
				&[Perbill::from_percent(90)],
			);

			{
				let total = bond;
				let normal = total * (100 - 80) / 100;
				let deposit = total * 80 / 100;

				assert!(normal + deposit == total);
				let total_slashed = bond * 90 / 100;

				assert!(total_slashed > normal);
				let normal_slashed = normal;
				let deposit_slashed = total_slashed - normal_slashed;

				assert_eq!(
					Staking::ledger(&account_id).unwrap(),
					StakingLedger {
						stash: account_id,
						active: total - total_slashed,
						active_deposit_ring: deposit - deposit_slashed,
						deposit_items: vec![TimeDepositItem {
							value: deposit - deposit_slashed,
							start_time: 30000,
							expire_time: 93312030000,
						}],
						ring_staking_lock: StakingLock {
							staking_amount: deposit - deposit_slashed,
							..Default::default()
						},
						..Default::default()
					},
				);
			}

			let ledger = Staking::ledger(&account_id).unwrap();

			// Should not overflow here
			assert_ok!(Staking::unbond(
				Origin::signed(account_id),
				StakingBalance::RingBalance(1)
			));

			assert_eq!(ledger, Staking::ledger(&account_id).unwrap());
		});
}

#[test]
fn kton_should_reward_even_does_not_own_kton_before() {
	// Tests that validator storage items are cleaned up when stash is empty
	// Tests that storage items are untouched when controller is empty
	ExtBuilder::default()
		.has_stakers(false)
		.build()
		.execute_with(|| {
			let account_id = 777;
			let _ = Ring::deposit_creating(&account_id, 10000);

			assert!(Kton::free_balance(&account_id).is_zero());
			assert_ok!(Staking::bond(
				Origin::signed(account_id),
				account_id,
				StakingBalance::RingBalance(10000),
				RewardDestination::Stash,
				36,
			));
			assert_eq!(Kton::free_balance(&account_id), 3);
		});
}

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
fn normal_kton_should_work() {
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
					kton_staking_lock: StakingLock {
						staking_amount: 10 * COIN,
						..Default::default()
					},
					..Default::default()
				}
			);
			assert_eq!(
				Kton::locks(&stash),
				vec![BalanceLock {
					id: STAKING_ID,
					lock_for: LockFor::Staking(StakingLock {
						staking_amount: 10 * COIN,
						..Default::default()
					}),
					lock_reasons: LockReasons::All
				}]
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
					kton_staking_lock: StakingLock {
						staking_amount: 10 * COIN,
						..Default::default()
					},
					..Default::default()
				}
			);
		}
	});
}

#[test]
fn time_deposit_ring_unbond_and_withdraw_automatically_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let (stash, controller) = (11, 10);

		let start = System::block_number();
		let unbond_value = 10;

		// unbond 10 for the first time
		assert_ok!(Staking::unbond(
			Origin::signed(controller),
			StakingBalance::RingBalance(unbond_value),
		));

		// check the lock
		assert_eq!(
			Ring::locks(stash),
			WeakBoundedVec::force_from(
				vec![BalanceLock {
					id: STAKING_ID,
					lock_for: LockFor::Staking(StakingLock {
						staking_amount: 1000 - unbond_value,
						unbondings: WeakBoundedVec::force_from(
							vec![Unbonding {
								amount: unbond_value,
								until: BondingDurationInBlockNumber::get() + start,
							}],
							None,
						),
					}),
					lock_reasons: LockReasons::All,
				}],
				None
			),
		);

		// check the ledger
		assert_eq!(
			Staking::ledger(controller).unwrap(),
			StakingLedger {
				stash,
				active: 1000 - unbond_value,
				active_deposit_ring: 0,
				active_kton: 0,
				deposit_items: vec![],
				ring_staking_lock: StakingLock {
					staking_amount: 1000 - unbond_value,
					unbondings: WeakBoundedVec::force_from(
						vec![Unbonding {
							amount: unbond_value,
							until: BondingDurationInBlockNumber::get() + start,
						}],
						None
					),
				},
				kton_staking_lock: Default::default(),
				claimed_rewards: vec![]
			},
		);

		let unbond_start = BondingDurationInBlockNumber::get() + start - 1;
		System::set_block_number(unbond_start);

		// unbond for the second time
		assert_ok!(Staking::unbond(
			Origin::signed(controller),
			StakingBalance::RingBalance(90)
		));

		// check the locks
		assert_eq!(
			Ring::locks(stash),
			vec![BalanceLock {
				id: STAKING_ID,
				lock_for: LockFor::Staking(StakingLock {
					staking_amount: 900,
					unbondings: WeakBoundedVec::force_from(
						vec![
							Unbonding {
								amount: unbond_value,
								until: BondingDurationInBlockNumber::get() + start,
							},
							Unbonding {
								amount: 90,
								until: BondingDurationInBlockNumber::get() + unbond_start,
							},
						],
						None
					),
				}),
				lock_reasons: LockReasons::All,
			}],
		);

		// We can't transfer current now.
		assert_err!(
			Ring::transfer(Origin::signed(stash), controller, 1),
			RingError::LiquidityRestrictions
		);

		let unbond_start_2 = BondingDurationInBlockNumber::get() + unbond_start + 1;
		System::set_block_number(unbond_start_2);

		// stash account can transfer again!
		assert_ok!(Ring::transfer(Origin::signed(stash), controller, 1));

		assert_eq!(
			Ring::locks(stash),
			vec![BalanceLock {
				id: STAKING_ID,
				lock_for: LockFor::Staking(StakingLock {
					staking_amount: 900,
					unbondings: WeakBoundedVec::force_from(
						vec![
							Unbonding {
								amount: unbond_value,
								until: BondingDurationInBlockNumber::get() + start,
							},
							Unbonding {
								amount: 90,
								until: BondingDurationInBlockNumber::get() + unbond_start,
							},
						],
						None
					),
				}),
				lock_reasons: LockReasons::All,
			}],
		);

		// Unbond all of it. Must be chilled first.
		assert_ok!(Staking::chill(Origin::signed(controller)));
		assert_ok!(Staking::unbond(
			Origin::signed(controller),
			StakingBalance::RingBalance(COIN)
		));

		assert_eq!(Ring::locks(&stash).len(), 1);

		System::set_block_number(BondingDurationInBlockNumber::get() + unbond_start_2 + 1);
		// Trigger the update lock.
		assert_ok!(Staking::unbond(
			Origin::signed(controller),
			StakingBalance::RingBalance(10)
		));

		// TODO: clean dust ledger
		// check the ledger, it will be empty because we have
		// just unbonded all balances, the ledger is drained.
		// assert!(Staking::ledger(controller).is_none());

		// check the ledger
		assert_eq!(
			Staking::ledger(controller).unwrap(),
			StakingLedger {
				stash,
				..Default::default()
			},
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
			ledger.ring_staking_lock.staking_amount += value;
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
			ledger.kton_staking_lock.staking_amount += kton_free_balance;
			assert_eq!(Staking::ledger(controller).unwrap(), ledger);

			assert_ok!(Staking::unbond(
				Origin::signed(controller),
				StakingBalance::KtonBalance(kton_free_balance)
			));
			ledger.active_kton = 0;
			ledger.kton_staking_lock.staking_amount = 0;
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
fn punished_claim_should_work() {
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
			ring_staking_lock: StakingLock {
				staking_amount: bond_value,
				..Default::default()
			},
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

	// slash value for unbond deposit claim after a duration should correct
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
			ring_staking_lock: StakingLock {
				staking_amount: bond_value,
				..Default::default()
			},
			..Default::default()
		};

		// will emit Event::RingBonded
		assert_ok!(Staking::bond(
			Origin::signed(stash),
			controller,
			StakingBalance::RingBalance(bond_value),
			RewardDestination::Stash,
			promise_month as u8,
		));
		assert_eq!(Staking::ledger(controller).unwrap(), ledger);

		// set a fake blockchain time to simulate elapsed time
		Timestamp::set_timestamp(Timestamp::now() + 14 * MONTH_IN_MILLISECONDS);
		assert_ok!(Staking::try_claim_deposits_with_punish(
			Origin::signed(controller),
			deposit_item_expire_time,
		));
		// ledger no change cause no kton for punishment
		assert_eq!(Staking::ledger(controller).unwrap(), ledger);

		// Set more kton balance to make it work.
		let _ = Kton::deposit_creating(&stash, COIN);
		let free_kton = Kton::free_balance(&stash);
		assert_ok!(Staking::try_claim_deposits_with_punish(
			Origin::signed(controller),
			deposit_item_expire_time,
		));

		// should claim success
		let slashed: KtonBalance<Test> = inflation::compute_kton_reward::<Test>(bond_value, 36)
			- inflation::compute_kton_reward::<Test>(bond_value, 14);
		System::assert_has_event(
			Event::DepositsClaimedWithPunish(ledger.stash.clone(), slashed * 3).into(),
		);
		// assert leger
		ledger.active_deposit_ring -= bond_value;
		ledger.deposit_items.clear();

		assert_eq!(Staking::ledger(controller).unwrap(), ledger);
		assert_eq!(Kton::free_balance(&stash), free_kton - slashed * 3);
	});
}

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

		assert!(Staking::ledger(&controller)
			.unwrap()
			.deposit_items
			.is_empty());

		// Deposit succeeded.
		assert_ok!(Staking::deposit_extra(Origin::signed(stash), COIN, 1));
		assert_eq!(Staking::ledger(&controller).unwrap().deposit_items.len(), 1);

		// NO-OP
		assert_ok!(Staking::deposit_extra(Origin::signed(stash), COIN, 1));
		assert_eq!(Staking::ledger(&controller).unwrap().deposit_items.len(), 1);
	});
}

#[test]
fn transform_to_deposited_ring_should_work() {
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
		assert_eq!(
			Kton::free_balance(&stash),
			kton_free_balance + (COIN / 10000)
		);
	});
}

#[test]
fn expired_ring_should_capable_to_promise_again() {
	ExtBuilder::default().build().execute_with(|| {
		let (stash, controller) = (1001, 1000);
		let _ = Ring::deposit_creating(&stash, 10);
		assert_ok!(Staking::bond(
			Origin::signed(stash),
			controller,
			StakingBalance::RingBalance(10),
			RewardDestination::Stash,
			12,
		));
		let mut ledger = Staking::ledger(controller).unwrap();
		let ts = 13 * MONTH_IN_MILLISECONDS;
		let promise_extra_value = 5;

		Timestamp::set_timestamp(ts);

		assert_ok!(Staking::deposit_extra(
			Origin::signed(stash),
			promise_extra_value,
			13,
		));
		ledger.active_deposit_ring = promise_extra_value;

		// old deposit_item with 12 months promised removed
		ledger.deposit_items = vec![TimeDepositItem {
			value: promise_extra_value,
			start_time: ts,
			expire_time: 2 * ts,
		}];
		assert_eq!(Staking::ledger(controller).unwrap(), ledger);
	});
}

#[test]
fn inflation_should_be_correct() {
	ExtBuilder::default().build().execute_with(|| {
		let initial_issuance = 1_200_000_000 * COIN;
		let surplus_needed = initial_issuance - Ring::total_issuance();
		let _ = Ring::deposit_into_existing(&11, surplus_needed);

		assert_eq!(Ring::total_issuance(), initial_issuance);
	});

	// breakpoint test
	// ExtBuilder::default().build().execute_with(|| {
	// 	gen_paired_account!(validator_1_stash(123), validator_1_controller(456), 0);
	// 	gen_paired_account!(validator_2_stash(234), validator_2_controller(567), 0);
	// 	gen_paired_account!(nominator_stash(345), nominator_controller(678), 0);
	//
	// 	assert_ok!(Staking::validate(
	// 		Origin::signed(validator_1_controller),
	// 		ValidatorPrefs::default(),
	// 	));
	// 	assert_ok!(Staking::validate(
	// 		Origin::signed(validator_2_controller),
	// 		ValidatorPrefs::default(),
	// 	));
	// 	assert_ok!(Staking::nominate(
	// 		Origin::signed(nominator_controller),
	// 		vec![validator_1_stash, validator_2_stash],
	// 	));
	//
	// 	Timestamp::set_timestamp(1_575_448_345_000 - 12_000);
	// 	// breakpoint here
	// 	Staking::new_era(1);
	//
	// 	Timestamp::set_timestamp(1_575_448_345_000);
	// 	// breakpoint here
	// 	Staking::new_era(2);
	//
	// 	// breakpoint here
	//     inflation::compute_total_payout::<Test>(11_999, 1_295_225_000, 9_987_999_900_000_000_000);
	//
	// 	loop {}
	// });
}

#[test]
fn slash_also_slash_unbondings() {
	ExtBuilder::default()
		.validator_count(1)
		.build()
		.execute_with(|| {
			start_active_era(0);

			let (account_id, bond) = (777, COIN);
			let _ = Ring::deposit_creating(&account_id, bond);

			assert_ok!(Staking::bond(
				Origin::signed(account_id),
				account_id,
				StakingBalance::RingBalance(bond),
				RewardDestination::Controller,
				0,
			));
			assert_ok!(Staking::validate(
				Origin::signed(account_id),
				ValidatorPrefs::default()
			));

			let mut ring_staking_lock = Staking::ledger(account_id)
				.unwrap()
				.ring_staking_lock
				.clone();

			start_active_era(1);

			assert_ok!(Staking::unbond(
				Origin::signed(account_id),
				StakingBalance::RingBalance(COIN / 2)
			));

			assert_eq_uvec!(validator_controllers(), vec![777]);

			on_offence_now(
				&[OffenceDetails {
					offender: (account_id, Staking::eras_stakers(active_era(), account_id)),
					reporters: vec![],
				}],
				&[Perbill::from_percent(100)],
			);

			ring_staking_lock.staking_amount = 0;
			ring_staking_lock.unbondings = WeakBoundedVec::force_from(vec![], None);

			assert_eq!(
				Staking::ledger(account_id).unwrap().ring_staking_lock,
				ring_staking_lock
			);
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
fn pool_should_be_increased_and_decreased_correctly() {
	ExtBuilder::default()
		.min_validator_bond(0)
		.build()
		.execute_with(|| {
			start_active_era(0);

			let mut ring_pool = Staking::ring_pool();
			let mut kton_pool = Staking::kton_pool();

			// bond: 100COIN
			gen_paired_account!(stash_1(111), controller_1(222), 0);
			gen_paired_account!(stash_2(333), controller_2(444), promise_month(12));
			ring_pool += 100 * COIN;
			kton_pool += 100 * COIN;
			assert_eq!(Staking::ring_pool(), ring_pool);
			assert_eq!(Staking::kton_pool(), kton_pool);

			// unbond: 50Ring 25Kton
			assert_ok!(Staking::unbond(
				Origin::signed(controller_1),
				StakingBalance::RingBalance(50 * COIN)
			));
			assert_ok!(Staking::unbond(
				Origin::signed(controller_1),
				StakingBalance::KtonBalance(25 * COIN)
			));
			// not yet expired: promise for 12 months
			assert_ok!(Staking::unbond(
				Origin::signed(controller_2),
				StakingBalance::RingBalance(50 * COIN)
			));
			assert_ok!(Staking::unbond(
				Origin::signed(controller_2),
				StakingBalance::KtonBalance(25 * COIN)
			));
			ring_pool -= 50 * COIN;
			kton_pool -= 50 * COIN;
			assert_eq!(Staking::ring_pool(), ring_pool);
			assert_eq!(Staking::kton_pool(), kton_pool);

			// claim: 50Ring
			assert_ok!(Staking::try_claim_deposits_with_punish(
				Origin::signed(controller_2),
				promise_month * MONTH_IN_MILLISECONDS,
			));
			// unbond deposit items: 12.5Ring
			let backup_ts = Timestamp::now();
			Timestamp::set_timestamp(INIT_TIMESTAMP + promise_month * MONTH_IN_MILLISECONDS);
			assert_ok!(Staking::unbond(
				Origin::signed(controller_2),
				StakingBalance::RingBalance(125 * COIN / 10),
			));
			ring_pool -= 125 * COIN / 10;
			assert_eq!(Staking::ring_pool(), ring_pool);

			Timestamp::set_timestamp(backup_ts);
			assert_ok!(Staking::validate(
				Origin::signed(controller_1),
				ValidatorPrefs::default()
			));
			assert_ok!(Staking::validate(
				Origin::signed(controller_2),
				ValidatorPrefs::default()
			));

			start_active_era(1);

			assert_eq_uvec!(validator_controllers(), vec![controller_1, controller_2]);

			// slash: 37.5Ring 50Kton
			on_offence_now(
				&[OffenceDetails {
					offender: (stash_1, Staking::eras_stakers(active_era(), stash_1)),
					reporters: vec![],
				}],
				&[Perbill::from_percent(100)],
			);
			on_offence_now(
				&[OffenceDetails {
					offender: (stash_2, Staking::eras_stakers(active_era(), stash_2)),
					reporters: vec![],
				}],
				&[Perbill::from_percent(100)],
			);

			ring_pool -= 375 * COIN / 10;
			kton_pool -= 50 * COIN;
			assert_eq!(Staking::ring_pool(), ring_pool);
			assert_eq!(Staking::kton_pool(), kton_pool);
		});

	ExtBuilder::default()
		.has_stakers(false)
		.build_and_execute(|| {
			bond_validator(11, 10, StakingBalance::RingBalance(1000));
			assert_ok!(Staking::set_payee(
				Origin::signed(10),
				RewardDestination::Staked
			));

			start_active_era(1);

			Staking::reward_by_ids(vec![(11, 1)]);
			let payout = current_total_payout_for_duration(reward_time_per_era());
			assert!(payout > 100);

			start_active_era(2);

			let ring_pool = Staking::ring_pool();
			assert_ok!(Staking::payout_stakers(Origin::signed(10), 11, 1));
			assert_eq!(Staking::ring_pool(), payout + ring_pool);
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

			assert_ok!(Staking::unbond(
				Origin::signed(controller),
				StakingBalance::RingBalance(1)
			));
		}

		assert_err!(
			Staking::unbond(Origin::signed(controller), StakingBalance::RingBalance(1)),
			StakingError::NoMoreChunks
		);
	});
}

#[test]
fn promise_extra_should_not_remove_unexpired_items() {
	ExtBuilder::default().build().execute_with(|| {
		gen_paired_account!(stash(123), controller(456), promise_month(12));
		let expired_items_len = 3;
		let expiry_date = INIT_TIMESTAMP + promise_month * MONTH_IN_MILLISECONDS;

		assert_ok!(Staking::bond_extra(
			Origin::signed(stash),
			StakingBalance::RingBalance(5 * COIN),
			0,
		));
		for _ in 0..expired_items_len {
			assert_ok!(Staking::deposit_extra(
				Origin::signed(stash),
				COIN,
				promise_month as u8
			));
		}

		Timestamp::set_timestamp(expiry_date - 1);
		assert_ok!(Staking::deposit_extra(
			Origin::signed(stash),
			2 * COIN,
			promise_month as u8,
		));
		assert_eq!(
			Staking::ledger(controller).unwrap().deposit_items.len(),
			2 + expired_items_len,
		);

		Timestamp::set_timestamp(expiry_date);
		assert_ok!(Staking::deposit_extra(
			Origin::signed(stash),
			2 * COIN,
			promise_month as u8,
		));
		assert_eq!(Staking::ledger(controller).unwrap().deposit_items.len(), 2);
	});
}

#[test]
fn unbond_zero() {
	ExtBuilder::default().build().execute_with(|| {
		gen_paired_account!(stash(123), controller(456), promise_month(12));
		let ledger = Staking::ledger(controller).unwrap();

		Timestamp::set_timestamp(promise_month * MONTH_IN_MILLISECONDS);
		assert_ok!(Staking::unbond(
			Origin::signed(10),
			StakingBalance::RingBalance(0)
		));
		assert_ok!(Staking::unbond(
			Origin::signed(10),
			StakingBalance::KtonBalance(0)
		));
		assert_eq!(Staking::ledger(controller).unwrap(), ledger);
	});
}

#[test]
fn on_deposit_redeem_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let deposit_amount = 1;
		let deposit_start_at = 1;
		let deposit_months = 3;
		let backing_account = 1;
		let deposit_item = TimeDepositItem {
			value: deposit_amount,
			start_time: deposit_start_at * 1000,
			expire_time: deposit_start_at * 1000 + deposit_months as TsInMs * MONTH_IN_MILLISECONDS,
		};

		// Not bond yet
		{
			let unbonded_account = 123;
			let ring_pool = Staking::ring_pool();

			assert_eq!(Ring::free_balance(unbonded_account), 0);
			assert!(Ring::locks(unbonded_account).is_empty());
			assert!(Staking::bonded(unbonded_account).is_none());
			assert_eq!(
				Staking::payee(unbonded_account),
				RewardDestination::default(),
			);
			assert!(Staking::ledger(unbonded_account).is_none());
			assert!(System::account(unbonded_account).providers == 0);

			assert_ok!(Staking::on_deposit_redeem(
				&backing_account,
				&unbonded_account,
				deposit_amount,
				deposit_start_at,
				deposit_months
			));

			assert_eq!(Ring::free_balance(unbonded_account), deposit_amount);
			assert_eq!(
				Ring::locks(unbonded_account),
				vec![BalanceLock {
					id: STAKING_ID,
					lock_for: LockFor::Staking(StakingLock {
						staking_amount: deposit_amount,
						..Default::default()
					}),
					lock_reasons: LockReasons::All,
				}]
			);
			assert_eq!(Staking::bonded(unbonded_account).unwrap(), unbonded_account);
			assert_eq!(Staking::payee(unbonded_account), RewardDestination::Stash);
			assert_eq!(
				Staking::ledger(unbonded_account).unwrap(),
				StakingLedger {
					stash: unbonded_account,
					active: deposit_amount,
					active_deposit_ring: deposit_amount,
					deposit_items: vec![deposit_item.clone()],
					ring_staking_lock: StakingLock {
						staking_amount: deposit_amount,
						unbondings: WeakBoundedVec::force_from(vec![], None)
					},
					..Default::default()
				}
			);
			assert_eq!(Staking::ring_pool(), ring_pool + deposit_amount);
			assert!(System::account(unbonded_account).providers != 0);
		}

		// Already bonded
		{
			gen_paired_account!(bonded_account(456), bonded_account(456), 0);

			let ring_pool = Staking::ring_pool();
			let mut ledger = Staking::ledger(bonded_account).unwrap();

			assert_eq!(Ring::free_balance(bonded_account), 101 * COIN);
			assert_eq!(Ring::locks(bonded_account).len(), 1);
			assert_eq!(Staking::bonded(bonded_account).unwrap(), bonded_account);

			assert_ok!(Staking::on_deposit_redeem(
				&backing_account,
				&bonded_account,
				deposit_amount,
				deposit_start_at,
				deposit_months
			));

			ledger.active += deposit_amount;
			ledger.active_deposit_ring += deposit_amount;
			ledger.deposit_items.push(deposit_item);
			ledger.ring_staking_lock.staking_amount += deposit_amount;

			assert_eq!(
				Ring::free_balance(bonded_account),
				101 * COIN + deposit_amount
			);
			assert_eq!(
				Ring::locks(bonded_account),
				vec![BalanceLock {
					id: STAKING_ID,
					lock_for: LockFor::Staking(StakingLock {
						staking_amount: 50 * COIN + deposit_amount,
						..Default::default()
					}),
					lock_reasons: LockReasons::All,
				}]
			);
			assert_eq!(Staking::ledger(bonded_account).unwrap(), ledger);
			assert_eq!(Staking::ring_pool(), ring_pool + deposit_amount);
		}
	});
}

// Origin test case name is `yakio_q1`
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
		assert_ok!(Staking::bond_extra(
			Origin::signed(stash),
			StakingBalance::KtonBalance(1),
			36
		));
		assert_eq!(Staking::ledger(controller).unwrap().active_kton, 1);

		// Become a nominator
		assert_ok!(Staking::nominate(
			Origin::signed(controller),
			vec![controller]
		));

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
fn staking_with_kton_with_unbondings() {
	ExtBuilder::default().build().execute_with(|| {
		let stash = 123;
		let controller = 456;
		let _ = Kton::deposit_creating(&stash, 10);

		assert_ok!(Staking::bond(
			Origin::signed(stash),
			controller,
			StakingBalance::KtonBalance(5),
			RewardDestination::Stash,
			0,
		));
		assert_eq!(Kton::free_balance(stash), 10);
		assert_eq!(
			Kton::locks(stash),
			vec![BalanceLock {
				id: STAKING_ID,
				lock_for: LockFor::Staking(StakingLock {
					staking_amount: 5,
					..Default::default()
				}),
				lock_reasons: LockReasons::All,
			}],
		);

		assert_ok!(Staking::bond_extra(
			Origin::signed(stash),
			StakingBalance::KtonBalance(5),
			0
		));
		assert_eq!(Kton::free_balance(stash), 10);
		assert_eq!(
			Kton::locks(stash),
			vec![BalanceLock {
				id: STAKING_ID,
				lock_for: LockFor::Staking(StakingLock {
					staking_amount: 10,
					..Default::default()
				}),
				lock_reasons: LockReasons::All,
			}]
		);

		let unbond_start = System::block_number();
		assert_ok!(Staking::unbond(
			Origin::signed(controller),
			StakingBalance::KtonBalance(9)
		));
		assert_eq!(Kton::free_balance(stash), 10);
		assert_eq!(
			Kton::locks(stash),
			vec![BalanceLock {
				id: STAKING_ID,
				lock_for: LockFor::Staking(StakingLock {
					staking_amount: 1,
					unbondings: WeakBoundedVec::force_from(
						vec![Unbonding {
							amount: 9,
							until: BondingDurationInBlockNumber::get() + unbond_start,
						}],
						None
					),
				}),
				lock_reasons: LockReasons::All,
			}]
		);

		assert_err!(
			Kton::transfer(Origin::signed(stash), controller, 1),
			KtonError::LiquidityRestrictions,
		);

		System::set_block_number(unbond_start + BondingDurationInBlockNumber::get());
		assert_ok!(Kton::transfer(Origin::signed(stash), controller, 1));
		assert_eq!(
			System::block_number(),
			unbond_start + BondingDurationInBlockNumber::get()
		);
		assert_eq!(Kton::free_balance(stash), 9);
		assert_eq!(
			Kton::locks(stash),
			vec![BalanceLock {
				id: STAKING_ID,
				lock_for: LockFor::Staking(StakingLock {
					staking_amount: 1,
					unbondings: WeakBoundedVec::force_from(
						vec![Unbonding {
							amount: 9,
							until: BondingDurationInBlockNumber::get() + unbond_start,
						}],
						None
					),
				}),
				lock_reasons: LockReasons::All,
			}]
		);

		let _ = Kton::deposit_creating(&stash, 20);
		assert_ok!(Staking::bond_extra(
			Origin::signed(stash),
			StakingBalance::KtonBalance(19),
			0
		));
		assert_eq!(Kton::free_balance(stash), 29);
		assert_eq!(
			Kton::locks(stash),
			vec![BalanceLock {
				id: STAKING_ID,
				lock_for: LockFor::Staking(StakingLock {
					staking_amount: 20,
					unbondings: WeakBoundedVec::force_from(
						vec![Unbonding {
							amount: 9,
							until: BondingDurationInBlockNumber::get() + unbond_start,
						}],
						None
					),
				}),
				lock_reasons: LockReasons::All,
			}]
		);
		assert_eq!(
			Staking::ledger(controller).unwrap(),
			StakingLedger {
				stash: 123,
				active_kton: 20,
				kton_staking_lock: StakingLock {
					staking_amount: 20,
					unbondings: WeakBoundedVec::force_from(
						vec![Unbonding {
							amount: 9,
							until: BondingDurationInBlockNumber::get() + unbond_start,
						}],
						None
					),
				},
				..Default::default()
			}
		);
	});

	ExtBuilder::default().build().execute_with(|| {
		let stash = 123;
		let controller = 456;
		let _ = Ring::deposit_creating(&stash, 10);

		assert_ok!(Staking::bond(
			Origin::signed(stash),
			controller,
			StakingBalance::RingBalance(5),
			RewardDestination::Stash,
			0,
		));
		assert_eq!(Ring::free_balance(stash), 10);
		assert_eq!(
			Ring::locks(stash),
			vec![BalanceLock {
				id: STAKING_ID,
				lock_for: LockFor::Staking(StakingLock {
					staking_amount: 5,
					..Default::default()
				}),
				lock_reasons: LockReasons::All,
			}]
		);

		assert_ok!(Staking::bond_extra(
			Origin::signed(stash),
			StakingBalance::RingBalance(5),
			0
		));
		assert_eq!(Ring::free_balance(stash), 10);
		assert_eq!(
			Ring::locks(stash),
			vec![BalanceLock {
				id: STAKING_ID,
				lock_for: LockFor::Staking(StakingLock {
					staking_amount: 10,
					..Default::default()
				}),
				lock_reasons: LockReasons::All,
			}]
		);

		let unbond_start = System::block_number();
		assert_ok!(Staking::unbond(
			Origin::signed(controller),
			StakingBalance::RingBalance(9)
		));
		assert_eq!(Ring::free_balance(stash), 10);
		assert_eq!(
			Ring::locks(stash),
			vec![BalanceLock {
				id: STAKING_ID,
				lock_for: LockFor::Staking(StakingLock {
					staking_amount: 1,
					unbondings: WeakBoundedVec::force_from(
						vec![Unbonding {
							amount: 9,
							until: BondingDurationInBlockNumber::get() + unbond_start,
						}],
						None
					)
				}),
				lock_reasons: LockReasons::All,
			}]
		);

		assert_err!(
			Ring::transfer(Origin::signed(stash), controller, 1),
			RingError::LiquidityRestrictions,
		);

		System::set_block_number(BondingDurationInBlockNumber::get() + unbond_start);
		assert_ok!(Ring::transfer(Origin::signed(stash), controller, 1));
		assert_eq!(
			System::block_number(),
			BondingDurationInBlockNumber::get() + unbond_start
		);
		assert_eq!(Ring::free_balance(stash), 9);
		assert_eq!(
			Ring::locks(stash),
			vec![BalanceLock {
				id: STAKING_ID,
				lock_for: LockFor::Staking(StakingLock {
					staking_amount: 1,
					unbondings: WeakBoundedVec::force_from(
						vec![Unbonding {
							amount: 9,
							until: BondingDurationInBlockNumber::get() + unbond_start,
						}],
						None
					)
				}),
				lock_reasons: LockReasons::All,
			}]
		);

		let _ = Ring::deposit_creating(&stash, 20);
		assert_ok!(Staking::bond_extra(
			Origin::signed(stash),
			StakingBalance::RingBalance(19),
			0
		));
		assert_eq!(Ring::free_balance(stash), 29);
		assert_eq!(
			Ring::locks(stash),
			vec![BalanceLock {
				id: STAKING_ID,
				lock_for: LockFor::Staking(StakingLock {
					staking_amount: 20,
					unbondings: WeakBoundedVec::force_from(
						vec![Unbonding {
							amount: 9,
							until: BondingDurationInBlockNumber::get() + unbond_start,
						}],
						None
					)
				}),
				lock_reasons: LockReasons::All,
			}]
		);
		assert_eq!(
			Staking::ledger(controller).unwrap(),
			StakingLedger {
				stash: 123,
				active: 20,
				ring_staking_lock: StakingLock {
					staking_amount: 20,
					unbondings: WeakBoundedVec::force_from(
						vec![Unbonding {
							amount: 9,
							until: BondingDurationInBlockNumber::get() + unbond_start,
						}],
						None
					)
				},
				..Default::default()
			}
		);
	});
}

#[test]
fn unbound_values_in_twice() {
	ExtBuilder::default().build().execute_with(|| {
		let stash = 123;
		let controller = 456;
		let _ = Kton::deposit_creating(&stash, 10);

		assert_ok!(Staking::bond(
			Origin::signed(stash),
			controller,
			StakingBalance::KtonBalance(5),
			RewardDestination::Stash,
			0,
		));
		assert_eq!(Kton::free_balance(stash), 10);
		assert_eq!(
			Kton::locks(stash),
			vec![BalanceLock {
				id: STAKING_ID,
				lock_for: LockFor::Staking(StakingLock {
					staking_amount: 5,
					..Default::default()
				}),
				lock_reasons: LockReasons::All,
			}]
		);

		assert_ok!(Staking::bond_extra(
			Origin::signed(stash),
			StakingBalance::KtonBalance(4),
			0
		));
		assert_eq!(Kton::free_balance(stash), 10);
		assert_eq!(
			Kton::locks(stash),
			vec![BalanceLock {
				id: STAKING_ID,
				lock_for: LockFor::Staking(StakingLock {
					staking_amount: 9,
					..Default::default()
				}),
				lock_reasons: LockReasons::All,
			}]
		);

		let (unbond_start_1, unbond_value_1) = (System::block_number(), 2);
		assert_ok!(Staking::unbond(
			Origin::signed(controller),
			StakingBalance::KtonBalance(unbond_value_1),
		));
		assert_eq!(Kton::free_balance(stash), 10);
		assert_eq!(
			Kton::locks(stash),
			vec![BalanceLock {
				id: STAKING_ID,
				lock_for: LockFor::Staking(StakingLock {
					staking_amount: 7,
					unbondings: WeakBoundedVec::force_from(
						vec![Unbonding {
							amount: 2,
							until: BondingDurationInBlockNumber::get() + unbond_start_1,
						}],
						None
					),
				}),
				lock_reasons: LockReasons::All,
			}]
		);

		let (unbond_start_2, unbond_value_2) = (System::block_number(), 6);
		assert_ok!(Staking::unbond(
			Origin::signed(controller),
			StakingBalance::KtonBalance(6)
		));
		assert_eq!(Kton::free_balance(stash), 10);
		assert_eq!(
			Kton::locks(stash),
			vec![BalanceLock {
				id: STAKING_ID,
				lock_for: LockFor::Staking(StakingLock {
					staking_amount: 1,
					unbondings: WeakBoundedVec::force_from(
						vec![
							Unbonding {
								amount: 2,
								until: BondingDurationInBlockNumber::get() + unbond_start_1,
							},
							Unbonding {
								amount: 6,
								until: BondingDurationInBlockNumber::get() + unbond_start_2,
							}
						],
						None
					)
				}),
				lock_reasons: LockReasons::All,
			}]
		);

		assert_err!(
			Kton::transfer(Origin::signed(stash), controller, unbond_value_1),
			KtonError::LiquidityRestrictions,
		);
		assert_ok!(Kton::transfer(
			Origin::signed(stash),
			controller,
			unbond_value_1 - 1
		));
		assert_eq!(Kton::free_balance(stash), 9);

		assert_err!(
			Kton::transfer(Origin::signed(stash), controller, unbond_value_1 + 1),
			KtonError::LiquidityRestrictions,
		);
		System::set_block_number(BondingDurationInBlockNumber::get() + unbond_start_1);
		assert_ok!(Kton::transfer(
			Origin::signed(stash),
			controller,
			unbond_value_1
		));
		assert_eq!(
			System::block_number(),
			BondingDurationInBlockNumber::get() + unbond_start_1
		);
		assert_eq!(Kton::free_balance(stash), 7);
		assert_eq!(
			Kton::locks(stash),
			vec![BalanceLock {
				id: STAKING_ID,
				lock_for: LockFor::Staking(StakingLock {
					staking_amount: 1,
					unbondings: WeakBoundedVec::force_from(
						vec![
							Unbonding {
								amount: 2,
								until: BondingDurationInBlockNumber::get() + unbond_start_1,
							},
							Unbonding {
								amount: 6,
								until: BondingDurationInBlockNumber::get() + unbond_start_2,
							}
						],
						None
					)
				}),
				lock_reasons: LockReasons::All,
			}]
		);

		assert_ok!(Kton::transfer(
			Origin::signed(stash),
			controller,
			unbond_value_2
		));
		assert_eq!(Kton::free_balance(stash), 1);
		assert_eq!(
			Kton::locks(stash),
			vec![BalanceLock {
				id: STAKING_ID,
				lock_for: LockFor::Staking(StakingLock {
					staking_amount: 1,
					unbondings: WeakBoundedVec::force_from(
						vec![
							Unbonding {
								amount: 2,
								until: BondingDurationInBlockNumber::get() + unbond_start_1,
							},
							Unbonding {
								amount: 6,
								until: BondingDurationInBlockNumber::get() + unbond_start_2,
							}
						],
						None
					)
				}),
				lock_reasons: LockReasons::All,
			}]
		);

		let _ = Kton::deposit_creating(&stash, 1);
		assert_eq!(Kton::free_balance(stash), 2);
		assert_ok!(Staking::bond_extra(
			Origin::signed(stash),
			StakingBalance::KtonBalance(1),
			0
		));
		assert_eq!(
			Kton::locks(stash),
			vec![BalanceLock {
				id: STAKING_ID,
				lock_for: LockFor::Staking(StakingLock {
					staking_amount: 2,
					unbondings: WeakBoundedVec::force_from(
						vec![
							Unbonding {
								amount: 2,
								until: BondingDurationInBlockNumber::get() + unbond_start_1,
							},
							Unbonding {
								amount: 6,
								until: BondingDurationInBlockNumber::get() + unbond_start_2,
							}
						],
						None
					)
				}),
				lock_reasons: LockReasons::All,
			}]
		);
	});

	ExtBuilder::default().build().execute_with(|| {
		let stash = 123;
		let controller = 456;
		let _ = Ring::deposit_creating(&stash, 10);

		Timestamp::set_timestamp(1);
		assert_ok!(Staking::bond(
			Origin::signed(stash),
			controller,
			StakingBalance::RingBalance(5),
			RewardDestination::Stash,
			0,
		));
		assert_eq!(Ring::free_balance(stash), 10);
		assert_eq!(
			Ring::locks(stash),
			vec![BalanceLock {
				id: STAKING_ID,
				lock_for: LockFor::Staking(StakingLock {
					staking_amount: 5,
					..Default::default()
				}),
				lock_reasons: LockReasons::All,
			}]
		);

		assert_ok!(Staking::bond_extra(
			Origin::signed(stash),
			StakingBalance::RingBalance(4),
			0
		));
		assert_eq!(Ring::free_balance(stash), 10);
		assert_eq!(
			Ring::locks(stash),
			vec![BalanceLock {
				id: STAKING_ID,
				lock_for: LockFor::Staking(StakingLock {
					staking_amount: 9,
					..Default::default()
				}),
				lock_reasons: LockReasons::All,
			}]
		);

		let (unbond_start_1, unbond_value_1) = (System::block_number(), 2);
		assert_ok!(Staking::unbond(
			Origin::signed(controller),
			StakingBalance::RingBalance(unbond_value_1)
		));
		assert_eq!(System::block_number(), unbond_start_1);
		assert_eq!(Ring::free_balance(stash), 10);
		assert_eq!(
			Ring::locks(stash),
			vec![BalanceLock {
				id: STAKING_ID,
				lock_for: LockFor::Staking(StakingLock {
					staking_amount: 7,
					unbondings: WeakBoundedVec::force_from(
						vec![Unbonding {
							amount: 2,
							until: BondingDurationInBlockNumber::get() + unbond_start_1,
						}],
						None
					)
				}),
				lock_reasons: LockReasons::All,
			}]
		);

		let (unbond_start_2, unbond_value_2) = (System::block_number(), 6);
		assert_ok!(Staking::unbond(
			Origin::signed(controller),
			StakingBalance::RingBalance(6)
		));
		assert_eq!(System::block_number(), unbond_start_2);
		assert_eq!(Ring::free_balance(stash), 10);
		assert_eq!(
			Ring::locks(stash),
			vec![BalanceLock {
				id: STAKING_ID,
				lock_for: LockFor::Staking(StakingLock {
					staking_amount: 1,
					unbondings: WeakBoundedVec::force_from(
						vec![
							Unbonding {
								amount: 2,
								until: BondingDurationInBlockNumber::get() + unbond_start_1,
							},
							Unbonding {
								amount: 6,
								until: BondingDurationInBlockNumber::get() + unbond_start_2,
							}
						],
						None
					)
				}),
				lock_reasons: LockReasons::All,
			}]
		);

		assert_err!(
			Ring::transfer(Origin::signed(stash), controller, unbond_value_1),
			RingError::LiquidityRestrictions,
		);

		assert_ok!(Ring::transfer(
			Origin::signed(stash),
			controller,
			unbond_value_1 - 1
		));
		assert_eq!(Ring::free_balance(stash), 9);
		assert_err!(
			Ring::transfer(Origin::signed(stash), controller, unbond_value_1 + 1),
			RingError::LiquidityRestrictions,
		);
		System::set_block_number(BondingDurationInBlockNumber::get() + unbond_start_1);
		assert_ok!(Ring::transfer(
			Origin::signed(stash),
			controller,
			unbond_value_1
		));
		assert_eq!(
			System::block_number(),
			BondingDurationInBlockNumber::get() + unbond_start_1
		);
		assert_eq!(Ring::free_balance(stash), 7);
		assert_eq!(
			Ring::locks(stash),
			vec![BalanceLock {
				id: STAKING_ID,
				lock_for: LockFor::Staking(StakingLock {
					staking_amount: 1,
					unbondings: WeakBoundedVec::force_from(
						vec![
							Unbonding {
								amount: 2,
								until: BondingDurationInBlockNumber::get() + unbond_start_1,
							},
							Unbonding {
								amount: 6,
								until: BondingDurationInBlockNumber::get() + unbond_start_2,
							}
						],
						None
					)
				}),
				lock_reasons: LockReasons::All,
			}]
		);
		assert_ok!(Ring::transfer(
			Origin::signed(stash),
			controller,
			unbond_value_2
		));
		assert_eq!(
			System::block_number(),
			BondingDurationInBlockNumber::get() + unbond_start_2
		);
		assert_eq!(Ring::free_balance(stash), 1);
		assert_eq!(
			Ring::locks(stash),
			vec![BalanceLock {
				id: STAKING_ID,
				lock_for: LockFor::Staking(StakingLock {
					staking_amount: 1,
					unbondings: WeakBoundedVec::force_from(
						vec![
							Unbonding {
								amount: 2,
								until: BondingDurationInBlockNumber::get() + unbond_start_1,
							},
							Unbonding {
								amount: 6,
								until: BondingDurationInBlockNumber::get() + unbond_start_2,
							}
						],
						None
					)
				}),
				lock_reasons: LockReasons::All,
			}]
		);

		let _ = Ring::deposit_creating(&stash, 1);
		//		println!("Staking Ledger: {:#?}", Staking::ledger(controller).unwrap());
		assert_eq!(Ring::free_balance(stash), 2);
		assert_ok!(Staking::bond_extra(
			Origin::signed(stash),
			StakingBalance::RingBalance(1),
			0
		));
		assert_eq!(
			Ring::locks(stash),
			vec![BalanceLock {
				id: STAKING_ID,
				lock_for: LockFor::Staking(StakingLock {
					staking_amount: 2,
					unbondings: WeakBoundedVec::force_from(
						vec![
							Unbonding {
								amount: 2,
								until: BondingDurationInBlockNumber::get() + unbond_start_1,
							},
							Unbonding {
								amount: 6,
								until: BondingDurationInBlockNumber::get() + unbond_start_2,
							}
						],
						None
					)
				}),
				lock_reasons: LockReasons::All,
			}]
		);
	});
}

// Original testcase name is `xavier_q3`
//
// The values(KTON, RING) are unbond in the moment that there are values unbonding
#[test]
fn bond_values_when_some_value_unbonding() {
	// The Kton part
	ExtBuilder::default().build().execute_with(|| {
		let stash = 123;
		let controller = 456;
		let _ = Kton::deposit_creating(&stash, 10);

		let start = System::block_number();
		assert_ok!(Staking::bond(
			Origin::signed(stash),
			controller,
			StakingBalance::KtonBalance(5),
			RewardDestination::Stash,
			0,
		));

		assert_eq!(
			Staking::ledger(controller).unwrap(),
			StakingLedger {
				stash: 123,
				active_kton: 5,
				kton_staking_lock: StakingLock {
					staking_amount: 5,
					..Default::default()
				},
				..Default::default()
			},
		);

		// all values are unbond
		assert_ok!(Staking::unbond(
			Origin::signed(controller),
			StakingBalance::KtonBalance(5)
		));
		assert_eq!(
			Staking::ledger(controller).unwrap(),
			StakingLedger {
				stash: 123,
				kton_staking_lock: StakingLock {
					staking_amount: 0,
					unbondings: WeakBoundedVec::force_from(
						vec![Unbonding {
							amount: 5,
							until: start + BondingDurationInBlockNumber::get(),
						}],
						None
					)
				},
				..Default::default()
			},
		);

		System::set_block_number(start + BondingDurationInBlockNumber::get());
		// unbond zero to release unbondings
		assert_ok!(Staking::unbond(
			Origin::signed(controller),
			StakingBalance::KtonBalance(0)
		));
		assert_eq!(
			Staking::ledger(controller).unwrap(),
			StakingLedger {
				stash: 123,
				..Default::default()
			},
		);

		// bond again
		assert_ok!(Staking::bond_extra(
			Origin::signed(stash),
			StakingBalance::KtonBalance(1),
			0,
		));
		assert_eq!(
			Staking::ledger(controller).unwrap(),
			StakingLedger {
				stash: 123,
				active_kton: 1,
				kton_staking_lock: StakingLock {
					staking_amount: 1,
					..Default::default()
				},
				..Default::default()
			},
		);
	});

	// The Ring part
	ExtBuilder::default().build().execute_with(|| {
		let stash = 123;
		let controller = 456;
		let _ = Ring::deposit_creating(&stash, 10);

		let start = System::block_number();
		assert_ok!(Staking::bond(
			Origin::signed(stash),
			controller,
			StakingBalance::RingBalance(5),
			RewardDestination::Stash,
			0,
		));

		assert_eq!(
			Staking::ledger(controller).unwrap(),
			StakingLedger {
				stash: 123,
				active: 5,
				ring_staking_lock: StakingLock {
					staking_amount: 5,
					..Default::default()
				},
				..Default::default()
			},
		);

		// all values are unbond
		assert_ok!(Staking::unbond(
			Origin::signed(controller),
			StakingBalance::RingBalance(5),
		));
		assert_eq!(
			Staking::ledger(controller).unwrap(),
			StakingLedger {
				stash: 123,
				ring_staking_lock: StakingLock {
					staking_amount: 0,
					unbondings: WeakBoundedVec::force_from(
						vec![Unbonding {
							amount: 5,
							until: start + BondingDurationInBlockNumber::get(),
						}],
						None
					)
				},
				..Default::default()
			},
		);

		System::set_block_number(start + BondingDurationInBlockNumber::get());
		// unbond zero to release unbondings
		assert_ok!(Staking::unbond(
			Origin::signed(controller),
			StakingBalance::RingBalance(0),
		));
		assert_eq!(
			Staking::ledger(controller).unwrap(),
			StakingLedger {
				stash: 123,
				..Default::default()
			},
		);

		// bond again
		assert_ok!(Staking::bond_extra(
			Origin::signed(stash),
			StakingBalance::RingBalance(1),
			0,
		));
		assert_eq!(
			Staking::ledger(controller).unwrap(),
			StakingLedger {
				stash: 123,
				active: 1,
				ring_staking_lock: StakingLock {
					staking_amount: 1,
					..Default::default()
				},
				..Default::default()
			}
		);
	});
}

#[test]
fn rebond_event_should_work() {
	ExtBuilder::default()
		.nominate(false)
		.build()
		.execute_with(|| {
			assert_ok!(Staking::set_payee(
				Origin::signed(10),
				RewardDestination::Controller
			));

			let _ = Ring::make_free_balance_be(&11, 1000000);

			run_to_block(5);

			assert_eq!(
				Staking::ledger(&10),
				Some(StakingLedger {
					stash: 11,
					active: 1000,
					ring_staking_lock: StakingLock {
						staking_amount: 1000,
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
						staking_amount: 600,
						unbondings: WeakBoundedVec::force_from(
							vec![Unbonding {
								amount: 400,
								until: 6 + bonding_duration_in_blocks(),
							}],
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
						staking_amount: 800,
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
