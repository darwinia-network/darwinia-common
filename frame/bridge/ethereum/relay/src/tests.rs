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
		let header_thing = from_file_to_eth_header_thing("./src/test-data/0.json");
		assert_eq!(header_thing.eth_header.number, 0);
		let header_thing = from_file_to_eth_header_thing("./src/test-data/1.json");
		assert_eq!(header_thing.eth_header.number, 1);
		assert_eq!(header_thing.ethash_proof.len(), 64);

		let header_thing = from_file_to_eth_header_thing("./src/test-data/2.json");
		assert_eq!(header_thing.eth_header.number, 2);
		assert_eq!(header_thing.ethash_proof.len(), 64);

		let header_thing = from_file_to_eth_header_thing("./src/test-data/3.json");
		assert_eq!(header_thing.eth_header.number, 3);
		assert_eq!(header_thing.ethash_proof.len(), 64);

		let header_thing = from_file_to_eth_header_thing("./src/test-data/8996776.json");
		assert_eq!(header_thing.eth_header.number, 8996776);
		assert_eq!(header_thing.ethash_proof.len(), 64);
		assert_eq!(
			EthRelay::verify_block_seal(&header_thing.eth_header, &header_thing.ethash_proof),
			true
		);
	})
}

#[test]
fn test_verify_test_data_mmr_proof() {
	ExtBuilder::default().build().execute_with(|| {
		let header_thing_1 = from_file_to_eth_header_thing("./src/test-data/1.json");
		let header_thing_2 = from_file_to_eth_header_thing("./src/test-data/2.json");
		let header_thing_3 = from_file_to_eth_header_thing("./src/test-data/3.json");
		assert_eq!(
			EthRelay::verify_mmr(
				header_thing_3.eth_header.number,
				array_unchecked!(header_thing_3.mmr_root, 0, 32).into(),
				header_thing_2
					.mmr_proof
					.iter()
					.map(|h| array_unchecked!(h, 0, 32).into())
					.collect(),
				vec![(
					header_thing_2.eth_header.number,
					array_unchecked!(header_thing_2.eth_header.hash.unwrap(), 0, 32).into(),
				)]
			),
			true
		);
		assert_eq!(
			EthRelay::verify_mmr(
				header_thing_3.eth_header.number,
				array_unchecked!(header_thing_3.mmr_root, 0, 32).into(),
				header_thing_1
					.mmr_proof
					.iter()
					.map(|h| array_unchecked!(h, 0, 32).into())
					.collect(),
				vec![(
					header_thing_1.eth_header.number,
					array_unchecked!(header_thing_1.eth_header.hash.unwrap(), 0, 32).into(),
				)]
			),
			true
		);
	})
}

#[test]
fn test_store_header() {
	ExtBuilder::default().build().execute_with(|| {
		let header_thing = from_file_to_eth_header_thing("./src/test-data/1.json");

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
		let header_thing = from_file_to_eth_header_thing("./src/test-data/2.json");

		assert_ok!(<EthRelay as Relayable>::verify_raw_header_thing(
			Box::new(header_thing).encode(),
			false
		));
	})
}

#[test]
fn test_verify_raw_header_thing_chain() {
	ExtBuilder::default().build().execute_with(|| {
		let header_thing = from_file_to_eth_header_thing("./src/test-data/0.json");

		assert_ok!(<EthRelay as Relayable>::store_header(
			Box::new(header_thing).encode()
		));
		let test_data_files = vec![
			"./src/test-data/3.json",
			"./src/test-data/2.json",
			"./src/test-data/1.json",
		];
		let raw_header_thing_chain: Vec<RawHeaderThing> = test_data_files
			.iter()
			.map(|f| Box::new(from_file_to_eth_header_thing(f)).encode())
			.collect();
		assert_ok!(<EthRelay as Relayable>::verify_raw_header_thing_chain(
			raw_header_thing_chain
		));
	})
}
