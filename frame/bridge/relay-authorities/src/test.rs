// --- substrate ---
use frame_support::{assert_err, assert_ok};
// --- darwinia ---
use crate::{
	mock::{AccountId, BlockNumber, *},
	*,
};

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

#[test]
fn insufficient_stake_should_fail() {
	new_test_ext().execute_with(|| {
		assert_err!(request_authority(0), RelayAuthoritiesError::StakeIns);

		let max_candidates = <MaxCandidates as Get<usize>>::get() as _;

		for i in 1..=max_candidates {
			assert_ok!(request_authority_with_stake(i, i as Balance * 10));
		}

		// The minimum stake around candidates is 10 and the queue is full
		let _ = Ring::deposit_creating(&123, 1);
		assert_err!(request_authority(123), RelayAuthoritiesError::StakeIns);

		for i in 1..=max_candidates {
			assert!(RelayAuthorities::candidates()
				.iter()
				.position(|candidate| candidate == &i)
				.is_some());
		}

		// Increase the stake to run for the candidates seat
		let _ = Ring::deposit_creating(&123, 11);
		assert_ok!(request_authority_with_stake(123, 11));

		// The minimum stake was removed, since there's a max candidates limitation
		assert!(RelayAuthorities::candidates()
			.iter()
			.position(|candidate| candidate == &1)
			.is_none());
	});
}

#[test]
fn cancel_request_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(request_authority(1));
		assert!(!RelayAuthorities::candidates().is_empty());
		assert!(!Ring::locks(1).is_empty());
		assert_ok!(RelayAuthorities::cancel_request(Origin::signed(1)));
		assert!(Ring::locks(1).is_empty());

		for i in 1..=<MaxCandidates as Get<usize>>::get() as _ {
			assert_ok!(request_authority(i));
		}
		assert_ok!(RelayAuthorities::cancel_request(Origin::signed(3)));
		assert!(RelayAuthorities::candidates()
			.iter()
			.position(|candidate| candidate == &3)
			.is_none())
	});
}

#[test]
fn renounce_authority_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(request_authority(1));
		assert_ok!(RelayAuthorities::add_authority(Origin::root(), 1));
		assert!(!Ring::locks(1).is_empty());

		assert_err!(
			RelayAuthorities::renounce_authority(Origin::signed(1)),
			RelayAuthoritiesError::OnAuthoritiesChangeDis
		);

		RelayAuthorities::finish_authorities_change();

		let term_duration = <TermDuration as Get<BlockNumber>>::get();

		for i in 0..=term_duration {
			System::set_block_number(term_duration);

			assert_err!(
				RelayAuthorities::renounce_authority(Origin::signed(1)),
				RelayAuthoritiesError::AuthorityIT
			);
		}

		System::set_block_number(term_duration + 1);

		assert_ok!(RelayAuthorities::renounce_authority(Origin::signed(1)));
		assert!(Ring::locks(1).is_empty());
	});
}

#[test]
fn add_authority_should_work() {
	new_test_ext().execute_with(|| {
		assert_err!(
			RelayAuthorities::add_authority(Origin::root(), 1),
			RelayAuthoritiesError::CandidateNE
		);

		assert!(Ring::locks(1).is_empty());
		assert_ok!(request_authority(1));
		assert_ok!(RelayAuthorities::add_authority(Origin::root(), 1));
		assert!(!Ring::locks(1).is_empty());
	});
}

#[test]
fn remove_authority_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(request_authority(1));
		assert_ok!(RelayAuthorities::add_authority(Origin::root(), 1));
		assert!(!Ring::locks(1).is_empty());
		assert_err!(
			RelayAuthorities::remove_authority(Origin::root(), 1),
			RelayAuthoritiesError::OnAuthoritiesChangeDis
		);

		RelayAuthorities::finish_authorities_change();

		assert_ok!(RelayAuthorities::remove_authority(Origin::root(), 1));
		assert!(Ring::locks(1).is_empty());
	});
}

#[test]
fn kill_candidates_should_work() {
	new_test_ext().execute_with(|| {
		let max_candidates = <MaxCandidates as Get<usize>>::get();

		for i in 1..=max_candidates {
			assert_ok!(request_authority(i as _));
			assert!(!Ring::locks(i as AccountId).is_empty());
		}
		assert_eq!(RelayAuthorities::candidates().len(), max_candidates);

		assert_ok!(RelayAuthorities::kill_candidates(Origin::root()));

		for i in 1..=max_candidates {
			assert!(Ring::locks(i as AccountId).is_empty());
		}
		assert!(RelayAuthorities::candidates().is_empty());
	});
}

#[test]
fn encode_message_should_work() {
	// --- substrate ---
	use sp_runtime::RuntimeString;

	// The message is composed of:
	//
	// codec(spec_name: RuntimeString, block number: BlockNumber, mmr_root: Hash)
	let message = {
		_S {
			_1: RuntimeString::from("DRML"),
			_2: 789u32,
			_3: [0u8; 32],
		}
		.encode()
	};
	println!("{:?}", message);

	// The message is composed of:
	//
	// codec(spec_name: RuntimeString, term: u32, new authorities: Vec<Signer>)
	let message = {
		_S {
			_1: RuntimeString::from("DRML"),
			_2: 789u32,
			_3: vec![[7u8; 20], [8u8; 20], [9u8; 20]],
		}
		.encode()
	};
	println!("{:?}", message);
}
