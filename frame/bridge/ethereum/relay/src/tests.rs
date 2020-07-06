use crate::mock::*;
use std::boxed::Box;

use codec::Encode;

use darwinia_support::relay::RawHeaderThing;
use darwinia_support::relay::Relayable;
use frame_support::assert_ok;

#[test]
fn test_check_test_date_decoding() {
	ExtBuilder::default().build().execute_with(|| {
		let header_thing = from_file_to_eth_header_thing("./src/test-data/0.json");
		assert_eq!(header_thing.header.number, 0);
		let header_thing = from_file_to_eth_header_thing("./src/test-data/1.json");
		assert_eq!(header_thing.header.number, 1);
		let header_thing = from_file_to_eth_header_thing("./src/test-data/2.json");
		assert_eq!(header_thing.header.number, 2);
		let header_thing = from_file_to_eth_header_thing("./src/test-data/3.json");
		assert_eq!(header_thing.header.number, 3);
	})
}
#[test]
fn test_store_header() {
	ExtBuilder::default().build().execute_with(|| {
		let header_thing = from_file_to_eth_header_thing("./src/test-data/1.json");

		assert_eq!(header_thing.header.number, 1);

		assert_ok!(<EthRelay as Relayable>::store_header(
			Box::new(header_thing).encode()
		));

		assert_eq!(<EthRelay as Relayable>::best_block_number(), 1);
	})
}
#[test]
fn test_header_existed() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(<EthRelay as Relayable>::header_existed(0), false);
		let header_thing = from_file_to_eth_header_thing("./src/test-data/0.json");

		assert_ok!(<EthRelay as Relayable>::store_header(
			Box::new(header_thing).encode()
		));

		assert_eq!(<EthRelay as Relayable>::header_existed(0), true);
	})
}
#[test]
fn test_verify_raw_header_thing() {
	ExtBuilder::default().build().execute_with(|| {
		let header_thing = from_file_to_eth_header_thing("./src/test-data/2.json");

		assert_ok!(<EthRelay as Relayable>::verify_raw_header_thing(
			Box::new(header_thing).encode()
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
