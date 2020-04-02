//! Tests for the module.

#![cfg(test)]

// --- substrate ---
use sp_runtime::{
	testing::{Digest, H256},
	traits::{Header, OnFinalize},
};

// --- darwinia ---
use crate::{mock::*, *};

#[test]
fn first_header_mmr() {
	new_test_ext().execute_with(|| {
		let parent_hash: H256 = Default::default();
		initialize_block(1, parent_hash);

		System::note_finished_extrinsics();
		HeaderMMR::on_finalize(1);

		let header = System::finalize();
		assert_eq!(
			header.digest,
			Digest {
				logs: vec![header_mmr_log(parent_hash)]
			}
		);
	});
}

#[test]
fn test_insert_header() {
	new_test_ext().execute_with(|| {
		initialize_block(1, Default::default());

		HeaderMMR::on_finalize(1);

		let mut headers = vec![];

		let mut header = System::finalize();
		headers.push(header.clone());

		for i in 2..30 {
			initialize_block(i, header.hash());

			HeaderMMR::on_finalize(i);
			header = System::finalize();
			headers.push(header.clone());
		}

		let h1 = 11 as u64;
		let h2 = 19 as u64;

		let prove_elem = headers[h1 as usize - 1].hash();

		let pos = 19;
		assert_eq!(pos, HeaderMMR::position_of(h1));
		assert_eq!(prove_elem, HeaderMMR::mmr_node_list(pos));

		let mmr_root = HeaderMMR::_find_mmr_root(headers[h2 as usize - 1].clone())
			.expect("Header mmr get failed");

		let store = ModuleMMRStore::<Test>::default();
		let mmr = MMR::<_, MMRMerge<Test>, _>::new(HeaderMMR::position_of(h2), store);

		assert_eq!(mmr.get_root().expect("Get Root Failed"), mmr_root);

		let proof = HeaderMMR::_gen_proof(h1, h2).expect("gen proof");

		let result = proof
			.verify(mmr_root, vec![(pos, prove_elem)])
			.expect("verify");
		assert!(result);
	});
}

#[test]
fn should_serialize_mmr_digest() {
	let digest = Digest {
		logs: vec![header_mmr_log(Default::default())],
	};

	assert_eq!(
		serde_json::to_string(&digest).unwrap(),
		// 0x90 is compact codec of the length 36, 0x4d4d5252 is prefix "MMRR"
		r#"{"logs":["0x00904d4d52520000000000000000000000000000000000000000000000000000000000000000"]}"#
	);
}

#[test]
fn non_system_mmr_digest_item_encoding() {
	let item = header_mmr_log(Default::default());
	let encoded = item.encode();
	assert_eq!(
		encoded,
		vec![
			0,    // type = DigestItemType::Other
			0x90, // vec length
			77, 77, 82, 82, // Prefix, *b"MMRR"
			0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
			0, 0, 0, // mmr root
		]
	);

	let decoded: DigestItem<H256> = Decode::decode(&mut &encoded[..]).unwrap();
	assert_eq!(item, decoded);
}
