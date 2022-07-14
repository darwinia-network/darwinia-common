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

// --- paritytech ---
use frame_support::{assert_err, assert_noop, assert_ok};
// --- darwinia-network ---
use crate::{
	mock::{AccountId, Balance, BlockNumber, Event, MaxMembers, SubmitDuration, *},
	*,
};

#[test]
fn duplicate_request_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(request_authority(1));
		// Already in candidates
		assert_err!(request_authority(1), RelayAuthoritiesError::CandidateAE);

		assert_ok!(RelayAuthorities::add_authorities(Origin::root(), vec![1]));

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

		let max_candidates = <MaxMembers as Get<u32>>::get() as _;

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

		for i in 1..=<MaxMembers as Get<u32>>::get() as _ {
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
		assert_ok!(RelayAuthorities::add_authorities(Origin::root(), vec![1]));
		assert!(!Ring::locks(1).is_empty());

		assert_err!(
			RelayAuthorities::renounce_authority(Origin::signed(1)),
			RelayAuthoritiesError::OnAuthoritiesChangeDis
		);

		RelayAuthorities::apply_authorities_change().unwrap();
		RelayAuthorities::sync_authorities_change().unwrap();

		let term_duration = <TermDuration as Get<BlockNumber>>::get();

		for i in 0..=term_duration {
			run_to_block(i);

			assert_err!(
				RelayAuthorities::renounce_authority(Origin::signed(1)),
				RelayAuthoritiesError::AuthorityIT
			);
		}

		run_to_block(term_duration + 1);

		assert_ok!(RelayAuthorities::renounce_authority(Origin::signed(1)));
		RelayAuthorities::apply_authorities_change().unwrap();
		assert!(Ring::locks(1).is_empty());
	});
}

#[test]
fn add_authorities_should_work() {
	new_test_ext().execute_with(|| {
		assert_err!(
			RelayAuthorities::add_authorities(Origin::root(), vec![1]),
			RelayAuthoritiesError::CandidateNE
		);

		assert!(Ring::locks(1).is_empty());
		assert!(Ring::locks(2).is_empty());
		assert!(Ring::locks(3).is_empty());
		assert!(RelayAuthorities::next_authorities().is_none());

		assert_ok!(request_authority(1));
		assert_ok!(request_authority(2));
		assert_ok!(request_authority(3));

		assert_ok!(RelayAuthorities::add_authorities(Origin::root(), vec![1, 2, 3]));

		assert!(!Ring::locks(1).is_empty());
		assert!(!Ring::locks(2).is_empty());
		assert!(!Ring::locks(3).is_empty());
		assert_eq!(
			RelayAuthorities::next_authorities().unwrap().next_authorities,
			vec![
				RelayAuthority { account_id: 9, signer: [0; 20], stake: 1, term: 10 },
				RelayAuthority { account_id: 1, signer: [0; 20], stake: 1, term: 10 },
				RelayAuthority { account_id: 2, signer: [0; 20], stake: 1, term: 10 },
				RelayAuthority { account_id: 3, signer: [0; 20], stake: 1, term: 10 }
			]
		);
	});
}

#[test]
fn remove_authorities_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(request_authority(1));
		assert_ok!(RelayAuthorities::add_authorities(Origin::root(), vec![1]));
		assert!(!Ring::locks(1).is_empty());
		assert_err!(
			RelayAuthorities::remove_authorities(Origin::root(), vec![1]),
			RelayAuthoritiesError::OnAuthoritiesChangeDis
		);

		RelayAuthorities::apply_authorities_change().unwrap();
		RelayAuthorities::sync_authorities_change().unwrap();

		assert_err!(
			RelayAuthorities::remove_authorities(Origin::root(), vec![10]),
			RelayAuthoritiesError::AuthorityNE
		);
		assert_ok!(RelayAuthorities::remove_authorities(Origin::root(), vec![1]));

		RelayAuthorities::apply_authorities_change().unwrap();
		assert!(Ring::locks(1).is_empty());
		RelayAuthorities::sync_authorities_change().unwrap();

		assert_err!(
			RelayAuthorities::remove_authorities(Origin::root(), vec![9]),
			RelayAuthoritiesError::AuthoritiesCountTL
		);

		assert_ok!(request_authority(3));
		assert_ok!(request_authority(4));
		assert_ok!(request_authority(5));
		assert_ok!(RelayAuthorities::add_authorities(Origin::root(), vec![3, 4, 5]));

		RelayAuthorities::apply_authorities_change().unwrap();
		RelayAuthorities::sync_authorities_change().unwrap();

		assert_eq!(
			RelayAuthorities::authorities(),
			vec![
				RelayAuthority { account_id: 9, signer: [0; 20], stake: 1, term: 10 },
				RelayAuthority { account_id: 3, signer: [0; 20], stake: 1, term: 10 },
				RelayAuthority { account_id: 4, signer: [0; 20], stake: 1, term: 10 },
				RelayAuthority { account_id: 5, signer: [0; 20], stake: 1, term: 10 }
			]
		);

		assert_ok!(RelayAuthorities::remove_authorities(Origin::root(), vec![9, 4, 5]));

		RelayAuthorities::apply_authorities_change().unwrap();
		RelayAuthorities::sync_authorities_change().unwrap();

		assert_eq!(
			RelayAuthorities::authorities(),
			vec![RelayAuthority { account_id: 3, signer: [0; 20], stake: 1, term: 10 }]
		);
	});
}

#[test]
fn kill_candidates_should_work() {
	new_test_ext().execute_with(|| {
		let max_candidates = <MaxMembers as Get<u32>>::get();

		for i in 1..=max_candidates {
			assert_ok!(request_authority(i as _));
			assert!(!Ring::locks(i as AccountId).is_empty());
		}
		assert_eq!(RelayAuthorities::candidates().len() as u32, max_candidates);

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
		let max_candidates = <MaxMembers as Get<u32>>::get();

		for i in 1..max_candidates {
			assert_eq!(RelayAuthorities::next_term(), i as Term - 1);
			assert_ok!(request_authority(i as _));
			assert_ok!(RelayAuthorities::add_authorities(Origin::root(), vec![i as _]));

			RelayAuthorities::apply_authorities_change().unwrap();
			RelayAuthorities::sync_authorities_change().unwrap();
			assert_eq!(RelayAuthorities::next_term(), i as Term);
		}

		assert_eq!(RelayAuthorities::next_term(), max_candidates as Term - 1);
		assert_ok!(request_authority(max_candidates as _));
		assert_err!(
			RelayAuthorities::add_authorities(Origin::root(), vec![max_candidates as _]),
			<Error<Test>>::TooManyMembers
		);
	});
}

#[test]
#[ignore]
fn encode_message_should_work() {
	// --- paritytech ---
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
		Message {
			_1: RuntimeString::from("DRML"),
			_2: array_bytes::hex2array_unchecked::<4>("0x479fbdf9"),
			_3: 789u32,
			_4: [0u8; 32],
		}
		.encode()
	};
	println!("{:?}", message);
	println!("{}", array_bytes::bytes2hex("0x", &message));

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
		Message {
			_1: RuntimeString::from("DRML"),
			_2: array_bytes::hex2array_unchecked::<4>("0xb4bcf497"),
			_3: 789u32,
			_4: vec![[7u8; 20], [8u8; 20], [9u8; 20]],
		}
		.encode()
	};
	println!("{:?}", message);
	println!("{}", array_bytes::bytes2hex("0x", &message));
}

#[test]
fn schedule_too_many_should_fail() {
	new_test_ext().execute_with(|| {
		let max_scheduled_num = MaxSchedules::get() as BlockNumber;

		for block_number in 0..max_scheduled_num {
			assert_ok!(RelayAuthorities::schedule_mmr_root(block_number));
		}

		assert_noop!(
			RelayAuthorities::schedule_mmr_root(max_scheduled_num),
			RelayAuthoritiesError::TooManySchedules
		);
	});
}

#[test]
fn schedule_mmr_root_and_mmr_root_signed_event_should_work() {
	for block_number in 4..25 {
		new_test_ext().execute_with(|| {
			assert_ok!(request_authority(1));
			assert_ok!(RelayAuthorities::add_authorities(Origin::root(), vec![1]));
			assert_ok!(RelayAuthorities::submit_signed_authorities(
				Origin::signed(9),
				DEFAULT_SIGNATURE
			));

			RelayAuthorities::apply_authorities_change().unwrap();
			RelayAuthorities::sync_authorities_change().unwrap();
			RelayAuthorities::schedule_mmr_root(block_number).unwrap();

			let headers = run_to_block_from_genesis(block_number + 2);
			let mmr_root = HeaderMmr::find_parent_mmr_root(&headers[headers.len() - 2]).unwrap();

			System::reset_events();

			assert_ok!(RelayAuthorities::submit_signed_mmr_root(
				Origin::signed(9),
				block_number,
				DEFAULT_SIGNATURE,
			));
			assert!(relay_authorities_events().is_empty());
			assert_ok!(RelayAuthorities::submit_signed_mmr_root(
				Origin::signed(1),
				block_number,
				DEFAULT_SIGNATURE,
			));
			assert_eq!(
				relay_authorities_events(),
				vec![Event::RelayAuthorities(crate::Event::MmrRootSigned(
					block_number,
					mmr_root,
					vec![(9, DEFAULT_SIGNATURE), (1, DEFAULT_SIGNATURE)]
				))]
			);
		});
	}
}

#[test]
fn authorities_change_signed_event_should_work() {
	new_test_ext().execute_with(|| {
		run_to_block(1);

		assert_ok!(request_authority(1));
		assert_ok!(RelayAuthorities::add_authorities(Origin::root(), vec![1]));

		System::reset_events();

		assert_ok!(RelayAuthorities::submit_signed_authorities(
			Origin::signed(9),
			DEFAULT_SIGNATURE
		));

		assert_eq!(
			relay_authorities_events(),
			vec![Event::RelayAuthorities(crate::Event::AuthoritiesChangeSigned(
				0,
				vec![signer_of(9), signer_of(1)],
				vec![(9, DEFAULT_SIGNATURE)]
			))]
		);

		RelayAuthorities::apply_authorities_change().unwrap();
		RelayAuthorities::sync_authorities_change().unwrap();

		assert_ok!(request_authority(2));
		assert_ok!(RelayAuthorities::add_authorities(Origin::root(), vec![2]));

		System::reset_events();

		assert_ok!(RelayAuthorities::submit_signed_authorities(
			Origin::signed(9),
			DEFAULT_SIGNATURE
		));
		// Not enough signatures, `1 / 2 < 60%`
		assert!(relay_authorities_events().is_empty());
		assert_ok!(RelayAuthorities::submit_signed_authorities(
			Origin::signed(1),
			DEFAULT_SIGNATURE
		));

		// Enough signatures, `2 / 2 > 60%`
		assert_eq!(
			relay_authorities_events(),
			vec![Event::RelayAuthorities(crate::Event::AuthoritiesChangeSigned(
				1,
				vec![signer_of(9), signer_of(1), signer_of(2)],
				vec![(9, DEFAULT_SIGNATURE), (1, DEFAULT_SIGNATURE)]
			))]
		);
	});
}

#[test]
fn schedule_authorities_change_should_work() {
	new_test_ext().execute_with(|| {
		assert!(RelayAuthorities::next_authorities().is_none());

		assert_ok!(request_authority(1));
		assert_ok!(RelayAuthorities::add_authorities(Origin::root(), vec![1]));

		let authorities =
			vec![RelayAuthority { account_id: 9, signer: [0; 20], stake: 1, term: 10 }];
		let schedule_authorities_change = ScheduledAuthoritiesChange {
			next_authorities: BoundedVec::try_from(vec![
				RelayAuthority { account_id: 9, signer: [0; 20], stake: 1, term: 10 },
				RelayAuthority { account_id: 1, signer: [0; 20], stake: 1, term: 10 },
			])
			.unwrap(),
			deadline: 3,
		};

		assert_eq!(RelayAuthorities::authorities(), authorities);
		assert_eq!(RelayAuthorities::next_authorities(), Some(schedule_authorities_change.clone()));

		RelayAuthorities::apply_authorities_change().unwrap();

		assert_eq!(RelayAuthorities::authorities(), authorities);
		assert_eq!(RelayAuthorities::next_authorities(), Some(schedule_authorities_change.clone()));

		RelayAuthorities::apply_authorities_change().unwrap();
		RelayAuthorities::sync_authorities_change().unwrap();

		assert_eq!(RelayAuthorities::authorities(), schedule_authorities_change.next_authorities);
		assert!(RelayAuthorities::next_authorities().is_none());
	});
}

#[test]
fn kill_authorities_and_force_new_term_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(request_authority(1));
		assert_ok!(RelayAuthorities::add_authorities(Origin::root(), vec![1]));

		RelayAuthorities::apply_authorities_change().unwrap();
		RelayAuthorities::sync_authorities_change().unwrap();

		assert_eq!(
			RelayAuthorities::authorities(),
			vec![
				RelayAuthority { account_id: 9, signer: [0; 20], stake: 1, term: 10 },
				RelayAuthority { account_id: 1, signer: [0; 20], stake: 1, term: 10 }
			]
		);
		assert!(RelayAuthorities::next_authorities().is_none());
		assert_eq!(RelayAuthorities::submit_duration(), SubmitDuration::get());

		assert_err!(
			RelayAuthorities::force_new_term(Origin::root()),
			RelayAuthoritiesError::NextAuthoritiesNE
		);

		assert_ok!(request_authority(2));
		assert_ok!(RelayAuthorities::add_authorities(Origin::root(), vec![2]));

		assert_ok!(RelayAuthorities::force_new_term(Origin::root()));

		assert_eq!(
			RelayAuthorities::authorities(),
			vec![
				RelayAuthority { account_id: 9, signer: [0; 20], stake: 1, term: 10 },
				RelayAuthority { account_id: 1, signer: [0; 20], stake: 1, term: 10 },
				RelayAuthority { account_id: 2, signer: [0; 20], stake: 1, term: 10 }
			]
		);
		assert!(RelayAuthorities::next_authorities().is_none());
		assert_eq!(RelayAuthorities::submit_duration(), SubmitDuration::get());

		assert_ok!(RelayAuthorities::kill_authorities(Origin::root()));
		assert_ok!(request_authority(3));
		assert_ok!(RelayAuthorities::add_authorities(Origin::root(), vec![3]));
		assert_ok!(RelayAuthorities::force_new_term(Origin::root()));

		assert_eq!(
			RelayAuthorities::authorities(),
			vec![RelayAuthority { account_id: 3, signer: [0; 20], stake: 1, term: 10 },]
		);
		assert!(RelayAuthorities::next_authorities().is_none());
		assert_eq!(RelayAuthorities::submit_duration(), SubmitDuration::get());
	});
}

#[test]
fn lock_after_authorities_change_should_work() {
	new_test_ext().execute_with(|| {
		assert!(!Ring::locks(9).is_empty());
		assert!(Ring::locks(1).is_empty());
		assert!(Ring::locks(2).is_empty());

		assert_ok!(request_authority(1));
		assert_ok!(request_authority(2));
		assert_ok!(RelayAuthorities::add_authorities(Origin::root(), vec![1, 2]));

		assert!(!Ring::locks(9).is_empty());
		assert!(!Ring::locks(1).is_empty());
		assert!(!Ring::locks(2).is_empty());

		RelayAuthorities::apply_authorities_change().unwrap();
		RelayAuthorities::sync_authorities_change().unwrap();

		assert!(!Ring::locks(9).is_empty());
		assert!(!Ring::locks(1).is_empty());
		assert!(!Ring::locks(2).is_empty());

		assert_ok!(RelayAuthorities::remove_authorities(Origin::root(), vec![9, 2]));

		RelayAuthorities::apply_authorities_change().unwrap();
		RelayAuthorities::sync_authorities_change().unwrap();

		assert!(Ring::locks(9).is_empty());
		assert!(!Ring::locks(1).is_empty());
		assert!(Ring::locks(2).is_empty());
	});
}

#[test]
fn check_authorities_change_to_sync_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(request_authority(1));
		assert_ok!(request_authority(2));
		assert_ok!(request_authority(3));
		assert_ok!(RelayAuthorities::add_authorities(Origin::root(), vec![1, 2, 3]));
		RelayAuthorities::apply_authorities_change().unwrap();

		assert_err!(
			RelayAuthorities::check_authorities_change_to_sync(
				0,
				vec![signer_of(1), signer_of(2), signer_of(3)]
			),
			RelayAuthoritiesError::AuthoritiesMis
		);
		assert_err!(
			RelayAuthorities::check_authorities_change_to_sync(
				0,
				vec![signer_of(3), signer_of(1), signer_of(2)]
			),
			RelayAuthoritiesError::AuthoritiesMis
		);
		assert_err!(
			RelayAuthorities::check_authorities_change_to_sync(
				0,
				vec![signer_of(3), signer_of(2), signer_of(1)]
			),
			RelayAuthoritiesError::AuthoritiesMis
		);
		assert_ok!(RelayAuthorities::check_authorities_change_to_sync(
			0,
			vec![signer_of(9), signer_of(1), signer_of(2), signer_of(3)]
		));
		assert_ok!(RelayAuthorities::check_authorities_change_to_sync(
			0,
			vec![signer_of(9), signer_of(3), signer_of(2), signer_of(1)]
		));
		assert_ok!(RelayAuthorities::check_authorities_change_to_sync(
			0,
			vec![signer_of(1), signer_of(3), signer_of(9), signer_of(2)]
		));
	});
}

#[test]
fn slash_should_work() {
	new_test_ext().execute_with(|| {
		run_to_block(1);

		assert_eq!(Ring::total_balance(&9), 900);
		assert_eq!(Ring::total_balance(&1), 100);
		assert_eq!(Ring::total_balance(&2), 200);

		assert_ok!(request_authority_with_stake(1, 50));
		assert_ok!(request_authority_with_stake(2, 60));
		assert_ok!(RelayAuthorities::add_authorities(Origin::root(), vec![1, 2]));

		RelayAuthorities::apply_authorities_change().unwrap();
		RelayAuthorities::sync_authorities_change().unwrap();

		assert_ok!(RelayAuthorities::remove_authorities(Origin::root(), vec![9]));

		assert!(!Ring::locks(9).is_empty());
		assert!(!Ring::locks(1).is_empty());
		assert!(!Ring::locks(2).is_empty());

		// First time miss signature
		System::reset_events();
		run_to_block(SubmitDuration::get() + 1);

		assert_eq!(
			relay_authorities_events(),
			vec![
				Event::RelayAuthorities(crate::Event::SlashOnMisbehavior(9, 1)),
				Event::RelayAuthorities(crate::Event::SlashOnMisbehavior(1, 50)),
				Event::RelayAuthorities(crate::Event::SlashOnMisbehavior(2, 60)),
			]
		);
		assert!(Ring::locks(9).is_empty());
		assert!(Ring::locks(1).is_empty());
		assert!(Ring::locks(2).is_empty());
		assert_eq!(Ring::total_balance(&9), 899);
		assert_eq!(Ring::total_balance(&1), 50);
		assert_eq!(Ring::total_balance(&2), 140);

		// N times miss signature (only slash on the first time)
		for i in 2..10 {
			System::reset_events();
			run_to_block(SubmitDuration::get() * i + 1);

			assert_eq!(
				relay_authorities_events(),
				vec![
					Event::RelayAuthorities(crate::Event::SlashOnMisbehavior(9, 0)),
					Event::RelayAuthorities(crate::Event::SlashOnMisbehavior(1, 0)),
					Event::RelayAuthorities(crate::Event::SlashOnMisbehavior(2, 0)),
				]
			);
			assert!(Ring::locks(9).is_empty());
			assert!(Ring::locks(1).is_empty());
			assert!(Ring::locks(2).is_empty());
			assert_eq!(Ring::total_balance(&9), 899);
			assert_eq!(Ring::total_balance(&1), 50);
			assert_eq!(Ring::total_balance(&2), 140);
		}
	});
}
