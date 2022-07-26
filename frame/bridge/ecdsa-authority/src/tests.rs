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
			173, 229, 18, 135, 146, 134, 171, 96, 131, 187, 119, 183, 95, 210, 137, 208, 249, 136,
			241, 186, 104, 150, 91, 176, 35, 7, 2, 247, 158, 63, 87, 235,
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
			210, 2, 34, 217, 126, 135, 88, 131, 212, 20, 55, 148, 78, 70, 27, 53, 239, 190, 86,
			117, 3, 43, 90, 208, 207, 197, 226, 139, 229, 85, 221, 163,
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
			49, 53, 163, 92, 173, 158, 75, 63, 241, 47, 24, 194, 9, 100, 66, 67, 0, 57, 40, 142,
			176, 167, 55, 2, 178, 173, 208, 140, 67, 0, 45, 240,
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
		for i in 2..SyncInterval::get() {
			run_to_block(i);
			assert!(EcdsaAuthority::new_message_root_to_sign().is_none());
		}
		run_to_block(SyncInterval::get());
		let message = [
			102, 190, 89, 43, 192, 253, 19, 111, 122, 166, 95, 131, 22, 69, 159, 173, 162, 46, 159,
			46, 83, 206, 2, 205, 140, 30, 252, 95, 208, 130, 34, 236,
		];
		assert_eq!(EcdsaAuthority::new_message_root_to_sign(), Some((message, Default::default())));
		assert_eq!(
			ecdsa_authority_events(),
			vec![Event::CollectingNewMessageRootSignature(message)]
		);

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
			242, 105, 250, 81, 250, 246, 5, 123, 84, 101, 113, 98, 158, 71, 43, 81, 252, 62, 3,
			216, 220, 95, 181, 205, 181, 180, 8, 241, 18, 93, 187, 150,
		];
		assert_eq!(EcdsaAuthority::new_message_root_to_sign(), Some((message, Default::default())));

		new_message_root(2);
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
			14, 219, 41, 66, 121, 156, 79, 243, 44, 71, 54, 27, 202, 26, 202, 64, 21, 31, 154, 78,
			178, 228, 238, 110, 14, 243, 116, 239, 246, 8, 126, 139,
		];
		assert_eq!(EcdsaAuthority::new_message_root_to_sign(), Some((message, Default::default())));
	});
}

#[test]
fn submit_authorities_change_signature() {
	let (secret_key_1, address_1) = gen_pair(1);
	let (secret_key_2, address_2) = gen_pair(2);
	let (secret_key_3, address_3) = gen_pair(3);

	ExtBuilder::default().authorities(vec![address_1, address_2]).build().execute_with(|| {
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
			150, 178, 198, 221, 131, 231, 216, 164, 244, 231, 54, 228, 139, 176, 101, 31, 148, 39,
			251, 187, 36, 119, 54, 250, 158, 170, 209, 158, 65, 191, 164, 127,
		];
		assert_eq!(
			EcdsaAuthority::authorities_change_to_sign(),
			Some((message, Default::default()))
		);
		assert_eq!(
			ecdsa_authority_events(),
			vec![Event::CollectingAuthoritiesChangeSignature(message)]
		);

		assert_noop!(
			EcdsaAuthority::submit_authorities_change_signature(
				Origin::signed(Default::default()),
				address_1,
				Default::default(),
			),
			EcdsaAuthorityError::BadSignature
		);

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
			102, 190, 89, 43, 192, 253, 19, 111, 122, 166, 95, 131, 22, 69, 159, 173, 162, 46, 159,
			46, 83, 206, 2, 205, 140, 30, 252, 95, 208, 130, 34, 236,
		];
		assert_eq!(EcdsaAuthority::new_message_root_to_sign(), Some((message, Default::default())));
		assert_eq!(
			ecdsa_authority_events(),
			vec![Event::CollectingNewMessageRootSignature(message)]
		);

		assert_noop!(
			EcdsaAuthority::submit_new_message_root_signature(
				Origin::signed(Default::default()),
				address_1,
				Default::default(),
			),
			EcdsaAuthorityError::BadSignature
		);

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
