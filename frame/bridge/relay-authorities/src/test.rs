// --- substrate ---
use frame_support::{assert_err, assert_ok};
// --- darwinia ---
use crate::{
	mock::{AccountId, *},
	*,
};

fn request_authority(account_id: AccountId) -> DispatchResult {
	RelayAuthorities::request_authority(Origin::signed(account_id), 1, [0; 20])
}

#[test]
fn duplicate_request_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(request_authority(1));
		// Already in candidates
		assert_err!(request_authority(1), RelayAuthoritiesError::CandidateAE);

		assert_ok!(RelayAuthorities::add_authority(Origin::root(), 1));

		// Already is authority
		assert_err!(request_authority(1), RelayAuthoritiesError::AuthorityAE);
	});
}
