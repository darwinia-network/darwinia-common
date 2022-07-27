// --- paritytech ---
use frame_support::{assert_noop, assert_ok};
// --- darwinia-network ---
use crate::{mock::*, Event, *};

#[test]
fn add_authority() {
	let address = Address::repeat_byte(0);

	ExtBuilder::default().build().execute_with(|| {
		// Case 1.
		assert!(EcdsaAuthority::authorities().is_empty());
		assert_eq!(EcdsaAuthority::nonce(), 0);
		assert_ok!(EcdsaAuthority::add_authority(Origin::root(), address));
		assert_eq!(EcdsaAuthority::authorities(), vec![address]);
		assert_eq!(EcdsaAuthority::nonce(), 1);
		let message = [
			166, 80, 52, 161, 60, 39, 155, 164, 20, 0, 64, 191, 253, 155, 55, 205, 154, 180, 85,
			72, 37, 72, 222, 120, 180, 148, 249, 153, 51, 235, 141, 239,
		];
		assert_eq!(
			EcdsaAuthority::authorities_change_to_sign(),
			Some((message, Default::default()))
		);
		assert_eq!(
			ecdsa_authority_events(),
			vec![Event::CollectingAuthoritiesChangeSignature(message)]
		);

		// Case 2.
		assert_noop!(
			EcdsaAuthority::add_authority(Origin::root(), address),
			EcdsaAuthorityError::OnAuthoritiesChange
		);
		clear_authorities_change();

		// Case 3.
		assert_noop!(
			EcdsaAuthority::add_authority(Origin::signed(Default::default()), address),
			DispatchError::BadOrigin
		);

		// Case 4.
		assert_noop!(
			EcdsaAuthority::add_authority(Origin::root(), address),
			EcdsaAuthorityError::AuthorityExisted
		);

		// Case 5.
		for i in 1..MaxAuthorities::get() {
			assert_ok!(EcdsaAuthority::add_authority(Origin::root(), Address::repeat_byte(i as _)));
			assert_eq!(EcdsaAuthority::nonce(), 1 + i);
			clear_authorities_change();
		}
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
		// Case 1.
		assert_eq!(EcdsaAuthority::authorities(), vec![address_1, address_2]);
		assert_eq!(EcdsaAuthority::nonce(), 0);
		assert_ok!(EcdsaAuthority::remove_authority(Origin::root(), address_1));
		assert_eq!(EcdsaAuthority::authorities(), vec![address_2]);
		assert_eq!(EcdsaAuthority::nonce(), 1);
		let message = [
			31, 184, 183, 33, 195, 43, 32, 46, 109, 42, 9, 39, 226, 164, 78, 90, 44, 123, 153, 162,
			53, 27, 104, 80, 63, 107, 29, 40, 250, 163, 142, 171,
		];
		assert_eq!(
			EcdsaAuthority::authorities_change_to_sign(),
			Some((message, Default::default()))
		);
		assert_eq!(
			ecdsa_authority_events(),
			vec![Event::CollectingAuthoritiesChangeSignature(message)]
		);

		// Case 2.
		assert_noop!(
			EcdsaAuthority::add_authority(Origin::root(), address_1),
			EcdsaAuthorityError::OnAuthoritiesChange
		);
		clear_authorities_change();

		// Case 3.
		assert_noop!(
			EcdsaAuthority::remove_authority(Origin::signed(Default::default()), address_2),
			DispatchError::BadOrigin
		);

		// Case 4.
		assert_noop!(
			EcdsaAuthority::remove_authority(Origin::root(), address_1),
			EcdsaAuthorityError::NotAuthority
		);

		// Case 5.
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
		// Case 1.
		assert_eq!(EcdsaAuthority::authorities(), vec![address_1]);
		assert_eq!(EcdsaAuthority::nonce(), 0);
		assert_ok!(EcdsaAuthority::swap_authority(Origin::root(), address_1, address_2));
		assert_eq!(EcdsaAuthority::authorities(), vec![address_2]);
		assert_eq!(EcdsaAuthority::nonce(), 1);
		let message = [
			247, 205, 122, 93, 139, 169, 77, 15, 141, 225, 69, 158, 253, 229, 5, 33, 120, 69, 151,
			241, 150, 172, 51, 136, 59, 108, 107, 171, 36, 34, 109, 182,
		];
		assert_eq!(
			EcdsaAuthority::authorities_change_to_sign(),
			Some((message, Default::default()))
		);
		assert_eq!(
			ecdsa_authority_events(),
			vec![Event::CollectingAuthoritiesChangeSignature(message)]
		);

		// Case 2.
		assert_noop!(
			EcdsaAuthority::swap_authority(Origin::root(), address_2, address_1),
			EcdsaAuthorityError::OnAuthoritiesChange
		);
		clear_authorities_change();

		// Case 3.
		assert_noop!(
			EcdsaAuthority::swap_authority(Origin::signed(1), address_2, address_1),
			DispatchError::BadOrigin
		);

		// Case 4.
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
		for i in 2..SyncInterval::get() {
			run_to_block(i);
			assert!(EcdsaAuthority::new_message_root_to_sign().is_none());
		}
		run_to_block(SyncInterval::get());
		let message = [
			177, 8, 115, 132, 134, 245, 108, 127, 183, 106, 146, 37, 87, 27, 171, 191, 142, 162,
			48, 121, 156, 216, 163, 174, 142, 43, 108, 43, 90, 151, 104, 141,
		];
		assert_eq!(EcdsaAuthority::new_message_root_to_sign(), Some((message, Default::default())));
		assert_eq!(
			ecdsa_authority_events(),
			vec![Event::CollectingNewMessageRootSignature(message)]
		);

		// Use a new message root while exceeding the max pending period.
		new_message_root(1);
		let offset = System::block_number() + 1;
		for i in offset..offset + MaxPendingPeriod::get() {
			run_to_block(i);
			assert_eq!(
				EcdsaAuthority::new_message_root_to_sign(),
				Some((message, Default::default()))
			);
		}
		run_to_block(offset + MaxPendingPeriod::get());
		let message = [
			172, 254, 187, 250, 52, 53, 75, 252, 65, 62, 14, 232, 176, 239, 189, 167, 68, 52, 3,
			158, 32, 166, 210, 236, 173, 29, 129, 129, 254, 9, 7, 229,
		];
		assert_eq!(EcdsaAuthority::new_message_root_to_sign(), Some((message, Default::default())));

		// Not allow to update the message root while authorities changing.
		assert_ok!(EcdsaAuthority::add_authority(Origin::root(), Default::default()));
		new_message_root(2);
		let offset = System::block_number() + 1;
		for i in offset..=offset + MaxPendingPeriod::get() {
			run_to_block(i);
			assert_eq!(
				EcdsaAuthority::new_message_root_to_sign(),
				Some((message, Default::default()))
			);
		}
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

		// Case 2.
		assert_ok!(EcdsaAuthority::add_authority(Origin::root(), address_3));
		let message = [
			171, 151, 18, 33, 161, 152, 40, 140, 39, 231, 61, 172, 224, 239, 228, 158, 100, 128,
			74, 220, 26, 89, 246, 82, 47, 58, 169, 246, 178, 41, 197, 11,
		];
		assert_eq!(
			EcdsaAuthority::authorities_change_to_sign(),
			Some((message, Default::default()))
		);
		assert_eq!(
			ecdsa_authority_events(),
			vec![Event::CollectingAuthoritiesChangeSignature(message)]
		);

		// Case 3.
		assert_noop!(
			EcdsaAuthority::submit_authorities_change_signature(
				Origin::signed(Default::default()),
				address_1,
				Default::default(),
			),
			EcdsaAuthorityError::BadSignature
		);

		// Case 4.
		let signature_3 = sign(&secret_key_3, &message);
		assert_noop!(
			EcdsaAuthority::submit_authorities_change_signature(
				Origin::signed(Default::default()),
				address_3,
				signature_3,
			),
			EcdsaAuthorityError::NotPreviousAuthority
		);

		// Case 5.
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

		// Case 6.
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

		// Case 2.
		run_to_block(SyncInterval::get());
		let message = [
			177, 8, 115, 132, 134, 245, 108, 127, 183, 106, 146, 37, 87, 27, 171, 191, 142, 162,
			48, 121, 156, 216, 163, 174, 142, 43, 108, 43, 90, 151, 104, 141,
		];
		assert_eq!(EcdsaAuthority::new_message_root_to_sign(), Some((message, Default::default())));
		assert_eq!(
			ecdsa_authority_events(),
			vec![Event::CollectingNewMessageRootSignature(message)]
		);

		// Case 3.
		assert_noop!(
			EcdsaAuthority::submit_new_message_root_signature(
				Origin::signed(Default::default()),
				address_1,
				Default::default(),
			),
			EcdsaAuthorityError::BadSignature
		);

		// Case 4.
		let signature_3 = sign(&secret_key_3, &message);
		assert_noop!(
			EcdsaAuthority::submit_new_message_root_signature(
				Origin::signed(Default::default()),
				address_3,
				signature_3,
			),
			EcdsaAuthorityError::NotAuthority
		);

		// Case 5.
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

		// Case 6.
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
