// --- paritytech ---
use frame_support::{assert_noop, assert_ok, weights::PostDispatchInfo};
// --- darwinia-network ---
use crate::{mock::*, Event, *};

#[test]
fn add_authority() {
	let address = Address::repeat_byte(0);

	ExtBuilder::default().build().execute_with(|| {
		assert!(EcdsaAuthority::authorities().is_empty());
		assert_eq!(EcdsaAuthority::nonce(), 0);
		assert_ok!(EcdsaAuthority::add_authority(Origin::root(), address));
		assert_eq!(EcdsaAuthority::authorities(), vec![address]);
		assert_eq!(EcdsaAuthority::previous_authorities(), vec![]);
		assert_eq!(EcdsaAuthority::nonce(), 1);
		let message = [
			95, 104, 154, 117, 185, 44, 82, 85, 71, 213, 152, 243, 143, 82, 23, 37, 45, 55, 74,
			243, 153, 158, 202, 214, 210, 40, 252, 113, 20, 63, 77, 71,
		];
		assert_eq!(
			EcdsaAuthority::authorities_change_to_sign(),
			Some((message, Default::default()))
		);
		assert_eq!(
			ecdsa_authority_events(),
			vec![Event::CollectingAuthoritiesChangeSignatures(message)]
		);

		// Case 1.
		assert_noop!(
			EcdsaAuthority::add_authority(Origin::root(), address),
			EcdsaAuthorityError::OnAuthoritiesChange
		);
		clear_authorities_change();

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
			assert_eq!(EcdsaAuthority::nonce(), 1 + i);
			clear_authorities_change();
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
		assert_eq!(EcdsaAuthority::nonce(), 0);
		assert_ok!(EcdsaAuthority::remove_authority(Origin::root(), address_1));
		assert_eq!(EcdsaAuthority::authorities(), vec![address_2]);
		assert_eq!(EcdsaAuthority::previous_authorities(), vec![address_1, address_2]);
		assert_eq!(EcdsaAuthority::nonce(), 1);
		let message = [
			44, 25, 30, 94, 69, 250, 185, 115, 202, 60, 67, 106, 30, 177, 187, 35, 107, 25, 207,
			57, 209, 20, 165, 40, 174, 157, 168, 124, 111, 62, 83, 176,
		];
		assert_eq!(
			EcdsaAuthority::authorities_change_to_sign(),
			Some((message, Default::default()))
		);
		assert_eq!(
			ecdsa_authority_events(),
			vec![Event::CollectingAuthoritiesChangeSignatures(message)]
		);

		// Case 1.
		assert_noop!(
			EcdsaAuthority::add_authority(Origin::root(), address_1),
			EcdsaAuthorityError::OnAuthoritiesChange
		);
		clear_authorities_change();

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
		assert_eq!(EcdsaAuthority::nonce(), 0);
		assert_ok!(EcdsaAuthority::swap_authority(Origin::root(), address_1, address_2));
		assert_eq!(EcdsaAuthority::authorities(), vec![address_2]);
		assert_eq!(EcdsaAuthority::previous_authorities(), vec![address_1]);
		assert_eq!(EcdsaAuthority::nonce(), 1);
		let message = [
			80, 165, 90, 130, 101, 89, 244, 106, 39, 22, 87, 235, 108, 75, 101, 52, 41, 12, 235, 9,
			56, 188, 57, 212, 91, 99, 31, 109, 115, 68, 233, 183,
		];
		assert_eq!(
			EcdsaAuthority::authorities_change_to_sign(),
			Some((message, Default::default()))
		);
		assert_eq!(
			ecdsa_authority_events(),
			vec![Event::CollectingAuthoritiesChangeSignatures(message)]
		);

		// Case 1.
		assert_noop!(
			EcdsaAuthority::swap_authority(Origin::root(), address_2, address_1),
			EcdsaAuthorityError::OnAuthoritiesChange
		);
		clear_authorities_change();

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
			59, 15, 82, 229, 131, 148, 234, 209, 165, 229, 179, 234, 227, 103, 200, 159, 241, 53,
			137, 112, 79, 255, 63, 224, 213, 254, 10, 47, 122, 129, 109, 41,
		];
		assert_eq!(EcdsaAuthority::new_message_root_to_sign(), Some((message, Default::default())));
		assert_eq!(
			ecdsa_authority_events(),
			vec![Event::CollectingNewMessageRootSignatures(message)]
		);

		// Use a new message root while exceeding the max pending period.
		new_message_root(1);
		let offset = System::block_number() + 1;
		(offset..offset + MaxPendingPeriod::get()).for_each(|i| {
			run_to_block(i);
			assert_eq!(
				EcdsaAuthority::new_message_root_to_sign(),
				Some((message, Default::default()))
			);
		});
		run_to_block(offset + MaxPendingPeriod::get());
		let message = [
			154, 33, 89, 195, 164, 222, 169, 115, 244, 147, 76, 79, 40, 78, 145, 92, 220, 91, 73,
			233, 104, 157, 167, 222, 64, 65, 39, 221, 83, 165, 6, 228,
		];
		assert_eq!(EcdsaAuthority::new_message_root_to_sign(), Some((message, Default::default())));

		// Not allow to update the message root while authorities changing.
		assert_ok!(EcdsaAuthority::add_authority(Origin::root(), Default::default()));
		new_message_root(2);
		let offset = System::block_number() + 1;
		(offset..=offset + MaxPendingPeriod::get()).for_each(|i| {
			run_to_block(i);
			assert_eq!(
				EcdsaAuthority::new_message_root_to_sign(),
				Some((message, Default::default()))
			);
		});
	});
}

#[test]
fn submit_authorities_change_signature() {
	let (secret_key_1, address_1) = gen_pair(1);
	let (secret_key_2, address_2) = gen_pair(2);
	let (secret_key_3, address_3) = gen_pair(3);

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
		let message = [
			207, 80, 241, 175, 3, 59, 89, 65, 13, 55, 249, 77, 110, 229, 85, 220, 109, 138, 196,
			148, 202, 209, 242, 217, 244, 40, 240, 171, 115, 110, 17, 53,
		];
		assert_eq!(
			EcdsaAuthority::authorities_change_to_sign(),
			Some((message, Default::default()))
		);
		assert_eq!(
			ecdsa_authority_events(),
			vec![Event::CollectingAuthoritiesChangeSignatures(message)]
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

		// Case 3.
		let signature_3 = sign(&secret_key_3, &message);
		assert_noop!(
			EcdsaAuthority::submit_authorities_change_signature(
				Origin::signed(Default::default()),
				address_3,
				signature_3,
			),
			EcdsaAuthorityError::NotPreviousAuthority
		);

		let signature_1 = sign(&secret_key_1, &message);
		assert_ok!(EcdsaAuthority::submit_authorities_change_signature(
			Origin::signed(Default::default()),
			address_1,
			signature_1.clone(),
		));
		assert_eq!(
			EcdsaAuthority::authorities_change_to_sign(),
			Some((message, BoundedVec::try_from(vec![(address_1, signature_1.clone())]).unwrap()))
		);

		let signature_2 = sign(&secret_key_2, &message);
		assert_ok!(EcdsaAuthority::submit_authorities_change_signature(
			Origin::signed(Default::default()),
			address_2,
			signature_2.clone(),
		));
		assert!(EcdsaAuthority::authorities_change_to_sign().is_none());
		assert_eq!(
			ecdsa_authority_events(),
			vec![Event::CollectedEnoughAuthoritiesChangeSignatures((
				message,
				vec![(address_1, signature_1), (address_2, signature_2)]
			))]
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
			59, 15, 82, 229, 131, 148, 234, 209, 165, 229, 179, 234, 227, 103, 200, 159, 241, 53,
			137, 112, 79, 255, 63, 224, 213, 254, 10, 47, 122, 129, 109, 41,
		];
		assert_eq!(EcdsaAuthority::new_message_root_to_sign(), Some((message, Default::default())));
		assert_eq!(
			ecdsa_authority_events(),
			vec![Event::CollectingNewMessageRootSignatures(message)]
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

		let signature_1 = sign(&secret_key_1, &message);
		assert_ok!(EcdsaAuthority::submit_new_message_root_signature(
			Origin::signed(Default::default()),
			address_1,
			signature_1.clone(),
		));
		assert_eq!(
			EcdsaAuthority::new_message_root_to_sign(),
			Some((message, BoundedVec::try_from(vec![(address_1, signature_1.clone())]).unwrap()))
		);

		let signature_2 = sign(&secret_key_2, &message);
		assert_ok!(EcdsaAuthority::submit_new_message_root_signature(
			Origin::signed(Default::default()),
			address_2,
			signature_2.clone(),
		));
		assert!(EcdsaAuthority::new_message_root_to_sign().is_none());
		assert_eq!(
			ecdsa_authority_events(),
			vec![Event::CollectedEnoughNewMessageRootSignatures((
				message,
				vec![(address_1, signature_1), (address_2, signature_2)]
			))]
		);
	});
}

#[test]
fn tx_fee() {
	let (secret_key_1, address_1) = gen_pair(1);
	let (_, address_2) = gen_pair(2);

	ExtBuilder::default().authorities(vec![address_1, address_2]).build().execute_with(|| {
		(2..SyncInterval::get()).for_each(|i| run_to_block(i));
		run_to_block(SyncInterval::get());
		let message = [
			59, 15, 82, 229, 131, 148, 234, 209, 165, 229, 179, 234, 227, 103, 200, 159, 241, 53,
			137, 112, 79, 255, 63, 224, 213, 254, 10, 47, 122, 129, 109, 41,
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
			167, 201, 211, 207, 38, 190, 116, 179, 123, 66, 81, 106, 39, 89, 201, 78, 59, 3, 100,
			51, 179, 121, 18, 192, 243, 120, 61, 167, 48, 135, 125, 32,
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
