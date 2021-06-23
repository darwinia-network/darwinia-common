// This file is part of Darwinia.
//
// Copyright (C) 2018-2021 Darwinia Network
// SPDX-License-Identifier: GPL-3.0
//
// Darwinia is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Darwinia is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

//! Tests for the module.

// --- crates.io ---
use codec::{Decode, Encode};
// --- github.com ---
use mmr::{MMRStore, Merge};
// --- substrate ---
use sp_runtime::{generic::DigestItem, testing::Digest};
// --- darwinia ---
use crate::{
	mock::{Hash, *},
	primitives::*,
};

fn headers_n_roots() -> Vec<(Hash, Hash)> {
	[
		(
			"0x34f61bfda344b3fad3c3e38832a91448b3c613b199eb23e5110a635d71c13c65",
			"0x34f61bfda344b3fad3c3e38832a91448b3c613b199eb23e5110a635d71c13c65",
		),
		(
			"0x70d641860d40937920de1eae29530cdc956be830f145128ebb2b496f151c1afb",
			"0x3aafcc7fe12cb8fad62c261458f1c19dba0a3756647fa4e8bff6e248883938be",
		),
		(
			"0x12e69454d992b9b1e00ea79a7fa1227c889c84d04b7cd47e37938d6f69ece45d",
			"0x7ddf10d67045173e3a59efafb304495d9a7c84b84f0bc0235470a5345e32535d",
		),
		(
			"0x3733bd06905e128d38b9b336207f301133ba1d0a4be8eaaff6810941f0ad3b1a",
			"0x488e9565547fec8bd36911dc805a7ed9f3d8d1eacabe429c67c6456933c8e0a6",
		),
		(
			"0x3d7572be1599b488862a1b35051c3ef081ba334d1686f9957dbc2afd52bd2028",
			"0x6e0c4ab56e0919a7d45867fcd1216e2891e06994699eb838386189e9abda55f1",
		),
		(
			"0x2a04add3ecc3979741afad967dfedf807e07b136e05f9c670a274334d74892cf",
			"0x293b49420345b185a1180e165c76f76d8cc28fe46c1f6eb4a96959253b571ccd",
		),
		(
			"0xc58e247ea35c51586de2ea40ac6daf90eac7ac7b2f5c88bbc7829280db7890f1",
			"0x2dee5b87a481a9105cb4b2db212a1d8031d65e9e6e68dc5859bef5e0fdd934b2",
		),
		(
			"0x2cf0262f0a8b00cad22afa04d70fb0c1dbb2eb4a783beb7c5e27bd89015ff573",
			"0x54be644b5b3291dd9ae9598b49d1f986e4ebd8171d5e89561b2a921764c7b17c",
		),
		(
			"0x05370d06def89f11486c994c459721b4bd023ff8c2347f3187e9f42ef39bddab",
			"0x620dbc3a28888da8b17ebf5b18dba53794621463e2bbabcf88b8cbc97508ab38",
		),
		(
			"0xc0c8c3f7dc9cdfa87d2433bcd72a744d634524a5ff76e019e44ea450476bac99",
			"0xa94bf2a4e0437c236c68675403d980697cf7c9b0f818a622cb40199db5e12cf8",
		),
	]
	.iter()
	.map(|(a, b)| {
		(
			array_bytes::hex_into_unchecked(a),
			array_bytes::hex_into_unchecked(b),
		)
	})
	.collect()
}

#[test]
fn first_header_mmr() {
	new_test_ext().execute_with(|| {
		let header = new_block();

		assert_eq!(
			header.digest,
			Digest {
				logs: vec![header_mmr_log(header.parent_hash)]
			}
		);
	});
}

#[test]
fn test_insert_header() {
	let mut headers = vec![];
	let mut ext = new_test_ext();

	ext.execute_with(|| {
		let mut parent_hash;

		headers.push({
			let header = new_block();

			parent_hash = header.hash();

			header
		});

		for _ in 2..=19 {
			headers.push({
				let header = new_block_with_parent_hash(parent_hash);

				parent_hash = header.hash();

				header
			});
		}
	});

	ext.persist_offchain_overlay();
	register_offchain_ext(&mut ext);

	ext.execute_with(|| {
		let h1 = 11;
		let h2 = 19;
		let position = 19;

		assert_eq!(position, mmr::leaf_index_to_pos(h1));

		let prove_elem = headers[h1 as usize - 1].hash();
		let parent_mmr_root = HeaderMMR::find_parent_mmr_root(&headers[h2 as usize - 1]).unwrap();
		let on_chain = <Mmr<RuntimeStorage, Test>>::with_size(mmr::leaf_index_to_mmr_size(h2 - 1));
		let off_chain =
			<Mmr<OffchainStorage, Test>>::with_size(mmr::leaf_index_to_mmr_size(h2 - 1));

		assert_eq!(
			prove_elem,
			<Storage<OffchainStorage, Test>>::default()
				.get_elem(position)
				.unwrap()
				.unwrap()
		);
		assert_eq!(on_chain.get_root().unwrap(), parent_mmr_root);
		assert!(off_chain
			.gen_proof(h1)
			.unwrap()
			.verify(parent_mmr_root, vec![(position, prove_elem)])
			.unwrap());
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

	let decoded = <DigestItem<Hash>>::decode(&mut &encoded[..]).unwrap();
	assert_eq!(item, decoded);
}

#[test]
fn test_mmr_root() {
	let headers_n_roots = headers_n_roots();
	let mut mmr = <Mmr<RuntimeStorage, Test>>::with_size(0);

	(0..10).for_each(|i| {
		mmr.push(headers_n_roots[i].0).unwrap();

		assert_eq!(
			mmr.get_root().expect("Failed to get root"),
			headers_n_roots[i].1
		);
	});
}

#[test]
fn test_mmr_merge() {
	let headers_n_roots = headers_n_roots();

	assert_eq!(
		<Hasher<Test>>::merge(&headers_n_roots[0].0, &headers_n_roots[1].0),
		array_bytes::hex_into_unchecked(
			"0x3aafcc7fe12cb8fad62c261458f1c19dba0a3756647fa4e8bff6e248883938be"
		)
	);
}
