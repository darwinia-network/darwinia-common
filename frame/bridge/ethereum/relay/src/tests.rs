// --- substrate ---
use frame_support::assert_ok;
// --- darwinia ---
use crate::{mock::*, *};
use array_bytes::array_unchecked;

#[test]
fn test_check_test_date_decoding() {
	ExtBuilder::default().build().execute_with(|| {
		let header_things_with_proof = header_things_with_proof().unwrap();
		let suit = [
			(&header_things_with_proof[0], 0_u64, 64),
			(&header_things_with_proof[1], 1_u64, 64),
			(&header_things_with_proof[2], 2_u64, 64),
			(&header_things_with_proof[3], 3_u64, 64),
		];

		suit.iter().for_each(|(ht, n, pl)| {
			assert_eq!(&ht.header.number, n);
			assert_eq!(&ht.ethash_proof.len(), pl);
		});
	})
}

#[test]
fn test_verify_test_data_mmr_proof() {
	ExtBuilder::default().build().execute_with(|| {
		let header_things_with_proof = header_things_with_proof().unwrap();
		&header_things_with_proof.iter().for_each(|ht| {
			assert_eq!(
				EthereumRelay::verify_mmr(
					header_things_with_proof[3].header.number,
					array_unchecked!(header_things_with_proof[3].mmr_root, 0, 32).into(),
					ht.mmr_proof
						.iter()
						.map(|h| array_unchecked!(h, 0, 32).into())
						.collect(),
					vec![(
						ht.header.number,
						array_unchecked!(ht.header.hash.unwrap(), 0, 32).into(),
					)]
				),
				true
			);
		});
	})
}

#[test]
fn test_store_header() {
	ExtBuilder::default().build().execute_with(|| {
		let header_thing_with_proof = &header_things_with_proof().unwrap()[1];
		assert_eq!(header_thing_with_proof.header.number, 1);
		assert_ok!(<EthereumRelay as Relayable>::store_header(
			EthereumHeaderThing {
				header: header_thing_with_proof.header.clone(),
				mmr_root: header_thing_with_proof.mmr_root.clone(),
			}
		));

		assert_eq!(<EthereumRelay as Relayable>::best_block_number(), 1);
	})
}

#[test]
fn proposal_basic_verification_should_sucess() {
	ExtBuilder::default().build().execute_with(|| {
		let mut header_things_with_proof = header_things_with_proof().unwrap();
		header_things_with_proof.reverse();

		assert_ok!(<EthereumRelay as Relayable>::store_header(
			EthereumHeaderThing {
				header: header_things_with_proof[3].header.clone(),
				mmr_root: header_things_with_proof[3].mmr_root.clone(),
			}
		));

		assert_ok!(<EthereumRelay as Relayable>::basic_verify(
			header_things_with_proof[..3].to_vec()
		));
	})
}
