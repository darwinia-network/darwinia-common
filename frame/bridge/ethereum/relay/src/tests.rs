// --- substrate ---
use frame_support::assert_ok;
// --- darwinia ---
use crate::{mock::*, *};

#[test]
fn test_store_header() {
	ExtBuilder::default().build().execute_with(|| {
		let header_thing_with_proof = &header_things_with_proof()[1];

		assert_eq!(header_thing_with_proof.header.number, 10);
		assert_ok!(<EthereumRelay as Relayable>::store_header(
			EthereumHeaderThing {
				header: header_thing_with_proof.header.clone(),
				mmr_root: header_thing_with_proof.mmr_root.clone(),
			}
		));
		assert_eq!(<EthereumRelay as Relayable>::best_block_number(), 10);
	})
}

#[test]
fn proposal_basic_verification_should_sucess() {
	ExtBuilder::default().build().execute_with(|| {
		let mut header_things_with_proof = header_things_with_proof();

		// 10 1
		header_things_with_proof.reverse();

		assert_ok!(<EthereumRelay as Relayable>::store_header(
			EthereumHeaderThing {
				header: header_things_with_proof[1].header.clone(),
				mmr_root: header_things_with_proof[1].mmr_root.clone(),
			}
		));

		// [10]
		assert_ok!(<EthereumRelay as Relayable>::basic_verify(
			header_things_with_proof[0..1].to_vec()
		));
	})
}
