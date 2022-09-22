// --- paritytech ---
use frame_support::{assert_ok, traits::Currency, WeakBoundedVec};
use sp_runtime::Perbill;
use sp_staking::offence::OffenceDetails;
use substrate_test_utils::assert_eq_uvec;
// --- darwinia-network ---
use crate::{mock::*, *};
use darwinia_support::balance::*;

#[test]
fn slash_ledger_should_work() {
	ExtBuilder::default().nominate(false).validator_count(1).build().execute_with(|| {
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
		assert_ok!(Staking::deposit_extra(Origin::signed(account_id), COIN * 80 / 100, 36));
		assert_ok!(Staking::validate(Origin::signed(account_id), ValidatorPrefs::default()));
		assert_ok!(Session::set_keys(Origin::signed(account_id), SessionKeys { other: account_id.into() }, Vec::new()));


		start_active_era(1);

		assert_eq_uvec!(validator_controllers(), vec![777]);

		on_offence_now(
			&[OffenceDetails {
				offender: (account_id, Staking::eras_stakers(active_era(), account_id)),
				reporters: Vec::new(),
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
					ring_staking_lock: StakingLock { staking_amount: 0, ..Default::default() },
					..Default::default()
				},
			);
		}

		let ledger = Staking::ledger(&account_id).unwrap();

		// Should not overflow here
		assert_ok!(Staking::unbond(Origin::signed(account_id), StakingBalance::RingBalance(1)));

		assert_eq!(ledger, Staking::ledger(&account_id).unwrap());
	});
}

#[test]
fn slash_also_slash_unbondings() {
	ExtBuilder::default().validator_count(1).build().execute_with(|| {
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
		assert_ok!(Staking::validate(Origin::signed(account_id), ValidatorPrefs::default()));
		assert_ok!(Session::set_keys(Origin::signed(account_id), SessionKeys { other: account_id.into() }, Vec::new()));

		let mut ring_staking_lock = Staking::ledger(account_id).unwrap().ring_staking_lock.clone();

		start_active_era(1);

		assert_ok!(Staking::unbond(
			Origin::signed(account_id),
			StakingBalance::RingBalance(COIN / 2)
		));

		assert_eq_uvec!(validator_controllers(), vec![777]);

		on_offence_now(
			&[OffenceDetails {
				offender: (account_id, Staking::eras_stakers(active_era(), account_id)),
				reporters: Vec::new(),
			}],
			&[Perbill::from_percent(100)],
		);

		ring_staking_lock.staking_amount = 0;
		ring_staking_lock.unbondings = WeakBoundedVec::force_from(Vec::new(), None);

		assert_eq!(Staking::ledger(account_id).unwrap().ring_staking_lock, ring_staking_lock);
	});
}
