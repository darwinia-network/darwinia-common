// --- substrate ---
use frame_support::assert_ok;
// --- darwinia ---
use crate::{mock::*, *};

#[test]
fn test_store_header() {
	ExtBuilder::default().build().execute_with(|| {
		let header_thing_with_proof = &proposal_of_game_with_id(2, 0)[0];

		assert_eq!(header_thing_with_proof.header.number, 3);
		assert_ok!(<EthereumRelay as Relayable>::store_header(
			EthereumHeaderThing {
				header: header_thing_with_proof.header.clone(),
				mmr_root: header_thing_with_proof.mmr_root.clone(),
			}
		));
		assert_eq!(<EthereumRelay as Relayable>::best_block_number(), 3);
	})
}

#[test]
fn proposal_basic_verification_should_sucess() {
	ExtBuilder::default().build().execute_with(|| {
		for &game in [2].iter() {
			for id in 0..=2 {
				// eprintln!("{}, {}", game, id);
				assert_ok!(<EthereumRelay as Relayable>::basic_verify(
					proposal_of_game_with_id(game, id)
				));
			}
		}
	})
}
