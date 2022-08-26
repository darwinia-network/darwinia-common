// --- paritytech ---
use frame_support::{assert_noop, assert_ok, weights::PostDispatchInfo};
// --- darwinia-network ---
use crate::{mock::*, Event, *};

#[test]
fn add_authority() {
	let address = Address::repeat_byte(0);

	ExtBuilder::default().build().execute_with(|| {
		assert!(EcdsaAuthority::authorities().is_empty());
		assert!(EcdsaAuthority::next_authorities().is_empty());
		assert_eq!(EcdsaAuthority::nonce(), 0);
		assert_ok!(EcdsaAuthority::add_authority(Origin::root(), address));
		assert!(EcdsaAuthority::authorities().is_empty());
		assert_eq!(EcdsaAuthority::next_authorities(), vec![address]);
		assert_eq!(EcdsaAuthority::nonce(), 0);
		let message = [
			167, 135, 21, 62, 159, 236, 10, 205, 140, 44, 190, 61, 63, 168, 9, 26, 88, 230, 156,
			27, 40, 48, 231, 120, 254, 96, 184, 174, 192, 153, 29, 246,
		];
		assert_eq!(
			EcdsaAuthority::authorities_change_to_sign(),
			Some((Operation::AddMember { new: address }, Some(1), message, Default::default()))
		);
		assert_eq!(
			ecdsa_authority_events(),
			vec![Event::CollectingAuthoritiesChangeSignatures { message }]
		);

		// Case 1.
		assert_noop!(
			EcdsaAuthority::add_authority(Origin::root(), address),
			EcdsaAuthorityError::OnAuthoritiesChange
		);
		presume_authority_change_succeed();
		assert_eq!(EcdsaAuthority::authorities(), vec![address]);
		assert_eq!(EcdsaAuthority::nonce(), 1);

		// Case 2.
		assert_noop!(
			EcdsaAuthority::add_authority(Origin::signed(Default::default()), address),
			DispatchError::BadOrigin
		);

		// Case 3.
		assert_noop!(
			EcdsaAuthority::add_authority(Origin::root(), address),
			EcdsaAuthorityError::AuthorityExisted
		);

		// Case 4.
		(1..MaxAuthorities::get()).for_each(|i| {
			assert_ok!(EcdsaAuthority::add_authority(Origin::root(), Address::repeat_byte(i as _)));
			presume_authority_change_succeed();
			assert_eq!(EcdsaAuthority::nonce(), 1 + i);
		});
		assert_noop!(
			EcdsaAuthority::add_authority(
				Origin::root(),
				Address::repeat_byte(MaxAuthorities::get() as _)
			),
			EcdsaAuthorityError::TooManyAuthorities
		);

		// Check order.
		assert_eq!(
			EcdsaAuthority::authorities(),
			(0..MaxAuthorities::get())
				.rev()
				.map(|i| Address::repeat_byte(i as _))
				.collect::<Vec<_>>()
		);
	});
}

#[test]
fn remove_authority() {
	let address_1 = Address::repeat_byte(1);
	let address_2 = Address::repeat_byte(2);

	ExtBuilder::default().authorities(vec![address_1, address_2]).build().execute_with(|| {
		assert_eq!(EcdsaAuthority::authorities(), vec![address_1, address_2]);
		assert_eq!(EcdsaAuthority::next_authorities(), vec![address_1, address_2]);
		assert_eq!(EcdsaAuthority::nonce(), 0);
		assert_ok!(EcdsaAuthority::remove_authority(Origin::root(), address_1));
		assert_eq!(EcdsaAuthority::authorities(), vec![address_1, address_2]);
		assert_eq!(EcdsaAuthority::next_authorities(), vec![address_2]);
		assert_eq!(EcdsaAuthority::nonce(), 0);
		let message = [
			11, 46, 204, 51, 51, 180, 179, 70, 172, 1, 88, 222, 62, 26, 21, 152, 145, 128, 202,
			144, 70, 40, 78, 207, 37, 176, 142, 60, 182, 133, 206, 20,
		];
		assert_eq!(
			EcdsaAuthority::authorities_change_to_sign(),
			Some((
				Operation::RemoveMember { pre: AUTHORITY_SENTINEL, old: address_1 },
				Some(1),
				message,
				Default::default()
			))
		);
		assert_eq!(
			ecdsa_authority_events(),
			vec![Event::CollectingAuthoritiesChangeSignatures { message }]
		);

		// Case 1.
		assert_noop!(
			EcdsaAuthority::add_authority(Origin::root(), address_1),
			EcdsaAuthorityError::OnAuthoritiesChange
		);
		presume_authority_change_succeed();
		assert_eq!(EcdsaAuthority::authorities(), vec![address_2]);
		assert_eq!(EcdsaAuthority::nonce(), 1);

		// Case 2.
		assert_noop!(
			EcdsaAuthority::remove_authority(Origin::signed(Default::default()), address_2),
			DispatchError::BadOrigin
		);

		// Case 3.
		assert_noop!(
			EcdsaAuthority::remove_authority(Origin::root(), address_1),
			EcdsaAuthorityError::NotAuthority
		);

		// Case 4.
		assert_noop!(
			EcdsaAuthority::remove_authority(Origin::root(), address_2),
			EcdsaAuthorityError::AtLeastOneAuthority
		);
	});
}

#[test]
fn swap_authority() {
	let address_1 = Address::repeat_byte(1);
	let address_2 = Address::repeat_byte(2);

	ExtBuilder::default().authorities(vec![address_1]).build().execute_with(|| {
		assert_eq!(EcdsaAuthority::authorities(), vec![address_1]);
		assert_eq!(EcdsaAuthority::next_authorities(), vec![address_1]);
		assert_eq!(EcdsaAuthority::nonce(), 0);
		assert_ok!(EcdsaAuthority::swap_authority(Origin::root(), address_1, address_2));
		assert_eq!(EcdsaAuthority::authorities(), vec![address_1]);
		assert_eq!(EcdsaAuthority::next_authorities(), vec![address_2]);
		assert_eq!(EcdsaAuthority::nonce(), 0);
		let message = [
			124, 233, 77, 172, 154, 1, 15, 166, 69, 156, 210, 158, 156, 177, 115, 47, 205, 200,
			106, 117, 44, 240, 90, 198, 83, 248, 26, 138, 37, 9, 105, 204,
		];
		assert_eq!(
			EcdsaAuthority::authorities_change_to_sign(),
			Some((
				Operation::SwapMembers { pre: AUTHORITY_SENTINEL, old: address_1, new: address_2 },
				None,
				message,
				Default::default()
			))
		);
		assert_eq!(
			ecdsa_authority_events(),
			vec![Event::CollectingAuthoritiesChangeSignatures { message }]
		);

		// Case 1.
		assert_noop!(
			EcdsaAuthority::swap_authority(Origin::root(), address_2, address_1),
			EcdsaAuthorityError::OnAuthoritiesChange
		);
		presume_authority_change_succeed();
		assert_eq!(EcdsaAuthority::authorities(), vec![address_2]);
		assert_eq!(EcdsaAuthority::nonce(), 1);

		// Case 2.
		assert_noop!(
			EcdsaAuthority::swap_authority(Origin::signed(1), address_2, address_1),
			DispatchError::BadOrigin
		);

		// Case 3.
		assert_noop!(
			EcdsaAuthority::swap_authority(Origin::root(), address_1, address_2),
			EcdsaAuthorityError::NotAuthority
		);
	});
}

#[test]
fn sync_interval_and_max_pending_period() {
	ExtBuilder::default().build().execute_with(|| {
		// Check new message root while reaching the sync interval checkpoint.
		(2..SyncInterval::get()).for_each(|i| {
			run_to_block(i);
			assert!(EcdsaAuthority::new_message_root_to_sign().is_none());
		});
		run_to_block(SyncInterval::get());
		let message = [
			159, 247, 43, 185, 157, 74, 126, 205, 108, 104, 253, 73, 176, 246, 156, 154, 97, 206,
			211, 254, 16, 3, 191, 15, 171, 104, 151, 60, 37, 145, 208, 225,
		];
		assert_eq!(
			EcdsaAuthority::new_message_root_to_sign(),
			Some((
				Commitment {
					block_number: System::block_number() as _,
					message_root: Default::default(),
					nonce: 0
				},
				message,
				Default::default()
			))
		);
		assert_eq!(
			ecdsa_authority_events(),
			vec![Event::CollectingNewMessageRootSignatures { message }]
		);

		// Use a new message root while exceeding the max pending period.
		new_message_root(1);
		let offset = System::block_number() + 1;
		(offset..offset + MaxPendingPeriod::get()).for_each(|i| {
			run_to_block(i);
			assert_eq!(
				EcdsaAuthority::new_message_root_to_sign(),
				Some((
					Commitment { block_number: 3, message_root: Default::default(), nonce: 0 },
					message,
					Default::default()
				))
			);
		});
		run_to_block(offset + MaxPendingPeriod::get());
		let message = [
			171, 2, 58, 75, 46, 20, 234, 199, 81, 136, 133, 190, 195, 28, 247, 156, 105, 23, 147,
			237, 231, 40, 180, 127, 138, 138, 21, 158, 23, 116, 176, 7,
		];
		assert_eq!(
			EcdsaAuthority::new_message_root_to_sign(),
			Some((
				Commitment { block_number: 9, message_root: message_root_of(1), nonce: 0 },
				message,
				Default::default()
			))
		);

		// Not allow to update the message root while authorities changing.
		assert_ok!(EcdsaAuthority::add_authority(Origin::root(), Default::default()));
		new_message_root(2);
		let offset = System::block_number() + 1;
		(offset..=offset + MaxPendingPeriod::get()).for_each(|i| {
			run_to_block(i);
			assert_eq!(
				EcdsaAuthority::new_message_root_to_sign(),
				Some((
					Commitment { block_number: 9, message_root: message_root_of(1), nonce: 0 },
					message,
					Default::default()
				))
			);
		});
	});
}

#[test]
fn submit_authorities_change_signature() {
	let (secret_key_1, address_1) = gen_pair(1);
	let (secret_key_2, address_2) = gen_pair(2);
	let (_, address_3) = gen_pair(3);

	ExtBuilder::default().authorities(vec![address_1, address_2]).build().execute_with(|| {
		// Case 1.
		assert_noop!(
			EcdsaAuthority::submit_authorities_change_signature(
				Origin::signed(Default::default()),
				address_1,
				Default::default(),
			),
			EcdsaAuthorityError::NoAuthoritiesChange
		);

		assert_ok!(EcdsaAuthority::add_authority(Origin::root(), address_3));
		let operation = Operation::AddMember { new: address_3 };
		let message = [
			180, 255, 102, 4, 68, 26, 118, 112, 154, 67, 234, 112, 236, 182, 231, 173, 135, 87,
			117, 122, 184, 129, 63, 49, 218, 224, 39, 39, 44, 240, 100, 255,
		];
		assert_eq!(
			EcdsaAuthority::authorities_change_to_sign(),
			Some((operation.clone(), Some(2), message, Default::default()))
		);
		assert_eq!(
			ecdsa_authority_events(),
			vec![Event::CollectingAuthoritiesChangeSignatures { message }]
		);

		// Case 2.
		assert_noop!(
			EcdsaAuthority::submit_authorities_change_signature(
				Origin::signed(Default::default()),
				address_1,
				Default::default(),
			),
			EcdsaAuthorityError::BadSignature
		);

		let nonce = EcdsaAuthority::nonce();
		let signature_1 = sign(&secret_key_1, &message);
		assert_eq!(EcdsaAuthority::nonce(), nonce);
		assert_ok!(EcdsaAuthority::submit_authorities_change_signature(
			Origin::signed(Default::default()),
			address_1,
			signature_1.clone(),
		));
		assert_eq!(
			EcdsaAuthority::authorities_change_to_sign(),
			Some((
				operation.clone(),
				Some(2),
				message,
				BoundedVec::try_from(vec![(address_1, signature_1.clone())]).unwrap()
			))
		);

		let signature_2 = sign(&secret_key_2, &message);
		assert_ok!(EcdsaAuthority::submit_authorities_change_signature(
			Origin::signed(Default::default()),
			address_2,
			signature_2.clone(),
		));
		assert_eq!(EcdsaAuthority::nonce(), nonce + 1);
		assert!(EcdsaAuthority::authorities_change_to_sign().is_none());
		assert_eq!(
			ecdsa_authority_events(),
			vec![
				Event::CollectedEnoughAuthoritiesChangeSignatures {
					operation,
					new_threshold: Some(2),
					message,
					signatures: vec![(address_1, signature_1), (address_2, signature_2)]
				},
				Event::CollectingNewMessageRootSignatures {
					message: [
						154, 219, 45, 185, 181, 249, 194, 236, 54, 17, 201, 121, 48, 58, 30, 38,
						23, 204, 118, 118, 94, 117, 242, 172, 64, 251, 245, 74, 235, 49, 46, 132
					]
				}
			]
		);
	});
}

#[test]
fn submit_new_message_root_signature() {
	let (secret_key_1, address_1) = gen_pair(1);
	let (secret_key_2, address_2) = gen_pair(2);
	let (secret_key_3, address_3) = gen_pair(3);

	ExtBuilder::default().authorities(vec![address_1, address_2]).build().execute_with(|| {
		// Case 1.
		assert_noop!(
			EcdsaAuthority::submit_new_message_root_signature(
				Origin::signed(Default::default()),
				address_1,
				Default::default(),
			),
			EcdsaAuthorityError::NoNewMessageRoot
		);

		run_to_block(SyncInterval::get());
		let message = [
			159, 247, 43, 185, 157, 74, 126, 205, 108, 104, 253, 73, 176, 246, 156, 154, 97, 206,
			211, 254, 16, 3, 191, 15, 171, 104, 151, 60, 37, 145, 208, 225,
		];
		assert_eq!(
			EcdsaAuthority::new_message_root_to_sign(),
			Some((
				Commitment {
					block_number: System::block_number() as _,
					message_root: Default::default(),
					nonce: 0
				},
				message,
				Default::default()
			))
		);
		assert_eq!(
			ecdsa_authority_events(),
			vec![Event::CollectingNewMessageRootSignatures { message }]
		);

		// Case 2.
		assert_noop!(
			EcdsaAuthority::submit_new_message_root_signature(
				Origin::signed(Default::default()),
				address_1,
				Default::default(),
			),
			EcdsaAuthorityError::BadSignature
		);

		// Case 3.
		let signature_3 = sign(&secret_key_3, &message);
		assert_noop!(
			EcdsaAuthority::submit_new_message_root_signature(
				Origin::signed(Default::default()),
				address_3,
				signature_3,
			),
			EcdsaAuthorityError::NotAuthority
		);

		let nonce = EcdsaAuthority::nonce();
		let signature_1 = sign(&secret_key_1, &message);
		assert_eq!(EcdsaAuthority::nonce(), nonce);
		assert_ok!(EcdsaAuthority::submit_new_message_root_signature(
			Origin::signed(Default::default()),
			address_1,
			signature_1.clone(),
		));
		assert_eq!(
			EcdsaAuthority::new_message_root_to_sign(),
			Some((
				Commitment {
					block_number: System::block_number() as _,
					message_root: Default::default(),
					nonce: 0
				},
				message,
				BoundedVec::try_from(vec![(address_1, signature_1.clone())]).unwrap()
			))
		);

		let signature_2 = sign(&secret_key_2, &message);
		assert_ok!(EcdsaAuthority::submit_new_message_root_signature(
			Origin::signed(Default::default()),
			address_2,
			signature_2.clone(),
		));
		assert_eq!(EcdsaAuthority::nonce(), nonce);
		assert!(EcdsaAuthority::new_message_root_to_sign().is_none());
		assert_eq!(
			ecdsa_authority_events(),
			vec![Event::CollectedEnoughNewMessageRootSignatures {
				commitment: Commitment {
					block_number: System::block_number() as _,
					message_root: Default::default(),
					nonce: EcdsaAuthority::nonce()
				},
				message,
				signatures: vec![(address_1, signature_1), (address_2, signature_2)]
			}]
		);
	});
}

#[test]
fn tx_fee() {
	let (secret_key_1, address_1) = gen_pair(1);
	let (_, address_2) = gen_pair(2);

	ExtBuilder::default().authorities(vec![address_1, address_2]).build().execute_with(|| {
		(2..SyncInterval::get()).for_each(run_to_block);
		run_to_block(SyncInterval::get());
		let message = [
			159, 247, 43, 185, 157, 74, 126, 205, 108, 104, 253, 73, 176, 246, 156, 154, 97, 206,
			211, 254, 16, 3, 191, 15, 171, 104, 151, 60, 37, 145, 208, 225,
		];

		// Free for first-correct signature.
		assert_eq!(
			EcdsaAuthority::submit_new_message_root_signature(
				Origin::signed(Default::default()),
				address_1,
				sign(&secret_key_1, &message),
			),
			Ok(PostDispatchInfo { actual_weight: None, pays_fee: Pays::No })
		);

		// Forbidden for submitting multiple times once the previous one succeeds.
		assert_noop!(
			EcdsaAuthority::submit_new_message_root_signature(
				Origin::signed(Default::default()),
				address_1,
				Default::default(),
			),
			EcdsaAuthorityError::AlreadySubmitted
		);

		assert_ok!(EcdsaAuthority::remove_authority(Origin::root(), address_1));
		let message = [
			226, 8, 210, 237, 239, 80, 33, 187, 89, 34, 131, 115, 232, 21, 120, 113, 61, 232, 73,
			197, 77, 209, 161, 27, 140, 82, 9, 45, 3, 98, 173, 40,
		];

		// Free for first-correct signature.
		assert_eq!(
			EcdsaAuthority::submit_authorities_change_signature(
				Origin::signed(Default::default()),
				address_1,
				sign(&secret_key_1, &message),
			),
			Ok(PostDispatchInfo { actual_weight: None, pays_fee: Pays::No })
		);

		// Forbidden for submitting multiple times once the previous one succeeds.
		assert_noop!(
			EcdsaAuthority::submit_authorities_change_signature(
				Origin::signed(Default::default()),
				address_1,
				Default::default(),
			),
			EcdsaAuthorityError::AlreadySubmitted
		);
	});
}
