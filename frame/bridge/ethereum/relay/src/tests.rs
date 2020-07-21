use crate::mock::*;
use std::boxed::Box;

use codec::Encode;

use array_bytes::array_unchecked;
use darwinia_support::relay::RawHeaderThing;
use darwinia_support::relay::Relayable;
use frame_support::assert_ok;

#[test]
fn test_check_test_date_decoding() {
	ExtBuilder::default().build().execute_with(|| {
		let header_things = header_things().unwrap();
		let suit = [
			(&header_things[0], 0_u64, 64),
			(&header_things[1], 1_u64, 64),
			(&header_things[2], 2_u64, 64),
			(&header_things[3], 3_u64, 64),
		];

		suit.iter().for_each(|(ht, n, pl)| {
			assert_eq!(&ht.eth_header.number, n);
			assert_eq!(&ht.ethash_proof.len(), pl);
		});
	})
}

#[test]
fn test_verify_test_data_mmr_proof() {
	ExtBuilder::default().build().execute_with(|| {
		let header_things = header_things().unwrap();
		&header_things.iter().for_each(|ht| {
			assert_eq!(
				EthRelay::verify_mmr(
					header_things[3].eth_header.number,
					array_unchecked!(header_things[3].mmr_root, 0, 32).into(),
					ht.mmr_proof
						.iter()
						.map(|h| array_unchecked!(h, 0, 32).into())
						.collect(),
					vec![(
						ht.eth_header.number,
						array_unchecked!(ht.eth_header.hash.unwrap(), 0, 32).into(),
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
		let header_thing = &header_things().unwrap()[1];
		assert_eq!(header_thing.eth_header.number, 1);
		assert_ok!(<EthRelay as Relayable>::store_header(
			Box::new(header_thing).encode()
		));

		assert_eq!(<EthRelay as Relayable>::best_block_number(), 1);
	})
}
#[test]
fn test_verify_raw_header_thing() {
	ExtBuilder::default().build().execute_with(|| {
		let header_thing = &header_things().unwrap()[2];
		assert_ok!(<EthRelay as Relayable>::verify_raw_header_thing(
			Box::new(header_thing).encode(),
			false
		));
	})
}

#[test]
fn test_verify_raw_header_thing_chain() {
	ExtBuilder::default().build().execute_with(|| {
		let mut header_things = header_things().unwrap();
		header_things.reverse();

		assert_ok!(<EthRelay as Relayable>::store_header(
			Box::new(&header_things[3]).encode()
		));

		assert_ok!(<EthRelay as Relayable>::verify_raw_header_thing_chain(
			header_things[..3]
				.iter()
				.map(|ht| Box::new(ht).encode())
				.collect::<Vec<RawHeaderThing>>()
		));
	})
}
