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

mod data {
	// --- darwinia ---
	use crate::mock::*;

	pub struct Hashes {
		pub hash: Hash,
		pub parent_mmr_root: Hash,
	}
	impl Hashes {
		// Data source: https://crab.subscan.io/block/1?tab=log
		pub const CRAB: &'static [(&'static str, &'static str)] = &[
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
		];
		// Data source: https://darwinia.subscan.io/block/1?tab=log
		pub const DARWINIA: &'static [(&'static str, &'static str)] = &[
			(
				"0x729cb8f2cf428adcf81fe69610edda32c5711b2ff17de747e8604a3587021db8",
				"0x729cb8f2cf428adcf81fe69610edda32c5711b2ff17de747e8604a3587021db8",
			),
			(
				"0xccdfb06966ededa7a15c5bf490190690084f02d6dc35abb79bc8705f4b7e9731",
				"0x5a06da3b3f0c3d73add4513d16a4af9623d4640eb1ba383d2ddb5645074ed868",
			),
			(
				"0x77161d7ee937ffae97f6c23d26791e05f9d08b138eb22b5f43e2cee52ddf50aa",
				"0xe9b8cec5c6b0b8efec21ce632228e94911b5030b85d8f314fbbfc5010fadbe33",
			),
			(
				"0x58677a31c0f3bff168a2b4d6a181eb7ecd465455045713720672b46d7fed9c40",
				"0xa2dc16ce3c3b2b05cfc722ba21ecd8a94bb2cc695f6761d1ca87444560ca7316",
			),
			(
				"0x4b858e3d5fd8fdf6f109b145760cb9041c6256fec52871d9b5339e4a6159c1c5",
				"0xef71dca09a2e683d487c1a143c1fd7ce0cf9f23ded4ecdbd526e6e9439d09535",
			),
			(
				"0xb9cf10c067b6cd408149e0430016c91b84b1a5ffddbb6cb7d52c977571db36ed",
				"0xd0e4d61572226255f6f42ceee6a9152110a6e1a7926108d2af75a675c5460af3",
			),
			(
				"0x58a7b1064f7db57f5bed34256bd5303db9884818bc8b76096b90f88411577b0f",
				"0x0dbfd31ea7601e25f5b8bc82b1eb82ff0f532826b12eb0f9cd8910ddcf56adf9",
			),
			(
				"0xf491c827e06f0f10020e63d16426745794e894032fa31c885b08be8668cab7d9",
				"0xce90f8a4d033368ffa22b61062f1963495922090f25b738248f29d7bd3179747",
			),
			(
				"0xcf0b16a5dde57cf31798b724ac7b05ccc0f88b4667aaec45ea21a5a03466883d",
				"0x05d87a9f7d4c85a48fc18286446e10740513354cb42827414795e4ac6cd7829f",
			),
			(
				"0x554afd5150f0e6e099c7bf03d3a761f0629d1ea55ff35e5c7e1ff1ee733e9eff",
				"0x768c6230e2fd46f3b4805ea036d0d6079fa3a15822d24f6832ef7cb4549500d6",
			),
		];

		pub fn from_raw(raw_hashes: &[(&str, &str)]) -> Vec<Self> {
			raw_hashes
				.iter()
				.map(|(raw_hash, raw_parent_mmr_root)| Hashes {
					hash: array_bytes::hex_into_unchecked(raw_hash),
					parent_mmr_root: array_bytes::hex_into_unchecked(raw_parent_mmr_root),
				})
				.collect()
		}
	}
}
use data::*;

// --- crates.io ---
use codec::Encode;
use rand::prelude::*;
// --- github.com ---
use mmr::MMRStore;
// --- substrate ---
use sp_runtime::testing::Digest;
// --- darwinia ---
use crate::{mock::*, primitives::*};

#[test]
fn codec_digest_should_work() {
	assert_eq!(
		header_parent_mmr_log(Default::default()).encode(),
		vec![
			// DigestItemType::Other
			vec![0],
			// Vector length
			vec![0x90],
			// Prefix *b"MMRR"
			vec![77, 77, 82, 82],
			// MMR root
			vec![
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 0
			],
		]
		.concat()
	);
}

#[test]
fn serialize_digest_should_work() {
	assert_eq!(
		serde_json::to_string(&Digest {
			logs: vec![header_parent_mmr_log(Default::default())],
		})
		.unwrap(),
		// 0x90 is compact codec of the length 36, 0x4d4d5252 is prefix "MMRR"
		r#"{"logs":["0x00904d4d52520000000000000000000000000000000000000000000000000000000000000000"]}"#
	);
}

#[test]
fn hasher_should_work() {
	fn assert_hashes(raw_hashes: &[(&str, &str)]) {
		let hashes = Hashes::from_raw(raw_hashes);
		let mut mmr = <Mmr<RuntimeStorage, Test>>::with_size(0);

		for i in 0..hashes.len() {
			mmr.push(hashes[i].hash).unwrap();

			assert_eq!(mmr.get_root().unwrap(), hashes[i].parent_mmr_root);
		}
	}

	assert_hashes(Hashes::CRAB);
	assert_hashes(Hashes::DARWINIA);
}

#[test]
fn header_digest_should_work() {
	new_test_ext().execute_with(|| {
		let mut header = new_block();
		let mut parent_mmr_root = header.parent_hash;

		for _ in 0..10 {
			assert_eq!(
				header.digest,
				Digest {
					logs: vec![header_parent_mmr_log(parent_mmr_root)]
				}
			);

			header = new_block();
			parent_mmr_root = mmr::<RuntimeStorage>().get_root().unwrap();
		}
	});
}

#[test]
fn integration_testing_should_work() {
	let mut rng = rand::thread_rng();
	let mut leaves = (1..30).collect::<Vec<_>>();
	let mut last_leaves = (1..30).collect::<Vec<_>>();

	leaves.shuffle(&mut rng);
	last_leaves.shuffle(&mut rng);

	let data = leaves
		.into_iter()
		.zip(last_leaves.into_iter())
		.filter(|(a, b)| a < b)
		.collect::<Vec<_>>();

	for (leaf, last_leaf) in data {
		let mut headers = vec![];
		let mut ext = new_test_ext();

		ext.execute_with(|| {
			headers = run_to_block(last_leaf);
		});

		register_offchain_ext(&mut ext);

		ext.execute_with(|| {
			let on_chain =
				<Mmr<RuntimeStorage, Test>>::with_size(mmr::leaf_index_to_mmr_size(last_leaf - 1));
			let off_chain =
				<Mmr<OffchainStorage, Test>>::with_size(mmr::leaf_index_to_mmr_size(last_leaf - 1));

			assert_eq!(
				headers[leaf as usize - 1].hash(),
				<Storage<OffchainStorage, Test>>::default()
					.get_elem(mmr::leaf_index_to_pos(leaf))
					.unwrap()
					.unwrap()
			);
			assert_eq!(
				on_chain.get_root().unwrap(),
				HeaderMMR::find_parent_mmr_root(&headers[last_leaf as usize - 1]).unwrap()
			);
			assert!(off_chain
				.gen_proof(leaf)
				.unwrap()
				.verify(
					HeaderMMR::find_parent_mmr_root(&headers[last_leaf as usize - 1]).unwrap(),
					vec![(
						mmr::leaf_index_to_pos(leaf),
						headers[leaf as usize - 1].hash()
					)]
				)
				.unwrap());
		});
	}
}
