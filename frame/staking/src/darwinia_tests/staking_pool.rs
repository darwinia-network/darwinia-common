// --- paritytech ---
use frame_support::{assert_ok, traits::Currency};
use sp_runtime::Perbill;
use sp_staking::offence::OffenceDetails;
use substrate_test_utils::assert_eq_uvec;
// --- darwinia-network ---
use crate::{mock::*, *};

#[test]
fn pool_should_be_increased_and_decreased_correctly() {
	ExtBuilder::default().min_validator_bond(0).build().execute_with(|| {
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
		assert_ok!(Staking::validate(Origin::signed(controller_1), ValidatorPrefs::default()));
		assert_ok!(Staking::validate(Origin::signed(controller_2), ValidatorPrefs::default()));

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

	ExtBuilder::default().has_stakers(false).build_and_execute(|| {
		bond_validator(11, 10, StakingBalance::RingBalance(1000));
		assert_ok!(Staking::set_payee(Origin::signed(10), RewardDestination::Staked));

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
