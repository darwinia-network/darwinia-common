//! Tests for the module.

// --- substrate ---
use frame_support::assert_ok;
use substrate_test_utils::assert_eq_uvec;
// --- darwinia ---
use crate::{mock::*, *};

#[test]
fn slash_ledger_should_work() {
	ExtBuilder::default()
		.nominate(false)
		.validator_count(1)
		.build()
		.execute_with(|| {
			start_era(0);

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

			start_era(1);

			assert_eq_uvec!(validator_controllers(), vec![777]);

			on_offence_now(
				&[OffenceDetails {
					offender: (
						account_id,
						Staking::eras_stakers(Staking::active_era().unwrap().index, account_id),
					),
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
						active_ring: total - total_slashed,
						active_deposit_ring: deposit - deposit_slashed,
						deposit_items: vec![TimeDepositItem {
							value: deposit - deposit_slashed,
							start_time: 30000,
							expire_time: 93312030000,
						}],
						ring_staking_lock: StakingLock {
							staking_amount: deposit - deposit_slashed,
							unbondings: vec![],
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
		.init_ring(false)
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
fn migration_should_fix_broken_ledger() {
	let mut s = sp_storage::Storage::default();
	let id: mock::AccountId = 777;
	let mut broken_ledger =
		StakingLedger::<mock::AccountId, mock::Balance, mock::Balance, mock::BlockNumber> {
			stash: id,
			active_ring: 1000,
			active_deposit_ring: 1,
			deposit_items: vec![
				TimeDepositItem {
					value: 1,
					start_time: 0,
					expire_time: 1,
				},
				TimeDepositItem {
					value: 2,
					start_time: 1,
					expire_time: 2,
				},
				TimeDepositItem {
					value: 3,
					start_time: 2,
					expire_time: 3,
				},
			],
			ring_staking_lock: StakingLock {
				staking_amount: 1000,
				unbondings: vec![],
			},
			..Default::default()
		};
	let data = vec![(
		<Ledger<Test>>::hashed_key_for(id),
		broken_ledger.encode().to_vec(),
	)];

	s.top = data.into_iter().collect();
	sp_io::TestExternalities::new(s).execute_with(|| {
		let _ = Ring::deposit_creating(&id, 200);

		assert_eq!(Staking::ledger(&id).unwrap(), broken_ledger);

		crate::migration::migrate::<Test>();

		broken_ledger.active_deposit_ring = 6;

		assert_eq!(Staking::ledger(&id).unwrap(), broken_ledger);
	});
}

#[cfg(feature = "backup")]
mod backup {
	/// gen_paired_account!(a(1), b(2), m(12));
	/// will create stash `a` and controller `b`
	/// `a` has 100 Ring and 100 Kton
	/// promise for `m` month with 50 Ring and 50 Kton
	///
	/// `m` can be ignore, this won't create variable `m`
	/// ```rust
	/// gen_parired_account!(a(1), b(2), 12);
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

	// @review(deprecated): this should not work, please delete it after review.
	// due to: https://github.com/paritytech/substrate/blob/013c1ee167354a08283fb69915fda56a62fee943/frame/staking/src/mock.rs#L290
	// #[test]
	// fn bond_zero_should_work() {
	// 	ExtBuilder::default().build().execute_with(|| {
	// 		let (stash, controller) = (123, 456);
	// 		assert_ok!(Staking::bond(
	// 			Origin::signed(stash),
	// 			controller,
	// 			StakingBalance::RingBalance(0),
	// 			RewardDestination::Stash,
	// 			0,
	// 		));
	//
	// 		let (stash, controller) = (234, 567);
	// 		assert_ok!(Staking::bond(
	// 			Origin::signed(stash),
	// 			controller,
	// 			StakingBalance::KtonBalance(0),
	// 			RewardDestination::Stash,
	// 			0,
	// 		));
	// 	});
	// }

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
						active_ring: 0,
						active_deposit_ring: 0,
						active_kton: 10 * COIN,
						deposit_items: vec![],
						ring_staking_lock: Default::default(),
						kton_staking_lock: StakingLock {
							staking_amount: 10 * COIN,
							unbondings: vec![],
						},
						last_reward: Some(0)
					}
				);
				assert_eq!(
					Kton::locks(&stash),
					vec![BalanceLock {
						id: STAKING_ID,
						lock_for: LockFor::Staking(StakingLock {
							staking_amount: 10 * COIN,
							unbondings: vec![],
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
						active_ring: 0,
						active_deposit_ring: 0,
						active_kton: 10 * COIN,
						deposit_items: vec![],
						ring_staking_lock: Default::default(),
						kton_staking_lock: StakingLock {
							staking_amount: 10 * COIN,
							unbondings: vec![],
						},
						last_reward: Some(0),
					}
				);
			}
		});
	}

	#[test]
	fn time_deposit_ring_unbond_and_withdraw_automatically_should_work() {
		ExtBuilder::default().build().execute_with(|| {
			let (stash, controller) = (11, 10);
			assert_eq!(BondingDurationInEra::get(), 3);

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
				vec![BalanceLock {
					id: STAKING_ID,
					lock_for: LockFor::Staking(StakingLock {
						staking_amount: 1000 - unbond_value,
						unbondings: vec![Unbonding {
							amount: unbond_value,
							until: BondingDurationInBlockNumber::get() + start,
						}],
					}),
					lock_reasons: LockReasons::All,
				}],
			);

			// check the ledger
			assert_eq!(
				Staking::ledger(controller).unwrap(),
				StakingLedger {
					stash,
					active_ring: 1000 - unbond_value,
					active_deposit_ring: 0,
					active_kton: 0,
					deposit_items: vec![],
					ring_staking_lock: StakingLock {
						staking_amount: 1000 - unbond_value,
						unbondings: vec![Unbonding {
							amount: unbond_value,
							until: BondingDurationInBlockNumber::get() + start,
						}],
					},
					kton_staking_lock: Default::default(),
					last_reward: None,
				},
			);

			let unbond_start = 30;
			System::set_block_number(unbond_start);

			// unbond for the second time
			assert_ok!(Staking::unbond(
				Origin::signed(controller),
				StakingBalance::RingBalance(COIN)
			));

			// check the locks
			assert_eq!(
				Ring::locks(stash),
				vec![BalanceLock {
					id: STAKING_ID,
					lock_for: LockFor::Staking(StakingLock {
						staking_amount: 0,
						unbondings: vec![
							Unbonding {
								amount: unbond_value,
								until: BondingDurationInBlockNumber::get() + start,
							},
							Unbonding {
								amount: 1000 - unbond_value,
								until: BondingDurationInBlockNumber::get() + unbond_start,
							},
						],
					}),
					lock_reasons: LockReasons::All,
				}],
			);

			// check the ledger, it will be empty because we have
			// just unbonded all balances, the ledger is drained.
			assert!(Staking::ledger(controller).is_none());

			// We can't transfer current now.
			assert_err!(
				Ring::transfer(Origin::signed(stash), controller, 1),
				RingError::<Test, _>::LiquidityRestrictions
			);

			// Let's move to the until block
			System::set_block_number(BondingDurationInBlockNumber::get() + unbond_start);
			assert_eq!(Ring::locks(&stash).len(), 1);

			// stash account can transfer again!
			assert_ok!(Ring::transfer(Origin::signed(stash), controller, 1));
		});
	}

	#[test]
	fn normal_unbond_should_work() {
		ExtBuilder::default().build().execute_with(|| {
			let (stash, controller) = (11, 10);
			let value = 200 * COIN;
			let promise_month = 12;
			let _ = Ring::deposit_creating(&stash, 1000 * COIN);
			let start = System::block_number();

			{
				let kton_free_balance = Kton::free_balance(&stash);
				let mut ledger = Staking::ledger(controller).unwrap();

				assert_ok!(Staking::bond_extra(
					Origin::signed(stash),
					StakingBalance::RingBalance(value),
					promise_month,
				));
				assert_eq!(
					Kton::free_balance(&stash),
					kton_free_balance
						+ inflation::compute_kton_return::<Test>(value, promise_month)
				);
				ledger.active_ring += value;
				ledger.active_deposit_ring += value;
				ledger.deposit_items.push(TimeDepositItem {
					value,
					start_time: 0,
					expire_time: promise_month * MONTH_IN_MILLISECONDS,
				});
				ledger.ring_staking_lock.staking_amount += value;
				assert_eq!(Staking::ledger(controller).unwrap(), ledger);
			}

			{
				let kton_free_balance = Kton::free_balance(&stash);
				let mut ledger = Staking::ledger(controller).unwrap();

				//TODO: checkout the staking following staking values
				// We try to bond 1 kton, but stash only has 0.2 Kton.
				// extra = COIN.min(20_000_000)
				// bond += 20_000_000
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
				ledger.kton_staking_lock.unbondings.push(Unbonding {
					amount: kton_free_balance,
					until: BondingDurationInBlockNumber::get() + start,
				});

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
				active_ring: bond_value,
				active_deposit_ring: bond_value,
				active_kton: 0,
				deposit_items: vec![TimeDepositItem {
					value: bond_value,
					start_time: 0,
					expire_time: promise_month * MONTH_IN_MILLISECONDS,
				}],
				ring_staking_lock: StakingLock {
					staking_amount: bond_value,
					unbondings: vec![],
				},
				kton_staking_lock: Default::default(),
				last_reward: Some(0),
			};

			assert_ok!(Staking::bond(
				Origin::signed(stash),
				controller,
				StakingBalance::RingBalance(bond_value),
				RewardDestination::Stash,
				promise_month,
			));
			assert_eq!(Staking::ledger(controller).unwrap(), ledger);
			// Kton is 0, skip `unbond_with_punish`.
			assert_ok!(Staking::try_claim_deposits_with_punish(
				Origin::signed(controller),
				promise_month * MONTH_IN_MILLISECONDS,
			));
			assert_eq!(Staking::ledger(controller).unwrap(), ledger);
			// Set more kton balance to make it work.
			let _ = Kton::deposit_creating(&stash, COIN);
			assert_ok!(Staking::try_claim_deposits_with_punish(
				Origin::signed(controller),
				promise_month * MONTH_IN_MILLISECONDS,
			));
			ledger.active_deposit_ring -= bond_value;
			ledger.deposit_items.clear();
			assert_eq!(Staking::ledger(controller).unwrap(), ledger);
			assert_eq!(Kton::free_balance(&stash), COIN - 3);
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
				start_time: 0,
				expire_time: 12 * MONTH_IN_MILLISECONDS,
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

		// @review(inflation): check the purpose.
		// TODO: Maybe we should remove this, if these is not used
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
	fn validator_payment_ratio_should_work() {
		ExtBuilder::default().build().execute_with(|| {
			gen_paired_account!(validator_stash(123), validator_controller(456), 0);
			gen_paired_account!(nominator_stash(345), nominator_controller(678), 0);

			assert_ok!(Staking::validate(
				Origin::signed(validator_controller),
				ValidatorPrefs::default(),
			));
			assert_ok!(Staking::nominate(
				Origin::signed(nominator_controller),
				vec![validator_stash],
			));

			// assert_eq!(Session::validators(&valdator_stash, COIN).0.peek(), 0);

			assert_ok!(Staking::chill(Origin::signed(validator_controller)));
			assert_ok!(Staking::chill(Origin::signed(nominator_controller)));

			assert_ok!(Staking::validate(
				Origin::signed(validator_controller),
				ValidatorPrefs {
					commission: Perbill::from_percent(100)
				},
			));
			assert_ok!(Staking::nominate(
				Origin::signed(nominator_controller),
				vec![validator_stash],
			));

			// assert_eq!(Staking::reward_validator(&validator_stash, COIN).0.peek(), COIN);
		});
	}

	// @rm(outdated): `check_node_name_should_work`

	// @darwinia(breakpoint)
	#[test]
	fn slash_should_not_touch_unbondings() {
		ExtBuilder::default().build().execute_with(|| {
			let (stash, controller) = (11, 10);

			assert_ok!(Staking::deposit_extra(Origin::signed(stash), 1000, 12));
			let ledger = Staking::ledger(controller).unwrap();
			// Only deposit_ring, no normal_ring.
			assert_eq!(
				(ledger.active_ring, ledger.active_deposit_ring),
				(1000, 1000)
			);

			let _ = Ring::deposit_creating(&stash, 1000);
			assert_ok!(Staking::bond_extra(
				Origin::signed(stash),
				StakingBalance::RingBalance(1000),
				0,
			));
			let _ = Kton::deposit_creating(&stash, 1000);
			assert_ok!(Staking::bond_extra(
				Origin::signed(stash),
				StakingBalance::KtonBalance(1000),
				0,
			));

			assert_ok!(Staking::unbond(
				Origin::signed(controller),
				StakingBalance::RingBalance(10)
			));
			let ledger = Staking::ledger(controller).unwrap();
			let unbondings = (
				ledger.ring_staking_lock.unbondings.clone(),
				ledger.kton_staking_lock.unbondings.clone(),
			);

			// @review(reward): check if below is correct
			// assert_eq!(
			// 	(ledger.active_ring, ledger.active_deposit_ring),
			// 	(1000 + 1000 - 10, 1000),
			// );
			// ----

			<ErasStakers<Test>>::insert(
				0,
				&stash,
				Exposure {
					own_ring_balance: 1,
					total_power: 1,
					own_kton_balance: 0,
					own_power: 0,
					others: vec![],
				},
			);

			// TODO: check slash_validator issue
			// FIXME: slash strategy
			// let _ = Staking::slash_validator(&stash, Power::max_value(), &Staking::stakers(&stash), &mut vec![]);
			// let ledger = Staking::ledger(controller).unwrap();
			// assert_eq!(
			// 	(
			// 		ledger.ring_staking_lock.unbondings.clone(),
			// 		ledger.kton_staking_lock.unbondings.clone(),
			// 	),
			// 	unbondings,
			// );
			// assert_eq!((ledger.active_ring, ledger.active_deposit_ring), (0, 0));
		});
	}

	#[test]
	fn check_stash_already_bonded_and_controller_already_paired() {
		ExtBuilder::default().build().execute_with(|| {
			gen_paired_account!(unpaired_stash(123), unpaired_controller(456));
			assert_noop!(
				Staking::bond(
					Origin::signed(11),
					unpaired_controller,
					StakingBalance::RingBalance(COIN),
					RewardDestination::Stash,
					0,
				),
				DispatchError::Module {
					index: 0,
					error: 2,
					message: Some("AlreadyBonded")
				}
			);
			assert_noop!(
				Staking::bond(
					Origin::signed(unpaired_stash),
					10,
					StakingBalance::RingBalance(COIN),
					RewardDestination::Stash,
					0,
				),
				DispatchError::Module {
					index: 0,
					error: 3,
					message: Some("AlreadyPaired")
				}
			);
		});
	}

	// @darwinia(breakpoint)
	#[test]
	fn pool_should_be_increased_and_decreased_correctly() {
		ExtBuilder::default().build().execute_with(|| {
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
			Timestamp::set_timestamp(promise_month * MONTH_IN_MILLISECONDS);
			assert_ok!(Staking::unbond(
				Origin::signed(controller_2),
				StakingBalance::RingBalance(125 * COIN / 10),
			));
			ring_pool -= 125 * COIN / 10;
			assert_eq!(Staking::ring_pool(), ring_pool);

			// slash: 37.5Ring 50Kton
			<ErasStakers<Test>>::insert(
				0,
				&stash_1,
				Exposure {
					own_ring_balance: 1,
					total_power: 1,
					own_kton_balance: 0,
					own_power: 0,
					others: vec![],
				},
			);
			<ErasStakers<Test>>::insert(
				0,
				&stash_2,
				Exposure {
					own_ring_balance: 1,
					total_power: 1,
					own_kton_balance: 0,
					own_power: 0,
					others: vec![],
				},
			);

			// TODO: check slash_validator issue
			// // FIXME: slash strategy
			// let _ = Staking::slash_validator(&stash_1, Power::max_value(), &Staking::stakers(&stash_1), &mut vec![]);
			// // FIXME: slash strategy
			// let _ = Staking::slash_validator(&stash_2, Power::max_value(), &Staking::stakers(&stash_2), &mut vec![]);
			// ring_pool -= 375 * COIN / 10;
			// kton_pool -= 50 * COIN;
			// assert_eq!(Staking::ring_pool(), ring_pool);
			// assert_eq!(Staking::kton_pool(), kton_pool);
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

			// TODO: original is following error, we need check about this
			// err::UNLOCK_CHUNKS_REACH_MAX,
			// assert_ok!(Staking::unbond(
			// 	Origin::signed(controller),
			// 	StakingBalance::RingBalance(1)
			// ));
		});
	}

	#[test]
	fn promise_extra_should_not_remove_unexpired_items() {
		ExtBuilder::default().build().execute_with(|| {
			gen_paired_account!(stash(123), controller(456), promise_month(12));
			let expired_items_len = 3;
			let expiry_date = promise_month * MONTH_IN_MILLISECONDS;

			assert_ok!(Staking::bond_extra(
				Origin::signed(stash),
				StakingBalance::RingBalance(5 * COIN),
				0,
			));
			for _ in 0..expired_items_len {
				assert_ok!(Staking::deposit_extra(
					Origin::signed(stash),
					COIN,
					promise_month
				));
			}

			Timestamp::set_timestamp(expiry_date - 1);
			assert_ok!(Staking::deposit_extra(
				Origin::signed(stash),
				2 * COIN,
				promise_month,
			));
			assert_eq!(
				Staking::ledger(controller).unwrap().deposit_items.len(),
				2 + expired_items_len,
			);

			Timestamp::set_timestamp(expiry_date);
			assert_ok!(Staking::deposit_extra(
				Origin::signed(stash),
				2 * COIN,
				promise_month,
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

			assert_eq!(Kton::free_balance(&stash), 1);

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
				12 * MONTH_IN_MILLISECONDS,
			));
			assert_eq!(Kton::free_balance(&stash), 1);

			let ledger = Staking::ledger(controller).unwrap();

			// Please Note:
			// not enough Kton to unbond, but the function will not fail
			assert_ok!(Staking::try_claim_deposits_with_punish(
				Origin::signed(controller),
				36 * MONTH_IN_MILLISECONDS,
			));
			assert_eq!(Staking::ledger(controller).unwrap(), ledger);
		});
	}

	// Origin test case name is `yakio_q2`
	// how to balance the power and calculate the reward if some validators have been chilled
	// more reward with more validators
	#[test]
	fn nominator_voting_a_validator_before_he_chill() {
		fn run(with_new_era: bool) -> u128 {
			let mut balance = 0;
			ExtBuilder::default().build().execute_with(|| {
				gen_paired_account!(validator_1_stash(123), validator_1_controller(456), 0);
				gen_paired_account!(validator_2_stash(234), validator_2_controller(567), 0);
				gen_paired_account!(nominator_stash(345), nominator_controller(678), 0);

				assert_ok!(Staking::validate(
					Origin::signed(validator_1_controller),
					ValidatorPrefs::default(),
				));

				assert_ok!(Staking::validate(
					Origin::signed(validator_2_controller),
					ValidatorPrefs::default()
				));
				assert_ok!(Staking::nominate(
					Origin::signed(nominator_controller),
					vec![validator_1_stash, validator_2_stash],
				));

				start_era(1);

				// A validator becomes to be chilled after the nominator voting him
				assert_ok!(Staking::chill(Origin::signed(validator_1_controller)));
				assert_ok!(Staking::chill(Origin::signed(validator_2_controller)));
				if with_new_era {
					start_era(2);
				}

				// FIXME
				// let _ = Staking::reward_validator(&validator_1_stash, 1000 * COIN);
				// let _ = Staking::reward_validator(&validator_2_stash, 1000 * COIN);

				balance = Ring::free_balance(&nominator_stash);
			});

			balance
		}

		let free_balance = run(false);
		let free_balance_with_new_era = run(true);

		assert_ne!(free_balance, 0);
		assert_ne!(free_balance_with_new_era, 0);
		// assert!(free_balance > free_balance_with_new_era);
	}

	// @review(reward)
	// ~~TODO: fix BondingDuration issue,~~
	//// Original testcase name is `xavier_q1`
	//#[test]
	//fn staking_with_kton_with_unbondings() {
	//	ExtBuilder::default().build().execute_with(|| {
	//		let stash = 123;
	//		let controller = 456;
	//		let _ = Kton::deposit_creating(&stash, 10);

	//		Timestamp::set_timestamp(0);
	//		assert_ok!(Staking::bond(
	//			Origin::signed(stash),
	//			controller,
	//			StakingBalance::KtonBalance(5),
	//			RewardDestination::Stash,
	//			0,
	//		));
	//		assert_eq!(Timestamp::get(), 0);
	//		assert_eq!(Kton::free_balance(stash), 10);
	//		assert_eq!(
	//			Kton::locks(stash),
	//			vec![BalanceLock {
	//				id: STAKING_ID,
	//				withdraw_lock: WithdrawLock::WithStaking(StakingLock {
	//					staking_amount: 5,
	//					unbondings: vec![],
	//				}),
	//				reasons: WithdrawReasons::all(),
	//			}]
	//		);
	//		//		println!("Ok Init - Kton Balance: {:?}", Kton::free_balance(stash));
	//		//		println!("Ok Init - Kton Locks: {:#?}", Kton::locks(stash));
	//		//		println!();

	//		Timestamp::set_timestamp(1);
	//		assert_ok!(Staking::bond_extra(
	//			Origin::signed(stash),
	//			StakingBalance::KtonBalance(5),
	//			0
	//		));
	//		assert_eq!(Timestamp::get(), 1);
	//		assert_eq!(Kton::free_balance(stash), 10);
	//		assert_eq!(
	//			Kton::locks(stash),
	//			vec![BalanceLock {
	//				id: STAKING_ID,
	//				withdraw_lock: WithdrawLock::WithStaking(StakingLock {
	//					staking_amount: 10,
	//					unbondings: vec![],
	//				}),
	//				reasons: WithdrawReasons::all(),
	//			}]
	//		);
	//		//		println!("Ok Bond Extra - Kton Balance: {:?}", Kton::free_balance(stash));
	//		//		println!("Ok Bond Extra - Kton Locks: {:#?}", Kton::locks(stash));
	//		//		println!();

	//		let unbond_start = 2;
	//		Timestamp::set_timestamp(unbond_start);
	//		assert_ok!(Staking::unbond(
	//			Origin::signed(controller),
	//			StakingBalance::KtonBalance(9)
	//		));
	//		assert_eq!(Timestamp::get(), 2);
	//		assert_eq!(Kton::free_balance(stash), 10);
	//		assert_eq!(
	//			Kton::locks(stash),
	//			vec![BalanceLock {
	//				id: STAKING_ID,
	//				withdraw_lock: WithdrawLock::WithStaking(StakingLock {
	//					staking_amount: 1,
	//					unbondings: vec![NormalLock {
	//						amount: 9,
	//						until: BondingDuration::get() + unbond_start,
	//					}],
	//				}),
	//				reasons: WithdrawReasons::all(),
	//			}]
	//		);
	//		//		println!("Ok Unbond - Kton Balance: {:?}", Kton::free_balance(stash));
	//		//		println!("Ok Unbond - Kton Locks: {:#?}", Kton::locks(stash));
	//		//		println!();

	//		assert_err!(
	//			Kton::transfer(Origin::signed(stash), controller, 1),
	//			"account liquidity restrictions prevent withdrawal",
	//		);
	//		//		println!("Locking Transfer - Kton Balance: {:?}", Kton::free_balance(stash));
	//		//		println!("Locking Transfer - Kton Locks: {:#?}", Kton::locks(stash));
	//		//		println!();

	//		Timestamp::set_timestamp(BondingDuration::get() + unbond_start);
	//		assert_ok!(Kton::transfer(Origin::signed(stash), controller, 1));
	//		//		println!("Unlocking Transfer - Kton Balance: {:?}", Kton::free_balance(stash));
	//		//		println!("Unlocking Transfer - Kton Locks: {:#?}", Kton::locks(stash));
	//		//		println!(
	//		//			"Unlocking Transfer - Kton StakingLedger: {:#?}",
	//		//			Staking::ledger(controller)
	//		//		);
	//		//		println!();
	//		assert_eq!(Timestamp::get(), BondingDuration::get() + unbond_start);
	//		assert_eq!(Kton::free_balance(stash), 9);
	//		assert_eq!(
	//			Kton::locks(stash),
	//			vec![BalanceLock {
	//				id: STAKING_ID,
	//				withdraw_lock: WithdrawLock::WithStaking(StakingLock {
	//					staking_amount: 1,
	//					unbondings: vec![NormalLock {
	//						amount: 9,
	//						until: BondingDuration::get() + unbond_start,
	//					}],
	//				}),
	//				reasons: WithdrawReasons::all(),
	//			}]
	//		);

	//		let _ = Kton::deposit_creating(&stash, 20);
	//		assert_ok!(Staking::bond_extra(
	//			Origin::signed(stash),
	//			StakingBalance::KtonBalance(19),
	//			0
	//		));
	//		assert_eq!(Kton::free_balance(stash), 29);
	//		assert_eq!(
	//			Kton::locks(stash),
	//			vec![BalanceLock {
	//				id: STAKING_ID,
	//				withdraw_lock: WithdrawLock::WithStaking(StakingLock {
	//					staking_amount: 20,
	//					unbondings: vec![NormalLock {
	//						amount: 9,
	//						until: BondingDuration::get() + unbond_start,
	//					}],
	//				}),
	//				reasons: WithdrawReasons::all(),
	//			}]
	//		);
	//		assert_eq!(
	//			Staking::ledger(controller).unwrap(),
	//			StakingLedger {
	//				stash: 123,
	//				active_ring: 0,
	//				active_deposit_ring: 0,
	//				active_kton: 20,
	//				deposit_items: vec![],
	//				ring_staking_lock: Default::default(),
	//				kton_staking_lock: StakingLock {
	//					staking_amount: 20,
	//					unbondings: vec![NormalLock {
	//						amount: 9,
	//						until: BondingDuration::get() + unbond_start,
	//					}],
	//				},
	//			}
	//		);
	//		//		println!("Unlocking Transfer - Kton Balance: {:?}", Kton::free_balance(stash));
	//		//		println!("Unlocking Transfer - Kton Locks: {:#?}", Kton::locks(stash));
	//		//		println!(
	//		//			"Unlocking Transfer - Kton StakingLedger: {:#?}",
	//		//			Staking::ledger(controller)
	//		//		);
	//		//		println!();
	//	});

	//	ExtBuilder::default().build().execute_with(|| {
	//		let stash = 123;
	//		let controller = 456;
	//		let _ = Ring::deposit_creating(&stash, 10);

	//		Timestamp::set_timestamp(0);
	//		assert_ok!(Staking::bond(
	//			Origin::signed(stash),
	//			controller,
	//			StakingBalance::RingBalance(5),
	//			RewardDestination::Stash,
	//			0,
	//		));
	//		assert_eq!(Timestamp::get(), 0);
	//		assert_eq!(Ring::free_balance(stash), 10);
	//		assert_eq!(
	//			Ring::locks(stash),
	//			vec![BalanceLock {
	//				id: STAKING_ID,
	//				withdraw_lock: WithdrawLock::WithStaking(StakingLock {
	//					staking_amount: 5,
	//					unbondings: vec![],
	//				}),
	//				reasons: WithdrawReasons::all(),
	//			}]
	//		);
	//		//		println!("Ok Init - Ring Balance: {:?}", Ring::free_balance(stash));
	//		//		println!("Ok Init - Ring Locks: {:#?}", Ring::locks(stash));
	//		//		println!();

	//		Timestamp::set_timestamp(1);
	//		assert_ok!(Staking::bond_extra(
	//			Origin::signed(stash),
	//			StakingBalance::RingBalance(5),
	//			0
	//		));
	//		assert_eq!(Timestamp::get(), 1);
	//		assert_eq!(Ring::free_balance(stash), 10);
	//		assert_eq!(
	//			Ring::locks(stash),
	//			vec![BalanceLock {
	//				id: STAKING_ID,
	//				withdraw_lock: WithdrawLock::WithStaking(StakingLock {
	//					staking_amount: 10,
	//					unbondings: vec![],
	//				}),
	//				reasons: WithdrawReasons::all(),
	//			}]
	//		);
	//		//		println!("Ok Bond Extra - Ring Balance: {:?}", Ring::free_balance(stash));
	//		//		println!("Ok Bond Extra - Ring Locks: {:#?}", Ring::locks(stash));
	//		//		println!();

	//		let unbond_start = 2;
	//		Timestamp::set_timestamp(unbond_start);
	//		assert_ok!(Staking::unbond(
	//			Origin::signed(controller),
	//			StakingBalance::RingBalance(9)
	//		));
	//		assert_eq!(Timestamp::get(), 2);
	//		assert_eq!(Ring::free_balance(stash), 10);
	//		assert_eq!(
	//			Ring::locks(stash),
	//			vec![BalanceLock {
	//				id: STAKING_ID,
	//				withdraw_lock: WithdrawLock::WithStaking(StakingLock {
	//					staking_amount: 1,
	//					unbondings: vec![NormalLock {
	//						amount: 9,
	//						until: BondingDuration::get() + unbond_start,
	//					}],
	//				}),
	//				reasons: WithdrawReasons::all(),
	//			}]
	//		);
	//		//		println!("Ok Unbond - Ring Balance: {:?}", Ring::free_balance(stash));
	//		//		println!("Ok Unbond - Ring Locks: {:#?}", Ring::locks(stash));
	//		//		println!();

	//		assert_err!(
	//			Ring::transfer(Origin::signed(stash), controller, 1),
	//			"account liquidity restrictions prevent withdrawal",
	//		);
	//		//		println!("Locking Transfer - Ring Balance: {:?}", Ring::free_balance(stash));
	//		//		println!("Locking Transfer - Ring Locks: {:#?}", Ring::locks(stash));
	//		//		println!();

	//		Timestamp::set_timestamp(BondingDuration::get() + unbond_start);
	//		assert_ok!(Ring::transfer(Origin::signed(stash), controller, 1));
	//		//		println!("Unlocking Transfer - Ring Balance: {:?}", Ring::free_balance(stash));
	//		//		println!("Unlocking Transfer - Ring Locks: {:#?}", Ring::locks(stash));
	//		//		println!(
	//		//			"Unlocking Transfer - Ring StakingLedger: {:#?}",
	//		//			Staking::ledger(controller)
	//		//		);
	//		//		println!();
	//		assert_eq!(Timestamp::get(), BondingDuration::get() + unbond_start);
	//		assert_eq!(Ring::free_balance(stash), 9);
	//		assert_eq!(
	//			Ring::locks(stash),
	//			vec![BalanceLock {
	//				id: STAKING_ID,
	//				withdraw_lock: WithdrawLock::WithStaking(StakingLock {
	//					staking_amount: 1,
	//					unbondings: vec![NormalLock {
	//						amount: 9,
	//						until: BondingDuration::get() + unbond_start,
	//					}],
	//				}),
	//				reasons: WithdrawReasons::all(),
	//			}]
	//		);

	//		let _ = Ring::deposit_creating(&stash, 20);
	//		assert_ok!(Staking::bond_extra(
	//			Origin::signed(stash),
	//			StakingBalance::RingBalance(19),
	//			0
	//		));
	//		assert_eq!(Ring::free_balance(stash), 29);
	//		assert_eq!(
	//			Ring::locks(stash),
	//			vec![BalanceLock {
	//				id: STAKING_ID,
	//				withdraw_lock: WithdrawLock::WithStaking(StakingLock {
	//					staking_amount: 20,
	//					unbondings: vec![NormalLock {
	//						amount: 9,
	//						until: BondingDuration::get() + unbond_start,
	//					}],
	//				}),
	//				reasons: WithdrawReasons::all(),
	//			}]
	//		);
	//		assert_eq!(
	//			Staking::ledger(controller).unwrap(),
	//			StakingLedger {
	//				stash: 123,
	//				active_ring: 20,
	//				active_deposit_ring: 0,
	//				active_kton: 0,
	//				deposit_items: vec![],
	//				ring_staking_lock: StakingLock {
	//					staking_amount: 20,
	//					unbondings: vec![NormalLock {
	//						amount: 9,
	//						until: BondingDuration::get() + unbond_start,
	//					}],
	//				},
	//				kton_staking_lock: Default::default(),
	//			}
	//		);
	//		//		println!("Unlocking Transfer - Ring Balance: {:?}", Ring::free_balance(stash));
	//		//		println!("Unlocking Transfer - Ring Locks: {:#?}", Ring::locks(stash));
	//		//		println!(
	//		//			"Unlocking Transfer - Ring StakingLedger: {:#?}",
	//		//			Staking::ledger(controller)
	//		//		);
	//		//		println!();
	//	});
	//}
	//
	// @review(reward)
	// ~~TODO: fix BondingDuration issue,~~
	//// Original testcase name is `xavier_q2`
	////
	//// The values(KTON, RING) are unbond twice with different amount and times
	//#[test]
	//fn unbound_values_in_twice() {
	//	ExtBuilder::default().build().execute_with(|| {
	//		let stash = 123;
	//		let controller = 456;
	//		let _ = Kton::deposit_creating(&stash, 10);

	//		Timestamp::set_timestamp(1);
	//		assert_ok!(Staking::bond(
	//			Origin::signed(stash),
	//			controller,
	//			StakingBalance::KtonBalance(5),
	//			RewardDestination::Stash,
	//			0,
	//		));
	//		assert_eq!(Kton::free_balance(stash), 10);
	//		assert_eq!(
	//			Kton::locks(stash),
	//			vec![BalanceLock {
	//				id: STAKING_ID,
	//				withdraw_lock: WithdrawLock::WithStaking(StakingLock {
	//					staking_amount: 5,
	//					unbondings: vec![],
	//				}),
	//				reasons: WithdrawReasons::all(),
	//			}]
	//		);
	//		//		println!("Ok Init - Kton Balance: {:?}", Kton::free_balance(stash));
	//		//		println!("Ok Init - Kton Locks: {:#?}", Kton::locks(stash));
	//		//		println!();

	//		Timestamp::set_timestamp(1);
	//		assert_ok!(Staking::bond_extra(
	//			Origin::signed(stash),
	//			StakingBalance::KtonBalance(4),
	//			0
	//		));
	//		assert_eq!(Timestamp::get(), 1);
	//		assert_eq!(Kton::free_balance(stash), 10);
	//		assert_eq!(
	//			Kton::locks(stash),
	//			vec![BalanceLock {
	//				id: STAKING_ID,
	//				withdraw_lock: WithdrawLock::WithStaking(StakingLock {
	//					staking_amount: 9,
	//					unbondings: vec![],
	//				}),
	//				reasons: WithdrawReasons::all(),
	//			}]
	//		);
	//		//		println!("Ok Bond Extra - Kton Balance: {:?}", Kton::free_balance(stash));
	//		//		println!("Ok Bond Extra - Kton Locks: {:#?}", Kton::locks(stash));
	//		//		println!();

	//		let (unbond_start_1, unbond_value_1) = (2, 2);
	//		Timestamp::set_timestamp(unbond_start_1);
	//		assert_ok!(Staking::unbond(
	//			Origin::signed(controller),
	//			StakingBalance::KtonBalance(unbond_value_1),
	//		));
	//		assert_eq!(Timestamp::get(), unbond_start_1);
	//		assert_eq!(Kton::free_balance(stash), 10);
	//		assert_eq!(
	//			Kton::locks(stash),
	//			vec![BalanceLock {
	//				id: STAKING_ID,
	//				withdraw_lock: WithdrawLock::WithStaking(StakingLock {
	//					staking_amount: 7,
	//					unbondings: vec![NormalLock {
	//						amount: 2,
	//						until: BondingDuration::get() + unbond_start_1,
	//					}],
	//				}),
	//				reasons: WithdrawReasons::all(),
	//			}]
	//		);
	//		//		println!("Ok Unbond - Kton Balance: {:?}", Kton::free_balance(stash));
	//		//		println!("Ok Unbond - Kton Locks: {:#?}", Kton::locks(stash));
	//		//		println!();

	//		let (unbond_start_2, unbond_value_2) = (3, 6);
	//		Timestamp::set_timestamp(unbond_start_2);
	//		assert_ok!(Staking::unbond(
	//			Origin::signed(controller),
	//			StakingBalance::KtonBalance(6)
	//		));
	//		assert_eq!(Timestamp::get(), unbond_start_2);
	//		assert_eq!(Kton::free_balance(stash), 10);
	//		assert_eq!(
	//			Kton::locks(stash),
	//			vec![BalanceLock {
	//				id: STAKING_ID,
	//				withdraw_lock: WithdrawLock::WithStaking(StakingLock {
	//					staking_amount: 1,
	//					unbondings: vec![
	//						NormalLock {
	//							amount: 2,
	//							until: BondingDuration::get() + unbond_start_1,
	//						},
	//						NormalLock {
	//							amount: 6,
	//							until: BondingDuration::get() + unbond_start_2,
	//						}
	//					],
	//				}),
	//				reasons: WithdrawReasons::all(),
	//			}]
	//		);
	//		//		println!("Ok Unbond - Kton Balance: {:?}", Kton::free_balance(stash));
	//		//		println!("Ok Unbond - Kton Locks: {:#?}", Kton::locks(stash));
	//		//		println!();

	//		assert_err!(
	//			Kton::transfer(Origin::signed(stash), controller, unbond_value_1),
	//			"account liquidity restrictions prevent withdrawal",
	//		);
	//		//		println!("Locking Transfer - Kton Balance: {:?}", Kton::free_balance(stash));
	//		//		println!("Locking Transfer - Kton Locks: {:#?}", Kton::locks(stash));
	//		//		println!();

	//		assert_ok!(Kton::transfer(Origin::signed(stash), controller, unbond_value_1 - 1));
	//		assert_eq!(Kton::free_balance(stash), 9);
	//		//		println!("Normal Transfer - Kton Balance: {:?}", Kton::free_balance(stash));
	//		//		println!("Normal Transfer - Kton Locks: {:#?}", Kton::locks(stash));

	//		Timestamp::set_timestamp(BondingDuration::get() + unbond_start_1);
	//		assert_err!(
	//			Kton::transfer(Origin::signed(stash), controller, unbond_value_1 + 1),
	//			"account liquidity restrictions prevent withdrawal",
	//		);
	//		//		println!("Locking Transfer - Kton Balance: {:?}", Kton::free_balance(stash));
	//		//		println!("Locking Transfer - Kton Locks: {:#?}", Kton::locks(stash));
	//		//		println!();
	//		assert_ok!(Kton::transfer(Origin::signed(stash), controller, unbond_value_1));
	//		assert_eq!(Timestamp::get(), BondingDuration::get() + unbond_start_1);
	//		assert_eq!(Kton::free_balance(stash), 7);
	//		assert_eq!(
	//			Kton::locks(stash),
	//			vec![BalanceLock {
	//				id: STAKING_ID,
	//				withdraw_lock: WithdrawLock::WithStaking(StakingLock {
	//					staking_amount: 1,
	//					unbondings: vec![
	//						NormalLock {
	//							amount: 2,
	//							until: BondingDuration::get() + unbond_start_1,
	//						},
	//						NormalLock {
	//							amount: 6,
	//							until: BondingDuration::get() + unbond_start_2,
	//						}
	//					],
	//				}),
	//				reasons: WithdrawReasons::all(),
	//			}]
	//		);
	//		//		println!("Unlocking Transfer - Kton Balance: {:?}", Kton::free_balance(stash));
	//		//		println!("Unlocking Transfer - Kton Locks: {:#?}", Kton::locks(stash));

	//		Timestamp::set_timestamp(BondingDuration::get() + unbond_start_2);
	//		assert_ok!(Kton::transfer(Origin::signed(stash), controller, unbond_value_2));
	//		assert_eq!(Timestamp::get(), BondingDuration::get() + unbond_start_2);
	//		assert_eq!(Kton::free_balance(stash), 1);
	//		assert_eq!(
	//			Kton::locks(stash),
	//			vec![BalanceLock {
	//				id: STAKING_ID,
	//				withdraw_lock: WithdrawLock::WithStaking(StakingLock {
	//					staking_amount: 1,
	//					unbondings: vec![
	//						NormalLock {
	//							amount: 2,
	//							until: BondingDuration::get() + unbond_start_1,
	//						},
	//						NormalLock {
	//							amount: 6,
	//							until: BondingDuration::get() + unbond_start_2,
	//						}
	//					],
	//				}),
	//				reasons: WithdrawReasons::all(),
	//			}]
	//		);
	//		//		println!("Unlocking Transfer - Kton Balance: {:?}", Kton::free_balance(stash));
	//		//		println!("Unlocking Transfer - Kton Locks: {:#?}", Kton::locks(stash));

	//		let _ = Kton::deposit_creating(&stash, 1);
	//		//		println!("Staking Ledger: {:#?}", Staking::ledger(controller).unwrap());
	//		assert_eq!(Kton::free_balance(stash), 2);
	//		assert_ok!(Staking::bond_extra(
	//			Origin::signed(stash),
	//			StakingBalance::KtonBalance(1),
	//			0
	//		));
	//		assert_eq!(
	//			Kton::locks(stash),
	//			vec![BalanceLock {
	//				id: STAKING_ID,
	//				withdraw_lock: WithdrawLock::WithStaking(StakingLock {
	//					staking_amount: 2,
	//					unbondings: vec![
	//						NormalLock {
	//							amount: 2,
	//							until: BondingDuration::get() + unbond_start_1,
	//						},
	//						NormalLock {
	//							amount: 6,
	//							until: BondingDuration::get() + unbond_start_2,
	//						}
	//					],
	//				}),
	//				reasons: WithdrawReasons::all(),
	//			}]
	//		);
	//	});

	//	ExtBuilder::default().build().execute_with(|| {
	//		let stash = 123;
	//		let controller = 456;
	//		let _ = Ring::deposit_creating(&stash, 10);

	//		Timestamp::set_timestamp(1);
	//		assert_ok!(Staking::bond(
	//			Origin::signed(stash),
	//			controller,
	//			StakingBalance::RingBalance(5),
	//			RewardDestination::Stash,
	//			0,
	//		));
	//		assert_eq!(Ring::free_balance(stash), 10);
	//		assert_eq!(
	//			Ring::locks(stash),
	//			vec![BalanceLock {
	//				id: STAKING_ID,
	//				withdraw_lock: WithdrawLock::WithStaking(StakingLock {
	//					staking_amount: 5,
	//					unbondings: vec![],
	//				}),
	//				reasons: WithdrawReasons::all(),
	//			}]
	//		);
	//		//		println!("Ok Init - Ring Balance: {:?}", Ring::free_balance(stash));
	//		//		println!("Ok Init - Ring Locks: {:#?}", Ring::locks(stash));
	//		//		println!();

	//		Timestamp::set_timestamp(1);
	//		assert_ok!(Staking::bond_extra(
	//			Origin::signed(stash),
	//			StakingBalance::RingBalance(4),
	//			0
	//		));
	//		assert_eq!(Timestamp::get(), 1);
	//		assert_eq!(Ring::free_balance(stash), 10);
	//		assert_eq!(
	//			Ring::locks(stash),
	//			vec![BalanceLock {
	//				id: STAKING_ID,
	//				withdraw_lock: WithdrawLock::WithStaking(StakingLock {
	//					staking_amount: 9,
	//					unbondings: vec![],
	//				}),
	//				reasons: WithdrawReasons::all(),
	//			}]
	//		);
	//		//		println!("Ok Bond Extra - Ring Balance: {:?}", Ring::free_balance(stash));
	//		//		println!("Ok Bond Extra - Ring Locks: {:#?}", Ring::locks(stash));
	//		//		println!();

	//		let (unbond_start_1, unbond_value_1) = (2, 2);
	//		Timestamp::set_timestamp(unbond_start_1);
	//		assert_ok!(Staking::unbond(
	//			Origin::signed(controller),
	//			StakingBalance::RingBalance(unbond_value_1)
	//		));
	//		assert_eq!(Timestamp::get(), unbond_start_1);
	//		assert_eq!(Ring::free_balance(stash), 10);
	//		assert_eq!(
	//			Ring::locks(stash),
	//			vec![BalanceLock {
	//				id: STAKING_ID,
	//				withdraw_lock: WithdrawLock::WithStaking(StakingLock {
	//					staking_amount: 7,
	//					unbondings: vec![NormalLock {
	//						amount: 2,
	//						until: BondingDuration::get() + unbond_start_1,
	//					}],
	//				}),
	//				reasons: WithdrawReasons::all(),
	//			}]
	//		);
	//		//		println!("Ok Unbond - Ring Balance: {:?}", Ring::free_balance(stash));
	//		//		println!("Ok Unbond - Ring Locks: {:#?}", Ring::locks(stash));
	//		//		println!();

	//		let (unbond_start_2, unbond_value_2) = (3, 6);
	//		Timestamp::set_timestamp(unbond_start_2);
	//		assert_ok!(Staking::unbond(
	//			Origin::signed(controller),
	//			StakingBalance::RingBalance(6)
	//		));
	//		assert_eq!(Timestamp::get(), unbond_start_2);
	//		assert_eq!(Ring::free_balance(stash), 10);
	//		assert_eq!(
	//			Ring::locks(stash),
	//			vec![BalanceLock {
	//				id: STAKING_ID,
	//				withdraw_lock: WithdrawLock::WithStaking(StakingLock {
	//					staking_amount: 1,
	//					unbondings: vec![
	//						NormalLock {
	//							amount: 2,
	//							until: BondingDuration::get() + unbond_start_1,
	//						},
	//						NormalLock {
	//							amount: 6,
	//							until: BondingDuration::get() + unbond_start_2,
	//						}
	//					],
	//				}),
	//				reasons: WithdrawReasons::all(),
	//			}]
	//		);
	//		//		println!("Ok Unbond - Ring Balance: {:?}", Ring::free_balance(stash));
	//		//		println!("Ok Unbond - Ring Locks: {:#?}", Ring::locks(stash));
	//		//		println!();

	//		assert_err!(
	//			Ring::transfer(Origin::signed(stash), controller, unbond_value_1),
	//			"account liquidity restrictions prevent withdrawal",
	//		);
	//		//		println!("Locking Transfer - Ring Balance: {:?}", Ring::free_balance(stash));
	//		//		println!("Locking Transfer - Ring Locks: {:#?}", Ring::locks(stash));
	//		//		println!();

	//		assert_ok!(Ring::transfer(Origin::signed(stash), controller, unbond_value_1 - 1));
	//		assert_eq!(Ring::free_balance(stash), 9);
	//		//		println!("Normal Transfer - Ring Balance: {:?}", Ring::free_balance(stash));
	//		//		println!("Normal Transfer - Ring Locks: {:#?}", Ring::locks(stash));

	//		Timestamp::set_timestamp(BondingDuration::get() + unbond_start_1);
	//		assert_err!(
	//			Ring::transfer(Origin::signed(stash), controller, unbond_value_1 + 1),
	//			"account liquidity restrictions prevent withdrawal",
	//		);
	//		//		println!("Locking Transfer - Ring Balance: {:?}", Ring::free_balance(stash));
	//		//		println!("Locking Transfer - Ring Locks: {:#?}", Ring::locks(stash));
	//		//		println!();
	//		assert_ok!(Ring::transfer(Origin::signed(stash), controller, unbond_value_1));
	//		assert_eq!(Timestamp::get(), BondingDuration::get() + unbond_start_1);
	//		assert_eq!(Ring::free_balance(stash), 7);
	//		assert_eq!(
	//			Ring::locks(stash),
	//			vec![BalanceLock {
	//				id: STAKING_ID,
	//				withdraw_lock: WithdrawLock::WithStaking(StakingLock {
	//					staking_amount: 1,
	//					unbondings: vec![
	//						NormalLock {
	//							amount: 2,
	//							until: BondingDuration::get() + unbond_start_1,
	//						},
	//						NormalLock {
	//							amount: 6,
	//							until: BondingDuration::get() + unbond_start_2,
	//						}
	//					],
	//				}),
	//				reasons: WithdrawReasons::all(),
	//			}]
	//		);
	//		//		println!("Unlocking Transfer - Ring Balance: {:?}", Ring::free_balance(stash));
	//		//		println!("Unlocking Transfer - Ring Locks: {:#?}", Ring::locks(stash));

	//		Timestamp::set_timestamp(BondingDuration::get() + unbond_start_2);
	//		assert_ok!(Ring::transfer(Origin::signed(stash), controller, unbond_value_2));
	//		assert_eq!(Timestamp::get(), BondingDuration::get() + unbond_start_2);
	//		assert_eq!(Ring::free_balance(stash), 1);
	//		assert_eq!(
	//			Ring::locks(stash),
	//			vec![BalanceLock {
	//				id: STAKING_ID,
	//				withdraw_lock: WithdrawLock::WithStaking(StakingLock {
	//					staking_amount: 1,
	//					unbondings: vec![
	//						NormalLock {
	//							amount: 2,
	//							until: BondingDuration::get() + unbond_start_1,
	//						},
	//						NormalLock {
	//							amount: 6,
	//							until: BondingDuration::get() + unbond_start_2,
	//						}
	//					],
	//				}),
	//				reasons: WithdrawReasons::all(),
	//			}]
	//		);
	//		//		println!("Unlocking Transfer - Ring Balance: {:?}", Ring::free_balance(stash));
	//		//		println!("Unlocking Transfer - Ring Locks: {:#?}", Ring::locks(stash));

	//		let _ = Ring::deposit_creating(&stash, 1);
	//		//		println!("Staking Ledger: {:#?}", Staking::ledger(controller).unwrap());
	//		assert_eq!(Ring::free_balance(stash), 2);
	//		assert_ok!(Staking::bond_extra(
	//			Origin::signed(stash),
	//			StakingBalance::RingBalance(1),
	//			0
	//		));
	//		assert_eq!(
	//			Ring::locks(stash),
	//			vec![BalanceLock {
	//				id: STAKING_ID,
	//				withdraw_lock: WithdrawLock::WithStaking(StakingLock {
	//					staking_amount: 2,
	//					unbondings: vec![
	//						NormalLock {
	//							amount: 2,
	//							until: BondingDuration::get() + unbond_start_1,
	//						},
	//						NormalLock {
	//							amount: 6,
	//							until: BondingDuration::get() + unbond_start_2,
	//						}
	//					],
	//				}),
	//				reasons: WithdrawReasons::all(),
	//			}]
	//		);
	//	});
	//}

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

			Timestamp::set_timestamp(1);
			assert_ok!(Staking::bond(
				Origin::signed(stash),
				controller,
				StakingBalance::KtonBalance(5),
				RewardDestination::Stash,
				0,
			));

			assert_eq!(Timestamp::get(), 1);
			assert_eq!(
				Staking::ledger(controller).unwrap(),
				StakingLedger {
					stash: 123,
					active_ring: 0,
					active_deposit_ring: 0,
					active_kton: 5,
					deposit_items: vec![],
					ring_staking_lock: Default::default(),
					kton_staking_lock: StakingLock {
						staking_amount: 5,
						unbondings: vec![],
					},
					last_reward: Some(0),
				},
			);

			// all values are unbond
			assert_ok!(Staking::unbond(
				Origin::signed(controller),
				StakingBalance::KtonBalance(5)
			));
			assert_eq!(Staking::ledger(controller), None);

			// bond again
			Timestamp::set_timestamp(61);
			assert_ok!(Staking::bond(
				Origin::signed(stash),
				controller,
				StakingBalance::KtonBalance(1),
				RewardDestination::Stash,
				0,
			));
			assert_eq!(Timestamp::get(), 61);
			assert_eq!(
				Staking::ledger(controller).unwrap(),
				StakingLedger {
					stash: 123,
					active_ring: 0,
					active_deposit_ring: 0,
					active_kton: 1,
					deposit_items: vec![],
					ring_staking_lock: Default::default(),
					kton_staking_lock: StakingLock {
						staking_amount: 1,
						unbondings: vec![],
					},
					last_reward: Some(0),
				},
			);
		});

		// The Ring part
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
			assert_eq!(Timestamp::get(), 1);
			assert_eq!(
				Staking::ledger(controller).unwrap(),
				StakingLedger {
					stash: 123,
					active_ring: 5,
					active_deposit_ring: 0,
					active_kton: 0,
					deposit_items: vec![],
					ring_staking_lock: StakingLock {
						staking_amount: 5,
						unbondings: vec![],
					},
					kton_staking_lock: Default::default(),
					last_reward: Some(0),
				},
			);

			// all values are unbond
			assert_ok!(Staking::unbond(
				Origin::signed(controller),
				StakingBalance::RingBalance(5),
			));
			assert_eq!(Staking::ledger(controller), None);

			// bond again
			Timestamp::set_timestamp(61);
			assert_ok!(Staking::bond(
				Origin::signed(stash),
				controller,
				StakingBalance::RingBalance(1),
				RewardDestination::Stash,
				0,
			));
			assert_eq!(Timestamp::get(), 61);
			assert_eq!(
				Staking::ledger(controller).unwrap(),
				StakingLedger {
					stash: 123,
					active_ring: 1,
					active_deposit_ring: 0,
					active_kton: 0,
					deposit_items: vec![],
					ring_staking_lock: StakingLock {
						staking_amount: 1,
						unbondings: vec![],
					},
					kton_staking_lock: Default::default(),
					last_reward: Some(0),
				}
			);
		});
	}

	// @darwinia(breakpoint): keep annotated is fine.
	// #[test]
	// fn xavier_q4() {
	// 	ExtBuilder::default().build().execute_with(|| {
	// 		let (stash, _controller) = (11, 10);
	// 		let _ = Kton::deposit_creating(&stash, 1000);
	// 		assert_ok!(Staking::bond_extra(
	// 			Origin::signed(stash),
	// 			StakingBalance::KtonBalance(1000),
	// 			0,
	// 		));
	//
	// 		let power = Staking::power_of(&11);
	// 		<Stakers<Test>>::insert(
	// 			&stash,
	// 			Exposure {
	// 				total: power,
	// 				own: power,
	// 				others: vec![],
	// 			},
	// 		);
	// 		let _ = Staking::slash_validator(&stash, power / 2, &Staking::stakers(&stash), &mut vec![]);
	// 	});
	// }
}
