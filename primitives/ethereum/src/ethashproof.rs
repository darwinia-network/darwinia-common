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

// --- alloc ---
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
// --- crates.io ---
use sp_debug_derive::RuntimeDebug;
// --- github.com ---
#[cfg(any(feature = "full-codec", test))]
use codec::{Decode, Encode};
#[cfg(any(feature = "full-codec", test))]
use scale_info::TypeInfo;
#[cfg(any(feature = "full-serde", test))]
use serde::Deserialize;
// --- darwinia-network ---
use crate::{H128, H512};

#[cfg_attr(any(feature = "full-codec", test), derive(Encode, Decode, TypeInfo))]
#[cfg_attr(any(feature = "full-serde", test), derive(Deserialize))]
#[derive(Clone, Default, PartialEq, Eq, RuntimeDebug)]
pub struct EthashProof {
	pub dag_nodes: [H512; 2],
	pub proof: Vec<H128>,
}
impl EthashProof {
	pub fn apply_merkle_proof(&self, index: u64) -> H128 {
		fn hash_h128(l: H128, r: H128) -> H128 {
			let mut data = [0u8; 64];
			data[16..32].copy_from_slice(&(l.0));
			data[48..64].copy_from_slice(&(r.0));

			// `H256` is 32 length, truncate is safe; qed
			array_bytes::dyn_into!(subhasher::sha2_256(&data)[16..], 16)
		}

		let mut data = [0u8; 128];
		data[..64].copy_from_slice(&(self.dag_nodes[0].0));
		data[64..].copy_from_slice(&(self.dag_nodes[1].0));

		// `H256` is 32 length, truncate is safe; qed
		let mut leaf = array_bytes::dyn_into!(subhasher::sha2_256(&data)[16..], 16);
		for i in 0..self.proof.len() {
			if (index >> i as u64) % 2 == 0 {
				leaf = hash_h128(leaf, self.proof[i]);
			} else {
				leaf = hash_h128(self.proof[i], leaf);
			}
		}

		leaf
	}
}

#[test]
fn scale_should_work() {
	let ethash_proof = EthashProof::default();
	let encoded_ethash_proof = ethash_proof.encode();

	assert_eq!(
		ethash_proof,
		EthashProof::decode(&mut &*encoded_ethash_proof).unwrap()
	);
}

#[test]
fn serde_should_work() {
	let ethash_proof = serde_json::from_str::<EthashProof>(r#"{
		"dag_nodes": [
			"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
			"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"
		],
		"proof": [
			"0x00000000000000000000000000000000",
			"0x00000000000000000000000000000000",
			"0x00000000000000000000000000000000",
			"0x00000000000000000000000000000000",
			"0x00000000000000000000000000000000",
			"0x00000000000000000000000000000000",
			"0x00000000000000000000000000000000",
			"0x00000000000000000000000000000000",
			"0x00000000000000000000000000000000",
			"0x00000000000000000000000000000000",
			"0x00000000000000000000000000000000",
			"0x00000000000000000000000000000000",
			"0x00000000000000000000000000000000",
			"0x00000000000000000000000000000000",
			"0x00000000000000000000000000000000",
			"0x00000000000000000000000000000000",
			"0x00000000000000000000000000000000",
			"0x00000000000000000000000000000000",
			"0x00000000000000000000000000000000",
			"0x00000000000000000000000000000000",
			"0x00000000000000000000000000000000",
			"0x00000000000000000000000000000000",
			"0x00000000000000000000000000000000",
			"0x00000000000000000000000000000000",
			"0x00000000000000000000000000000000"
			]
	}"#).unwrap();

	assert_eq!(
		ethash_proof,
		EthashProof {
			proof: vec![Default::default(); 25],
			..Default::default()
		}
	);
}
