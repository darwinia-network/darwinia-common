// --- substrate ---
use frame_support::{assert_err, assert_ok};
// --- darwinia ---
use crate::{
	mock::{AccountId, BlockNumber, Event, SubmitDuration, *},
	*,
};

#[test]
fn duplicate_request_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(request_authority(1));
		// Already in candidates
		assert_err!(request_authority(1), RelayAuthoritiesError::CandidateAE);

		assert_ok!(RelayAuthorities::add_authority(Origin::root(), 1));

		// Already in next authorities
		assert_err!(request_authority(1), RelayAuthoritiesError::AuthorityAE);

		// Already in authorities
		assert_err!(request_authority(9), RelayAuthoritiesError::AuthorityAE);
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

		RelayAuthorities::apply_authorities_change().unwrap();
		RelayAuthorities::sync_authorities_change();

		let term_duration = <TermDuration as Get<BlockNumber>>::get();

		for i in 0..=term_duration {
			System::set_block_number(i);

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

		RelayAuthorities::apply_authorities_change().unwrap();
		RelayAuthorities::sync_authorities_change();

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
fn authority_term_should_work() {
	new_test_ext().execute_with(|| {
		let max_candidates = <MaxCandidates as Get<usize>>::get();

		for i in 1..=max_candidates {
			assert_eq!(RelayAuthorities::authority_term(), i as Term - 1);
			assert_ok!(request_authority(i as _));
			assert_ok!(RelayAuthorities::add_authority(Origin::root(), i as _));

			RelayAuthorities::sync_authorities_change();
			assert_eq!(RelayAuthorities::authority_term(), i as Term);
		}
	});
}

#[test]
fn encode_message_should_work() {
	// --- substrate ---
	use sp_runtime::RuntimeString;

	// The message is composed of:
	//
	// hash(
	// 	codec(
	// 		spec_name: String,
	// 		op_code: OpCode,
	// 		block number: Compact<BlockNumber>,
	// 		mmr_root: Hash
	// 	)
	// )
	let message = {
		_S {
			_1: RuntimeString::from("DRML"),
			_2: array_bytes::hex_str_array_unchecked!("0x479fbdf9", 4),
			_3: 789u32,
			_4: [0u8; 32],
		}
		.encode()
	};
	println!("{:?}", message);
	println!("{}", array_bytes::hex_str("0x", message));

	// The message is composed of:
	//
	// hash(
	// 	codec(
	// 		spec_name: String,
	// 		op_code: OpCode,
	// 		term: Compact<u32>,
	// 		next authorities: Vec<Signer>
	// 	)
	// )
	let message = {
		_S {
			_1: RuntimeString::from("DRML"),
			_2: array_bytes::hex_str_array_unchecked!("0xb4bcf497", 4),
			_3: 789u32,
			_4: vec![[7u8; 20], [8u8; 20], [9u8; 20]],
		}
		.encode()
	};
	println!("{:?}", message);
	println!("{}", array_bytes::hex_str("0x", message));
}

#[test]
fn mmr_root_signed_event_should_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(request_authority(1));
		assert_ok!(RelayAuthorities::add_authority(Origin::root(), 1));
		assert_ok!(RelayAuthorities::submit_signed_authorities(
			Origin::signed(9),
			[0; 65]
		));

		RelayAuthorities::sync_authorities_change();
		System::reset_events();

		RelayAuthorities::schedule_mmr_root(10);
		System::reset_events();

		assert_ok!(RelayAuthorities::submit_signed_mmr_root(
			Origin::signed(9),
			10,
			[0; 65],
		));
		assert!(relay_authorities_events().is_empty());
		assert_ok!(RelayAuthorities::submit_signed_mmr_root(
			Origin::signed(1),
			10,
			[0; 65],
		));
		assert_eq!(
			relay_authorities_events(),
			vec![Event::relay_authorities(RawEvent::MMRRootSigned(
				10,
				Default::default(),
				vec![(9, [0; 65]), (1, [0; 65])]
			))]
		);
	});
}

#[test]
fn authorities_change_signed_event_should_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(request_authority(1));
		assert_ok!(RelayAuthorities::add_authority(Origin::root(), 1));

		System::reset_events();

		assert_ok!(RelayAuthorities::submit_signed_authorities(
			Origin::signed(9),
			[0; 65]
		));

		assert_eq!(
			relay_authorities_events(),
			vec![Event::relay_authorities(RawEvent::AuthoritiesChangeSigned(
				0,
				vec![Default::default(), Default::default()],
				vec![(9, [0; 65])]
			))]
		);

		RelayAuthorities::sync_authorities_change();

		assert_ok!(request_authority(2));
		assert_ok!(RelayAuthorities::add_authority(Origin::root(), 2));

		System::reset_events();

		assert_ok!(RelayAuthorities::submit_signed_authorities(
			Origin::signed(9),
			[0; 65]
		));
		// Not enough signatures, `1 / 2 < 60%`
		assert!(relay_authorities_events().is_empty());
		assert_ok!(RelayAuthorities::submit_signed_authorities(
			Origin::signed(1),
			[0; 65]
		));

		// Enough signatures, `2 / 2 > 60%`
		assert_eq!(
			relay_authorities_events(),
			vec![Event::relay_authorities(RawEvent::AuthoritiesChangeSigned(
				1,
				vec![Default::default(), Default::default(), Default::default()],
				vec![(9, [0; 65]), (1, [0; 65])]
			))]
		);
	});
}

#[test]
fn schedule_authorities_change_should_work() {
	new_test_ext().execute_with(|| {
		assert!(RelayAuthorities::next_authorities().is_none());

		assert_ok!(request_authority(1));
		assert_ok!(RelayAuthorities::add_authority(Origin::root(), 1));

		assert_eq!(
			RelayAuthorities::authorities(),
			vec![RelayAuthority {
				account_id: 9,
				signer: [0; 20],
				stake: 1,
				term: 10
			}]
		);
		assert_eq!(
			RelayAuthorities::next_authorities(),
			Some(ScheduledAuthoritiesChange {
				next_authorities: vec![
					RelayAuthority {
						account_id: 9,
						signer: [0; 20],
						stake: 1,
						term: 10
					},
					RelayAuthority {
						account_id: 1,
						signer: [0; 20],
						stake: 1,
						term: 10
					}
				],
				deadline: 3
			})
		);

		RelayAuthorities::apply_authorities_change().unwrap();

		assert_eq!(
			RelayAuthorities::authorities(),
			vec![
				RelayAuthority {
					account_id: 9,
					signer: [0; 20],
					stake: 1,
					term: 10
				},
				RelayAuthority {
					account_id: 1,
					signer: [0; 20],
					stake: 1,
					term: 10
				}
			]
		);
		assert_eq!(
			RelayAuthorities::next_authorities(),
			Some(ScheduledAuthoritiesChange {
				next_authorities: vec![],
				deadline: 3
			})
		);

		RelayAuthorities::sync_authorities_change();

		assert_eq!(
			RelayAuthorities::authorities(),
			vec![
				RelayAuthority {
					account_id: 9,
					signer: [0; 20],
					stake: 1,
					term: 10
				},
				RelayAuthority {
					account_id: 1,
					signer: [0; 20],
					stake: 1,
					term: 10
				}
			]
		);
		assert!(RelayAuthorities::next_authorities().is_none());
	});
}

#[test]
fn kill_authorities_and_force_new_term_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(request_authority(1));
		assert_ok!(RelayAuthorities::add_authority(Origin::root(), 1));

		RelayAuthorities::apply_authorities_change().unwrap();
		RelayAuthorities::sync_authorities_change();

		assert_eq!(
			RelayAuthorities::authorities(),
			vec![
				RelayAuthority {
					account_id: 9,
					signer: [0; 20],
					stake: 1,
					term: 10
				},
				RelayAuthority {
					account_id: 1,
					signer: [0; 20],
					stake: 1,
					term: 10
				}
			]
		);
		assert!(RelayAuthorities::next_authorities().is_none());
		assert_eq!(RelayAuthorities::submit_duration(), SubmitDuration::get());

		assert_err!(
			RelayAuthorities::force_new_term(Origin::root()),
			RelayAuthoritiesError::NextAuthoritiesNE
		);

		assert_ok!(request_authority(2));
		assert_ok!(RelayAuthorities::add_authority(Origin::root(), 2));

		assert_ok!(RelayAuthorities::force_new_term(Origin::root()));

		assert_eq!(
			RelayAuthorities::authorities(),
			vec![
				RelayAuthority {
					account_id: 9,
					signer: [0; 20],
					stake: 1,
					term: 10
				},
				RelayAuthority {
					account_id: 1,
					signer: [0; 20],
					stake: 1,
					term: 10
				},
				RelayAuthority {
					account_id: 2,
					signer: [0; 20],
					stake: 1,
					term: 10
				}
			]
		);
		assert!(RelayAuthorities::next_authorities().is_none());
		assert_eq!(RelayAuthorities::submit_duration(), SubmitDuration::get());

		assert_ok!(RelayAuthorities::kill_authorities(Origin::root()));
		assert_ok!(request_authority(3));
		assert_ok!(RelayAuthorities::add_authority(Origin::root(), 3));
		assert_ok!(RelayAuthorities::force_new_term(Origin::root()));

		assert_eq!(
			RelayAuthorities::authorities(),
			vec![RelayAuthority {
				account_id: 3,
				signer: [0; 20],
				stake: 1,
				term: 10
			},]
		);
		assert!(RelayAuthorities::next_authorities().is_none());
		assert_eq!(RelayAuthorities::submit_duration(), SubmitDuration::get());
	});
}
