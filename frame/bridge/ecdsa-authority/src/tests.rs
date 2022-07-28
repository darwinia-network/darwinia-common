// --- paritytech ---
use frame_support::{assert_noop, assert_ok};
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
		assert_eq!(EcdsaAuthority::nonce(), 1);
		let message = [
			174, 100, 4, 94, 82, 79, 48, 42, 207, 105, 194, 101, 109, 239, 60, 24, 73, 199, 88, 37,
			51, 111, 217, 230, 235, 89, 84, 199, 89, 119, 21, 159,
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
		assert_eq!(EcdsaAuthority::authorities(), vec![address_1, address_2]);
		assert_eq!(EcdsaAuthority::nonce(), 0);
		assert_ok!(EcdsaAuthority::remove_authority(Origin::root(), address_1));
		assert_eq!(EcdsaAuthority::authorities(), vec![address_2]);
		assert_eq!(EcdsaAuthority::nonce(), 1);
		let message = [
			108, 234, 113, 175, 5, 108, 151, 151, 10, 4, 193, 178, 252, 85, 226, 155, 30, 36, 40,
			61, 123, 54, 94, 45, 57, 108, 72, 214, 37, 30, 197, 216,
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
		assert_eq!(EcdsaAuthority::nonce(), 1);
		let message = [
			155, 114, 191, 93, 68, 113, 219, 91, 99, 71, 240, 175, 58, 249, 231, 60, 60, 80, 243,
			98, 122, 86, 24, 52, 139, 163, 232, 159, 92, 78, 65, 218,
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
			vec![Event::CollectingNewMessageRootSignatures(message)]
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

		assert_ok!(EcdsaAuthority::add_authority(Origin::root(), address_3));
		let message = [
			221, 253, 108, 189, 214, 200, 30, 115, 171, 233, 233, 167, 132, 76, 171, 243, 138, 51,
			146, 139, 168, 96, 192, 82, 237, 176, 78, 1, 157, 40, 210, 81,
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
			177, 8, 115, 132, 134, 245, 108, 127, 183, 106, 146, 37, 87, 27, 171, 191, 142, 162,
			48, 121, 156, 216, 163, 174, 142, 43, 108, 43, 90, 151, 104, 141,
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
