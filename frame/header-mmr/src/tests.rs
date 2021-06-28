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
	// --- std ---
	use std::fs::File;
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

	// 0~10000 MMR nodes from Darwinia
	pub fn darwinia_mmr() -> Vec<Hash> {
		serde_json::from_reader::<_, Vec<Hash>>(File::open("mmr.json").unwrap()).unwrap()
	}
}
use data::*;

// --- crates.io ---
use codec::Encode;
use rand::prelude::*;
use serde_json::Value;
// --- github.com ---
use mmr::MMRStore;
// --- substrate ---
use sp_runtime::testing::Digest;
// --- darwinia ---
use crate::{mock::*, primitives::*, *};
use darwinia_header_mmr_rpc_runtime_api::*;

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
			headers = run_to_block_from_genesis(last_leaf);
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
				HeaderMmr::find_parent_mmr_root(&headers[last_leaf as usize - 1]).unwrap()
			);
			assert!(off_chain
				.gen_proof(leaf)
				.unwrap()
				.verify(
					HeaderMmr::find_parent_mmr_root(&headers[last_leaf as usize - 1]).unwrap(),
					vec![(
						mmr::leaf_index_to_pos(leaf),
						headers[leaf as usize - 1].hash()
					)]
				)
				.unwrap());
		});
	}
}

#[test]
fn prune_darwinia_should_work() {
	// Perform `set_code` at block `500`
	//
	// The new runtime logic will be applied at block `501`
	const BLOCK_NUMBER: NodeIndex = 500;
	const PRUNING_STEP: NodeIndex = 10;

	// The `500` leaves' MMR size is `995` the last position is `994`
	// So, the pruning will stop at `994`
	let size = mmr::leaf_index_to_mmr_size(BLOCK_NUMBER);
	let mmr = darwinia_mmr()[..size as usize].to_vec();
	let mut ext = new_test_ext();

	register_offchain_ext(&mut ext);

	ext.execute_with(|| {
		System::set_block_number(BLOCK_NUMBER);

		migration::initialize_new_mmr_state::<Test>(size, mmr, PRUNING_STEP);

		// Able to `gen_proof` from for runtime DB `MMRNodeList`, during pruning
		{
			let block_number = System::block_number();
			let off_chain =
				<Mmr<OffchainStorage, Test>>::with_size(mmr::leaf_index_to_mmr_size(block_number));

			for index in 0..block_number {
				assert!(off_chain.gen_proof(index).is_ok());
			}
		}

		// A single pruning step
		{
			for position in (0 as NodeIndex)..PRUNING_STEP {
				assert!(<MMRNodeList<Test>>::contains_key(position));
			}
			// Ignore parent hash here, just pruning testing
			new_block();
			for position in (0 as NodeIndex)..PRUNING_STEP {
				assert!(!<MMRNodeList<Test>>::contains_key(position));
			}
			for position in PRUNING_STEP..size {
				assert!(<MMRNodeList<Test>>::contains_key(position));
			}
		}

		// Pruning `MMRNodeList` to empty
		{
			for _ in 0..size / PRUNING_STEP {
				new_block();
			}
			for position in 0..size {
				assert!(!<MMRNodeList<Test>>::contains_key(position));
			}
		}
	});

	register_offchain_ext(&mut ext);

	// `gen_proof` only works in off-chain context now
	ext.execute_with(|| {
		let block_number = System::block_number();
		let off_chain =
			<Mmr<OffchainStorage, Test>>::with_size(mmr::leaf_index_to_mmr_size(block_number));

		for index in 0..block_number {
			assert!(off_chain.gen_proof(index).is_ok());
		}

		fn assert_rpc_result(rpc: &str, params: [NodeIndex; 2]) {
			let json = serde_json::from_str::<Value>(rpc).unwrap();

			assert_eq!(
				RuntimeDispatchInfo {
					mmr_size: json["mmrSize"].as_str().unwrap().parse().unwrap(),
					proof: Proof(
						json["proof"]
							.as_str()
							.unwrap()
							.trim_matches(&['[', ']'] as &[_])
							.split(',')
							.map(|hex| array_bytes::hex_into_unchecked(hex.trim()))
							.collect()
					),
				},
				HeaderMmr::gen_proof_rpc(params[0], params[1])
			);
		}

		// λ subalfred send-rpc --method headerMMR_genProof --params '[15, 27]' --uri https://rpc.darwinia.network
		assert_rpc_result(
			r#"{
				"mmrSize": "53",
				"proof": "[0x53239551406c7443ba08a8bf3295b5808f1117809fdc941251859764454b6127,0x9e9af6c7c85c72eca5f6eab91e51f46200d78ac781fce88fd320884af75c1fb0,0xdc74423b38518438f55c30c3039ea411c1884994164133d20b63f0ac158f693c,0xce90f8a4d033368ffa22b61062f1963495922090f25b738248f29d7bd3179747,0x9ef94a314854595c579175bf8a50c377ac674c7daab1b7ce73cd479585bf2243]"
			}"#,
			[15, 27],
		);
		// λ subalfred send-rpc --method headerMMR_genProof --params '[12, 345]' --uri https://rpc.darwinia.network
		assert_rpc_result(
			r#"{
				"mmrSize": "687",
				"proof": "[0xf470e3bc1d1675ae7178241dd2e3a86c9a5eb8069138aec2592f5e5e22e8f7c4,0x2176cea5ee7d347933c5b6e59e71868260da55e221c249981daba74140ba9c18,0xdc74423b38518438f55c30c3039ea411c1884994164133d20b63f0ac158f693c,0xce90f8a4d033368ffa22b61062f1963495922090f25b738248f29d7bd3179747,0xd75b80f04d876c18d00ca3c218a11975b3033efb21eca27053ba0ee0444c4734,0x889370a0a23fcb1f5b47c9ed85dd79396fce408e14640a19b897b6defb4685a7,0xdfd961454b76255ea5391ba747e64c1993b36df0eef9f70cbdb80f4839bc607f,0x42847d00d10e4ab1097281098bb01f6339ea0faf2f9df08734f471f241bd2067,0x969c918e8525124ffd0d93a29135b9420e898f277dfc67385900d8e872a9ea19]"
			}"#,
			[12, 345],
		);
		// λ subalfred send-rpc --method headerMMR_genProof --params '[50, 500]' --uri https://rpc.darwinia.network
		assert_rpc_result(
			r#"{
				"mmrSize": "995",
				"proof": "[0x6a85080d832bb8c61733c6a1d5205fa2f87514438fa46e9b446917aa8b0f28c7,0x691fa5a6305fdc0c1117cd6eacb9d45d5aed170a98d2143b9e616fc884bd9391,0x13e0e7836729160b07fa9ce24ab44e5bb2d2f25c24f29a604ea70df81c89622d,0x6beb56e3af536d5e4837f7469c22db15fa4a7bae0ce3bbfbf35a469e4ea81ba2,0x44cadf7ac096628db0cd3ccb827a31972cbfcef13de9563ca7c643242ff5f982,0x05d87c7306675244aee90748fb31776b4fbb65fd9082c198944bf03dbace2836,0xdfd961454b76255ea5391ba747e64c1993b36df0eef9f70cbdb80f4839bc607f,0x42847d00d10e4ab1097281098bb01f6339ea0faf2f9df08734f471f241bd2067,0xdf3d2d2278ae5a93db21bcbfcbc3dad584729044afab8b5450ed8d571c151ce7]"
			}"#,
			[50, 500],
		);
	});
}
