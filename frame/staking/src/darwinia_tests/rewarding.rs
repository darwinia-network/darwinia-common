// --- paritytech ---
use frame_support::{assert_ok, traits::Currency};
use sp_runtime::traits::Zero;
// --- darwinia-network ---
use crate::{mock::*, *};

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
