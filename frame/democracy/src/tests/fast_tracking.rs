//! The tests for fast-tracking functionality.

use super::*;

#[test]
fn fast_track_referendum_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(0);
		let h = set_balance_proposal_hash_and_note(2);
		assert_noop!(
			Democracy::fast_track(Origin::signed(5), h, 3, 2),
			Error::<Test>::ProposalMissing
		);
		assert_ok!(Democracy::external_propose_majority(
			Origin::signed(3),
			set_balance_proposal_hash_and_note(2)
		));
		assert_noop!(Democracy::fast_track(Origin::signed(1), h, 3, 2), BadOrigin);
		assert_ok!(Democracy::fast_track(Origin::signed(5), h, 2, 0));
		assert_eq!(
			Democracy::referendum_status(0),
			Ok(ReferendumStatus {
				end: 2,
				proposal_hash: set_balance_proposal_hash_and_note(2),
				threshold: VoteThreshold::SimpleMajority,
				delay: 0,
				tally: Tally { ayes: 0, nays: 0, turnout: 0 },
			})
		);
	});
}

#[test]
fn instant_referendum_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(0);
		let h = set_balance_proposal_hash_and_note(2);
		assert_noop!(
			Democracy::fast_track(Origin::signed(5), h, 3, 2),
			Error::<Test>::ProposalMissing
		);
		assert_ok!(Democracy::external_propose_majority(
			Origin::signed(3),
			set_balance_proposal_hash_and_note(2)
		));
		assert_noop!(Democracy::fast_track(Origin::signed(1), h, 3, 2), BadOrigin);
		assert_noop!(Democracy::fast_track(Origin::signed(5), h, 1, 0), BadOrigin);
		assert_noop!(
			Democracy::fast_track(Origin::signed(6), h, 1, 0),
			Error::<Test>::InstantNotAllowed
		);
		INSTANT_ALLOWED.with(|v| *v.borrow_mut() = true);
		assert_ok!(Democracy::fast_track(Origin::signed(6), h, 1, 0));
		assert_eq!(
			Democracy::referendum_status(0),
			Ok(ReferendumStatus {
				end: 1,
				proposal_hash: set_balance_proposal_hash_and_note(2),
				threshold: VoteThreshold::SimpleMajority,
				delay: 0,
				tally: Tally { ayes: 0, nays: 0, turnout: 0 },
			})
		);
	});
}

#[test]
fn fast_track_referendum_fails_when_no_simple_majority() {
	new_test_ext().execute_with(|| {
		System::set_block_number(0);
		let h = set_balance_proposal_hash_and_note(2);
		assert_ok!(Democracy::external_propose(
			Origin::signed(2),
			set_balance_proposal_hash_and_note(2)
		));
		assert_noop!(
			Democracy::fast_track(Origin::signed(5), h, 3, 2),
			Error::<Test>::NotSimpleMajority
		);
	});
}
