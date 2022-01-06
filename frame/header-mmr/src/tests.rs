// This file is part of Darwinia.
//
// Copyright (C) 2018-2022 Darwinia Network
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
use codec::Encode;
// --- paritytech ---
use sp_runtime::testing::Digest;
// --- darwinia-network ---
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
