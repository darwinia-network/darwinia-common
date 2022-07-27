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
			1, 199, 159, 35, 170, 22, 99, 168, 20, 181, 233, 7, 238, 255, 161, 76, 85, 233, 68,
			200, 79, 243, 86, 114, 49, 7, 176, 174, 235, 59, 154, 116,
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
			42, 227, 101, 70, 211, 109, 181, 49, 250, 122, 203, 58, 67, 23, 246, 117, 156, 147, 74,
			236, 122, 87, 157, 63, 94, 30, 111, 111, 155, 9, 50, 225,
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
			10, 192, 195, 127, 18, 237, 195, 186, 62, 232, 215, 74, 12, 6, 43, 250, 99, 69, 230,
			19, 99, 217, 98, 78, 137, 167, 227, 106, 226, 153, 109, 176,
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
			44, 39, 91, 214, 145, 66, 50, 252, 206, 22, 236, 35, 31, 233, 168, 167, 26, 79, 243,
			40, 129, 9, 132, 147, 59, 55, 156, 139, 133, 52, 248, 36,
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
			128, 132, 19, 156, 164, 82, 223, 170, 252, 100, 231, 62, 20, 58, 20, 161, 227, 57, 118,
			142, 203, 69, 192, 171, 236, 176, 11, 80, 155, 71, 193, 204,
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
			245, 26, 220, 42, 236, 140, 55, 124, 244, 182, 112, 24, 57, 97, 0, 139, 47, 101, 158,
			98, 20, 106, 1, 68, 112, 134, 165, 108, 78, 55, 75, 165,
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
			150, 17, 199, 54, 6, 229, 149, 12, 9, 75, 179, 16, 226, 37, 83, 39, 109, 157, 95, 169,
			168, 118, 12, 39, 192, 58, 32, 48, 243, 91, 64, 126,
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
			44, 39, 91, 214, 145, 66, 50, 252, 206, 22, 236, 35, 31, 233, 168, 167, 26, 79, 243,
			40, 129, 9, 132, 147, 59, 55, 156, 139, 133, 52, 248, 36,
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
