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

#[cfg(test)]
mod trie_tests {
	use std::rc::Rc;

	use rand::Rng;
	use rlp::{self};

	use crate::{db::MemoryDB, proof::Proof, trie::*};

	fn assert_root(data: Vec<(&[u8], &[u8])>, hash: &str) {
		let memdb = Rc::new(MemoryDB::new());
		let mut trie = MerklePatriciaTrie::new(Rc::clone(&memdb));
		for (k, v) in data.into_iter() {
			trie.insert(k.to_vec(), v.to_vec()).unwrap();
		}
		let r = trie.root().unwrap();
		let rs = array_bytes::bytes2hex("0x", r.clone());
		assert_eq!(rs.as_str(), hash);
		let mut trie = MerklePatriciaTrie::from(Rc::clone(&memdb), &r).unwrap();
		let r2 = trie.root().unwrap();
		let rs2 = array_bytes::bytes2hex("0x", r2);
		assert_eq!(rs2.as_str(), hash);
	}

	#[test]
	fn test_root() {
		// See: https://github.com/ethereum/tests/blob/develop/TrieTests
		// Copy from trietest.json and trieanyorder.json
		assert_root(
			vec![(b"A", b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")],
			"0xd23786fb4a010da3ce639d66d5e904a11dbc02746d1ce25029e53290cabf28ab",
		);
		assert_root(
			vec![
				(b"doe", b"reindeer"),
				(b"dog", b"puppy"),
				(b"dogglesworth", b"cat"),
			],
			"0x8aad789dff2f538bca5d8ea56e8abe10f4c7ba3a5dea95fea4cd6e7c3a1168d3",
		);
		assert_root(
			vec![
				(b"do", b"verb"),
				(b"horse", b"stallion"),
				(b"doge", b"coin"),
				(b"dog", b"puppy"),
			],
			"0x5991bb8c6514148a29db676a14ac506cd2cd5775ace63c30a4fe457715e9ac84",
		);
		assert_root(
			vec![(b"foo", b"bar"), (b"food", b"bass")],
			"0x17beaa1648bafa633cda809c90c04af50fc8aed3cb40d16efbddee6fdf63c4c3",
		);

		assert_root(
			vec![(b"be", b"e"), (b"dog", b"puppy"), (b"bed", b"d")],
			"0x3f67c7a47520f79faa29255d2d3c084a7a6df0453116ed7232ff10277a8be68b",
		);
		assert_root(
			vec![(b"test", b"test"), (b"te", b"testy")],
			"0x8452568af70d8d140f58d941338542f645fcca50094b20f3c3d8c3df49337928",
		);
		assert_root(
			vec![
				(
					array_bytes::hex2bytes("0045").unwrap().as_slice(),
					array_bytes::hex2bytes("0123456789").unwrap().as_slice(),
				),
				(
					array_bytes::hex2bytes("4500").unwrap().as_slice(),
					array_bytes::hex2bytes("9876543210").unwrap().as_slice(),
				),
			],
			"0x285505fcabe84badc8aa310e2aae17eddc7d120aabec8a476902c8184b3a3503",
		);
		assert_root(
			vec![
				(b"do", b"verb"),
				(b"ether", b"wookiedoo"),
				(b"horse", b"stallion"),
				(b"shaman", b"horse"),
				(b"doge", b"coin"),
				(b"ether", b""),
				(b"dog", b"puppy"),
				(b"shaman", b""),
			],
			"0x5991bb8c6514148a29db676a14ac506cd2cd5775ace63c30a4fe457715e9ac84",
		);
		assert_root(
			vec![
				(b"do", b"verb"),
				(b"ether", b"wookiedoo"),
				(b"horse", b"stallion"),
				(b"shaman", b"horse"),
				(b"doge", b"coin"),
				(b"ether", b""),
				(b"dog", b"puppy"),
				(b"shaman", b""),
			],
			"0x5991bb8c6514148a29db676a14ac506cd2cd5775ace63c30a4fe457715e9ac84",
		);
		assert_root(
			vec![
				(
					array_bytes::hex2bytes("04110d816c380812a427968ece99b1c963dfbce6")
						.unwrap()
						.as_slice(),
					b"something",
				),
				(
					array_bytes::hex2bytes("095e7baea6a6c7c4c2dfeb977efac326af552d87")
						.unwrap()
						.as_slice(),
					b"something",
				),
				(
					array_bytes::hex2bytes("0a517d755cebbf66312b30fff713666a9cb917e0")
						.unwrap()
						.as_slice(),
					b"something",
				),
				(
					array_bytes::hex2bytes("24dd378f51adc67a50e339e8031fe9bd4aafab36")
						.unwrap()
						.as_slice(),
					b"something",
				),
				(
					array_bytes::hex2bytes("293f982d000532a7861ab122bdc4bbfd26bf9030")
						.unwrap()
						.as_slice(),
					b"something",
				),
				(
					array_bytes::hex2bytes("2cf5732f017b0cf1b1f13a1478e10239716bf6b5")
						.unwrap()
						.as_slice(),
					b"something",
				),
				(
					array_bytes::hex2bytes("31c640b92c21a1f1465c91070b4b3b4d6854195f")
						.unwrap()
						.as_slice(),
					b"something",
				),
				(
					array_bytes::hex2bytes("37f998764813b136ddf5a754f34063fd03065e36")
						.unwrap()
						.as_slice(),
					b"something",
				),
				(
					array_bytes::hex2bytes("37fa399a749c121f8a15ce77e3d9f9bec8020d7a")
						.unwrap()
						.as_slice(),
					b"something",
				),
				(
					array_bytes::hex2bytes("4f36659fa632310b6ec438dea4085b522a2dd077")
						.unwrap()
						.as_slice(),
					b"something",
				),
				(
					array_bytes::hex2bytes("62c01474f089b07dae603491675dc5b5748f7049")
						.unwrap()
						.as_slice(),
					b"something",
				),
				(
					array_bytes::hex2bytes("729af7294be595a0efd7d891c9e51f89c07950c7")
						.unwrap()
						.as_slice(),
					b"something",
				),
				(
					array_bytes::hex2bytes("83e3e5a16d3b696a0314b30b2534804dd5e11197")
						.unwrap()
						.as_slice(),
					b"something",
				),
				(
					array_bytes::hex2bytes("8703df2417e0d7c59d063caa9583cb10a4d20532")
						.unwrap()
						.as_slice(),
					b"something",
				),
				(
					array_bytes::hex2bytes("8dffcd74e5b5923512916c6a64b502689cfa65e1")
						.unwrap()
						.as_slice(),
					b"something",
				),
				(
					array_bytes::hex2bytes("95a4d7cccb5204733874fa87285a176fe1e9e240")
						.unwrap()
						.as_slice(),
					b"something",
				),
				(
					array_bytes::hex2bytes("99b2fcba8120bedd048fe79f5262a6690ed38c39")
						.unwrap()
						.as_slice(),
					b"something",
				),
				(
					array_bytes::hex2bytes("a4202b8b8afd5354e3e40a219bdc17f6001bf2cf")
						.unwrap()
						.as_slice(),
					b"something",
				),
				(
					array_bytes::hex2bytes("a94f5374fce5edbc8e2a8697c15331677e6ebf0b")
						.unwrap()
						.as_slice(),
					b"something",
				),
				(
					array_bytes::hex2bytes("a9647f4a0a14042d91dc33c0328030a7157c93ae")
						.unwrap()
						.as_slice(),
					b"something",
				),
				(
					array_bytes::hex2bytes("aa6cffe5185732689c18f37a7f86170cb7304c2a")
						.unwrap()
						.as_slice(),
					b"something",
				),
				(
					array_bytes::hex2bytes("aae4a2e3c51c04606dcb3723456e58f3ed214f45")
						.unwrap()
						.as_slice(),
					b"something",
				),
				(
					array_bytes::hex2bytes("c37a43e940dfb5baf581a0b82b351d48305fc885")
						.unwrap()
						.as_slice(),
					b"something",
				),
				(
					array_bytes::hex2bytes("d2571607e241ecf590ed94b12d87c94babe36db6")
						.unwrap()
						.as_slice(),
					b"something",
				),
				(
					array_bytes::hex2bytes("f735071cbee190d76b704ce68384fc21e389fbe7")
						.unwrap()
						.as_slice(),
					b"something",
				),
				(
					array_bytes::hex2bytes("04110d816c380812a427968ece99b1c963dfbce6")
						.unwrap()
						.as_slice(),
					b"",
				),
				(
					array_bytes::hex2bytes("095e7baea6a6c7c4c2dfeb977efac326af552d87")
						.unwrap()
						.as_slice(),
					b"",
				),
				(
					array_bytes::hex2bytes("0a517d755cebbf66312b30fff713666a9cb917e0")
						.unwrap()
						.as_slice(),
					b"",
				),
				(
					array_bytes::hex2bytes("24dd378f51adc67a50e339e8031fe9bd4aafab36")
						.unwrap()
						.as_slice(),
					b"",
				),
				(
					array_bytes::hex2bytes("293f982d000532a7861ab122bdc4bbfd26bf9030")
						.unwrap()
						.as_slice(),
					b"",
				),
				(
					array_bytes::hex2bytes("2cf5732f017b0cf1b1f13a1478e10239716bf6b5")
						.unwrap()
						.as_slice(),
					b"",
				),
				(
					array_bytes::hex2bytes("31c640b92c21a1f1465c91070b4b3b4d6854195f")
						.unwrap()
						.as_slice(),
					b"",
				),
				(
					array_bytes::hex2bytes("37f998764813b136ddf5a754f34063fd03065e36")
						.unwrap()
						.as_slice(),
					b"",
				),
				(
					array_bytes::hex2bytes("37fa399a749c121f8a15ce77e3d9f9bec8020d7a")
						.unwrap()
						.as_slice(),
					b"",
				),
				(
					array_bytes::hex2bytes("4f36659fa632310b6ec438dea4085b522a2dd077")
						.unwrap()
						.as_slice(),
					b"",
				),
				(
					array_bytes::hex2bytes("62c01474f089b07dae603491675dc5b5748f7049")
						.unwrap()
						.as_slice(),
					b"",
				),
				(
					array_bytes::hex2bytes("729af7294be595a0efd7d891c9e51f89c07950c7")
						.unwrap()
						.as_slice(),
					b"",
				),
				(
					array_bytes::hex2bytes("83e3e5a16d3b696a0314b30b2534804dd5e11197")
						.unwrap()
						.as_slice(),
					b"",
				),
				(
					array_bytes::hex2bytes("8703df2417e0d7c59d063caa9583cb10a4d20532")
						.unwrap()
						.as_slice(),
					b"",
				),
				(
					array_bytes::hex2bytes("8dffcd74e5b5923512916c6a64b502689cfa65e1")
						.unwrap()
						.as_slice(),
					b"",
				),
				(
					array_bytes::hex2bytes("95a4d7cccb5204733874fa87285a176fe1e9e240")
						.unwrap()
						.as_slice(),
					b"",
				),
				(
					array_bytes::hex2bytes("99b2fcba8120bedd048fe79f5262a6690ed38c39")
						.unwrap()
						.as_slice(),
					b"",
				),
				(
					array_bytes::hex2bytes("a4202b8b8afd5354e3e40a219bdc17f6001bf2cf")
						.unwrap()
						.as_slice(),
					b"",
				),
				(
					array_bytes::hex2bytes("a94f5374fce5edbc8e2a8697c15331677e6ebf0b")
						.unwrap()
						.as_slice(),
					b"",
				),
				(
					array_bytes::hex2bytes("a9647f4a0a14042d91dc33c0328030a7157c93ae")
						.unwrap()
						.as_slice(),
					b"",
				),
				(
					array_bytes::hex2bytes("aa6cffe5185732689c18f37a7f86170cb7304c2a")
						.unwrap()
						.as_slice(),
					b"",
				),
				(
					array_bytes::hex2bytes("aae4a2e3c51c04606dcb3723456e58f3ed214f45")
						.unwrap()
						.as_slice(),
					b"",
				),
				(
					array_bytes::hex2bytes("c37a43e940dfb5baf581a0b82b351d48305fc885")
						.unwrap()
						.as_slice(),
					b"",
				),
				(
					array_bytes::hex2bytes("d2571607e241ecf590ed94b12d87c94babe36db6")
						.unwrap()
						.as_slice(),
					b"",
				),
				(
					array_bytes::hex2bytes("f735071cbee190d76b704ce68384fc21e389fbe7")
						.unwrap()
						.as_slice(),
					b"",
				),
			],
			"0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
		);
		assert_root(
			vec![
				(
					array_bytes::hex2bytes(
						"0000000000000000000000000000000000000000000000000000000000000045",
					)
					.unwrap()
					.as_slice(),
					array_bytes::hex2bytes("22b224a1420a802ab51d326e29fa98e34c4f24ea")
						.unwrap()
						.as_slice(),
				),
				(
					array_bytes::hex2bytes(
						"0000000000000000000000000000000000000000000000000000000000000046",
					)
					.unwrap()
					.as_slice(),
					array_bytes::hex2bytes(
						"67706c2076330000000000000000000000000000000000000000000000000000",
					)
					.unwrap()
					.as_slice(),
				),
				(
					array_bytes::hex2bytes(
						"0000000000000000000000000000000000000000000000000000001234567890",
					)
					.unwrap()
					.as_slice(),
					array_bytes::hex2bytes("697c7b8c961b56f675d570498424ac8de1a918f6")
						.unwrap()
						.as_slice(),
				),
				(
					array_bytes::hex2bytes(
						"000000000000000000000000697c7b8c961b56f675d570498424ac8de1a918f6",
					)
					.unwrap()
					.as_slice(),
					array_bytes::hex2bytes("1234567890").unwrap().as_slice(),
				),
				(
					array_bytes::hex2bytes(
						"0000000000000000000000007ef9e639e2733cb34e4dfc576d4b23f72db776b2",
					)
					.unwrap()
					.as_slice(),
					array_bytes::hex2bytes(
						"4655474156000000000000000000000000000000000000000000000000000000",
					)
					.unwrap()
					.as_slice(),
				),
				(
					array_bytes::hex2bytes(
						"000000000000000000000000ec4f34c97e43fbb2816cfd95e388353c7181dab1",
					)
					.unwrap()
					.as_slice(),
					array_bytes::hex2bytes(
						"4e616d6552656700000000000000000000000000000000000000000000000000",
					)
					.unwrap()
					.as_slice(),
				),
				(
					array_bytes::hex2bytes(
						"4655474156000000000000000000000000000000000000000000000000000000",
					)
					.unwrap()
					.as_slice(),
					array_bytes::hex2bytes("7ef9e639e2733cb34e4dfc576d4b23f72db776b2")
						.unwrap()
						.as_slice(),
				),
				(
					array_bytes::hex2bytes(
						"4e616d6552656700000000000000000000000000000000000000000000000000",
					)
					.unwrap()
					.as_slice(),
					array_bytes::hex2bytes("ec4f34c97e43fbb2816cfd95e388353c7181dab1")
						.unwrap()
						.as_slice(),
				),
				(
					array_bytes::hex2bytes(
						"0000000000000000000000000000000000000000000000000000001234567890",
					)
					.unwrap()
					.as_slice(),
					array_bytes::hex2bytes("").unwrap().as_slice(),
				),
				(
					array_bytes::hex2bytes(
						"000000000000000000000000697c7b8c961b56f675d570498424ac8de1a918f6",
					)
					.unwrap()
					.as_slice(),
					array_bytes::hex2bytes(
						"6f6f6f6820736f2067726561742c207265616c6c6c793f000000000000000000",
					)
					.unwrap()
					.as_slice(),
				),
				(
					array_bytes::hex2bytes(
						"6f6f6f6820736f2067726561742c207265616c6c6c793f000000000000000000",
					)
					.unwrap()
					.as_slice(),
					array_bytes::hex2bytes("697c7b8c961b56f675d570498424ac8de1a918f6")
						.unwrap()
						.as_slice(),
				),
			],
			"0x9f6221ebb8efe7cff60a716ecb886e67dd042014be444669f0159d8e68b42100",
		);
		assert_root(
			vec![
				(b"key1aa", b"0123456789012345678901234567890123456789xxx"),
				(
					b"key1",
					b"0123456789012345678901234567890123456789Very_Long",
				),
				(b"key2bb", b"aval3"),
				(b"key2", b"short"),
				(b"key3cc", b"aval3"),
				(b"key3", b"1234567890123456789012345678901"),
			],
			"0xcb65032e2f76c48b82b5c24b3db8f670ce73982869d38cd39a624f23d62a9e89",
		);
		assert_root(
			vec![(b"abc", b"123"), (b"abcd", b"abcd"), (b"abc", b"abc")],
			"0x7a320748f780ad9ad5b0837302075ce0eeba6c26e3d8562c67ccc0f1b273298a",
		);
	}

	// proof test ref:
	// - https://github.com/ethereum/go-ethereum/blob/master/trie/proof_test.go
	// - https://github.com/ethereum/py-trie/blob/master/tests/test_proof.py
	#[test]
	fn test_proof_basic() {
		let memdb = Rc::new(MemoryDB::new());
		let mut trie = MerklePatriciaTrie::new(Rc::clone(&memdb));
		trie.insert(b"doe".to_vec(), b"reindeer".to_vec()).unwrap();
		trie.insert(b"dog".to_vec(), b"puppy".to_vec()).unwrap();
		trie.insert(b"dogglesworth".to_vec(), b"cat".to_vec())
			.unwrap();
		let root = trie.root().unwrap();
		let r = format!("0x{}", hex::encode(trie.root().unwrap()));
		assert_eq!(
			r.as_str(),
			"0x8aad789dff2f538bca5d8ea56e8abe10f4c7ba3a5dea95fea4cd6e7c3a1168d3"
		);

		// proof of key exists
		let proof = trie.get_proof(b"doe").unwrap();
		let expected = vec![
			"e5831646f6a0db6ae1fda66890f6693f36560d36b4dca68b4d838f17016b151efe1d4c95c453",
			"f83b8080808080ca20887265696e6465657280a037efd11993cb04a54048c25320e9f29c50a432d28afdf01598b2978ce1ca3068808080808080808080",
		];
		assert_eq!(
			proof
				.clone()
				.nodes
				.into_iter()
				.map(hex::encode)
				.collect::<Vec<_>>(),
			expected
		);
		let value = MerklePatriciaTrie::verify_proof(root.clone(), b"doe", proof).unwrap();
		assert_eq!(value, Some(b"reindeer".to_vec()));

		// proof of key not exist
		let proof = trie.get_proof(b"dogg").unwrap();
		let expected = vec![
			"e5831646f6a0db6ae1fda66890f6693f36560d36b4dca68b4d838f17016b151efe1d4c95c453",
			"f83b8080808080ca20887265696e6465657280a037efd11993cb04a54048c25320e9f29c50a432d28afdf01598b2978ce1ca3068808080808080808080",
			"e4808080808080ce89376c6573776f72746883636174808080808080808080857075707079",
		];
		assert_eq!(
			proof
				.clone()
				.nodes
				.into_iter()
				.map(hex::encode)
				.collect::<Vec<_>>(),
			expected
		);
		let value = MerklePatriciaTrie::verify_proof(root.clone(), b"dogg", proof).unwrap();
		assert_eq!(value, None);

		// empty proof
		let proof = vec![];
		let value = MerklePatriciaTrie::verify_proof(root.clone(), b"doe", proof.into());
		assert_eq!(value.is_err(), true);

		// bad proof
		let proof = vec![b"aaa".to_vec(), b"ccc".to_vec()];
		let value = MerklePatriciaTrie::verify_proof(root.clone(), b"doe", proof.into());
		assert_eq!(value.is_err(), true);
	}

	#[test]
	fn test_proof_random() {
		let memdb = Rc::new(MemoryDB::new());
		let mut trie = MerklePatriciaTrie::new(Rc::clone(&memdb));
		let mut rng = rand::thread_rng();
		let mut keys = vec![];
		for _ in 0..100 {
			let random_bytes: Vec<u8> = (0..rng.gen_range(2..30))
				.map(|_| rand::random::<u8>())
				.collect();
			trie.insert(random_bytes.to_vec(), random_bytes.clone())
				.unwrap();
			keys.push(random_bytes.clone());
		}
		for k in keys.clone().into_iter() {
			trie.insert(k.clone(), k.clone()).unwrap();
		}
		let root = trie.root().unwrap();
		for k in keys.into_iter() {
			let proof = trie.get_proof(&k).unwrap();
			let value = MerklePatriciaTrie::verify_proof(root.clone(), &k, proof)
				.unwrap()
				.unwrap();
			assert_eq!(value, k);
		}
	}

	#[test]
	fn test_proof_empty_trie() {
		let memdb = Rc::new(MemoryDB::new());
		let mut trie = MerklePatriciaTrie::new(Rc::clone(&memdb));
		trie.root().unwrap();
		let proof = trie.get_proof(b"not-exist").unwrap();
		assert_eq!(proof.len(), 0);
	}

	#[test]
	fn test_proof_one_element() {
		let memdb = Rc::new(MemoryDB::new());
		let mut trie = MerklePatriciaTrie::new(Rc::clone(&memdb));
		trie.insert(b"k".to_vec(), b"v".to_vec()).unwrap();
		let root = trie.root().unwrap();
		let proof = trie.get_proof(b"k").unwrap();
		assert_eq!(proof.len(), 1);
		let value = MerklePatriciaTrie::verify_proof(root.clone(), b"k", proof.clone()).unwrap();
		assert_eq!(value, Some(b"v".to_vec()));

		// remove key does not affect the verify process
		trie.remove(b"k").unwrap();
		let _root = trie.root().unwrap();
		let value = MerklePatriciaTrie::verify_proof(root.clone(), b"k", proof.clone()).unwrap();
		assert_eq!(value, Some(b"v".to_vec()));
	}

	#[test]
	fn test_ethereum_receipts_proof() {
		let rlp_proof: Vec<u8> = array_bytes::hex2bytes("f9016ef9016bb853f851a009b67a67265063da0dd6a7abad695edb2c439f6b458f2a2ee48a21442fef8a2680808080808080a0a7d4f8b974d21b7244014729b07e9c9f19fdc445da2ceddc089d90cead74be618080808080808080b90113f9011031b9010cf9010901835cdb6eb9010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000c0").unwrap();
		let expected: Vec<u8> = array_bytes::hex2bytes("f9010901835cdb6eb9010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000c0").unwrap();
		let root = array_bytes::hex2bytes(
			"7fa081e3e33e53c4d09ae691af3853bb73a7e02c856104fe843172abab85df7b",
		)
		.unwrap();

		let proof: Proof = rlp::decode(&rlp_proof).unwrap();
		let key = rlp::encode(&1usize);
		let value = MerklePatriciaTrie::verify_proof(root.clone(), &key, proof.clone()).unwrap();
		assert!(value.is_some());
		assert_eq!(value.unwrap(), expected);
	}

	#[test]
	fn test_ethereum_receipts_proof2() {
		let rlp_proof: Vec<u8> = array_bytes::hex2bytes("f908b8f908b5b90134f90131a0e7206eab439417b2863e5b677527cb60269ba3d8af2d8c623872237f3d542e65a0b46a06500bb5935a94e15c5740b5c0c589f2392accd913e8f380a090d104176aa0f620e968acc21cedc8030f4bcdde8307a742b216a7ec632ee325314e173c2a4ca0732e3bda61e32fe2d0a8e17b754e382902f4dcc59ff363dd6a725f8957696ed5a0b8b265f067f78cbfa1f661342887599c8f2eae0fe2e03797ea7f9396eac3d733a06c50b5ad0042758db2693b34b4270c0b184cfcf4187a152c3c827f77fea1b957a05434b1d7fc89fb6f71aa90208b0be6bad7119d5101581570eb63a349a96cc937a04c3c7bd070e7e107fd6b50c22cafc53733af15bdfdc6cd2ef1a7e55ea121fa58a0157399b3777e29da6fdb395deee0949333d35f04bd2d37a2ce97a502bdd7410d8080808080808080b90214f90211a05998ffd905334dff9092dd85e88fa341f23b53f75114134f1580e5fb6f06157fa086c534484bfefd8ce70dd756dfd6d91979ab96bcce228636936966213a733997a01c8946eb82acb9f73ec6cdf47f2e04890b27d2304b48f31460d8aaef198f14a3a05de0a90155b81ca909473926cb88157dd4d55a38a2735e61d2cc91beb9b5d36ba02e314e7f821941f0ef4bd141e81257926d795aaa123a88269f4e4d9767b5e606a07b521e30207c404624189e217e53aab7e9d9c763e99e8591c0943acbc1d20635a09a9f625821788c2a040049099d23eb54ac525f380c4803140265c9cd516d75afa0a515d3a321b2e53fbfbf668b046a032ee8a669a55c0b5abcf3b16f3008325020a0e39ced73e44b0d76efc326c0bcdf0e051a85d3284c6ee99f899e34f9dfb6561aa0aac2571c5fac310080edd1f02b467773d1d3fd9b8ca1a76593bec16063e880dfa02e03d8dbc02629df2f36f91822915c6d78f463d1f7a913c79c95683a414a63f5a0ec9e7eeaa8c2884d6c8d63d4ac533e402473c6125f6ad55dac36df373475d248a0132b1916527d24b50a512fa8fc8d63f4427187188d79d744aa69b8f618631881a0d353f9d4f8e34aed416a6928a61635d61c832246fdb5eef03cef34c8a71ce846a0d2cedf98a55e6bd832d28ad75ee9d6f05297467bb27579b887d6922646b8efeea069bada2d2102507431aa74b513b30971927d1df678ada8c9f0c65b7d17c34bf180b90564f9056120b9055df9055a01835227f9b9010000000000000000000000000000000000400000000000001000000000000000000000000000000002000000000000000000000000000000400000000000002000000000000000020000000008000000400000000000000000000000000000008000000000020000000100200000000800080000101000000000000010000000400000000000000000000000000000000000000000004000000000000000200400200000000000000000000000000000000000000000000000080000000000000080000002000000000000100000008000000010000000000000000000010028000000000000000000000000000000000000000200000000000000000000000000f9044ff89b949469d013805bffb7d3debe5e7839237e535ec483f863a0ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3efa00000000000000000000000001649b7518ed8d64f07771ee16def11174afe8b12a0000000000000000000000000ea7938985898af7fd945b03b7bc2e405e744e913a000000000000000000000000000000000000000000000054b40b1f852bda00000f89b949469d013805bffb7d3debe5e7839237e535ec483f863a0ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3efa00000000000000000000000001649b7518ed8d64f07771ee16def11174afe8b12a0000000000000000000000000781bcec6dd947dc7214e54d52e599afc78980268a00000000000000000000000000000000000000000000000008ac7230489e80000f87a949469d013805bffb7d3debe5e7839237e535ec483f842a0cc16f5dbb4873280815c1ee09dbd06736cffcc184412cf7a71a0fdb75d397ca5a0000000000000000000000000ea7938985898af7fd945b03b7bc2e405e744e913a000000000000000000000000000000000000000000000054b40b1f852bda00000f89b949469d013805bffb7d3debe5e7839237e535ec483f863a0ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3efa0000000000000000000000000ea7938985898af7fd945b03b7bc2e405e744e913a00000000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000054b40b1f852bda00000f8fc94ea7938985898af7fd945b03b7bc2e405e744e913f863a0c9dcda609937876978d7e0aa29857cb187aea06ad9e843fd23fd32108da73f10a00000000000000000000000009469d013805bffb7d3debe5e7839237e535ec483a00000000000000000000000001649b7518ed8d64f07771ee16def11174afe8b12b88000000000000000000000000000000000000000000000054b40b1f852bda0000000000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000020cec1dfd0180c824e602a1982869dd25b28d6590836c8415340954460be73fe77f8fc949469d013805bffb7d3debe5e7839237e535ec483f863a09bfafdc2ae8835972d7b64ef3f8f307165ac22ceffde4a742c52da5487f45fd1a00000000000000000000000001649b7518ed8d64f07771ee16def11174afe8b12a0000000000000000000000000ea7938985898af7fd945b03b7bc2e405e744e913b88000000000000000000000000000000000000000000000054b40b1f852bda0000000000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000020cec1dfd0180c824e602a1982869dd25b28d6590836c8415340954460be73fe77").unwrap();
		// let expected: Vec<u8> = array_bytes::hex2bytes("f9010901835cdb6eb9010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000c0").unwrap();
		let root = array_bytes::hex2bytes(
			"9e5110c92c2fbab59db4661696e0fc1f9e22259c68d2e95accd6286abd3b0c0c",
		)
			.unwrap();

		let proof: Proof = rlp::decode(&rlp_proof).unwrap();
		let key = rlp::encode(&81usize);
		let value = MerklePatriciaTrie::verify_proof(root.clone(), &key, proof.clone()).unwrap();
		assert!(value.is_some());
		// assert_eq!(value.unwrap(), expected);
	}

	#[test]
	fn test_ethereum_receipts_build_proof() {
		// transaction hash 0xb04fcb9822eb21b5ffdbf89df076de58469af66d23c86abe30266e5d3c5e0db2   in ropsten
		// build trie
		let data = vec![
			array_bytes::hex2bytes("f90184018261beb9010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000040000000000000000000000800000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000800000020000000000000000000000000000000000f87bf87994095c5cbf4937d0a21f6f395194e95b6ebe8616b9e1a06ef95f06320e7a25a04a175ca677b7052bdd97131872c2192525a629f51be770b8400000000000000000000000002e0a521fe69c14d99c8d236d8c3cd5353cc44e720000000000000000000000000000000000000000000000000000000000000000").unwrap(),
			array_bytes::hex2bytes("f9010901835cdb6eb9010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000c0").unwrap(),
		];
		let hash = "0x7fa081e3e33e53c4d09ae691af3853bb73a7e02c856104fe843172abab85df7b";

		let memdb = Rc::new(MemoryDB::new());
		let mut trie = MerklePatriciaTrie::new(Rc::clone(&memdb));
		for (k, v) in data
			.clone()
			.into_iter()
			.enumerate()
			.map(|(i, v)| (rlp::encode(&i), v))
		{
			trie.insert(k.to_vec(), v.to_vec()).unwrap();
		}
		let r = trie.root().unwrap();
		let rs = array_bytes::bytes2hex("0x", r.clone());

		assert_eq!(rs.as_str(), hash);

		// check proof
		let key = rlp::encode(&1usize);
		let proof = trie.get_proof(&key).unwrap();
		let value = MerklePatriciaTrie::verify_proof(r.clone(), &key, proof.clone()).unwrap();

		assert_eq!(value.unwrap(), data[1]);
	}
}
