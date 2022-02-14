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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

pub type NetworkId = [u8; 32];

pub trait AsciiId {
	fn ascii_id() -> NetworkId;
}

macro_rules! impl_network_ids {
	($($network:ident: $ascii_id:expr),+) => {
		$(
			pub struct $network;
			impl AsciiId for $network {
				fn ascii_id() -> NetworkId {
					$ascii_id
				}
			}
		)+
	};
}
impl_network_ids![
	Darwinia: [
		68, 65, 82, 87, 73, 78, 73, 65, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
		0, 0, 0, 0, 0, 0,
	],
	Crab: [
		67, 82, 65, 66, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
		0, 0,
	],
	Pangoro: [
		80, 65, 78, 71, 79, 82, 79, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
		0, 0, 0,
	],
	Pangolin:  [
		80, 65, 78, 71, 79, 76, 73, 78, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
		0, 0, 0,
	]
];

pub fn convert(name: &[u8]) -> NetworkId {
	let mut ary = [0; 32];

	for i in 0..name.len().min(32) {
		ary[i] = name[i];
	}

	ary
}

#[test]
fn network_ascii_should_work() {
	for (network_id, network_id_hex) in [
		(
			Darwinia::ascii_id(),
			"0x44415257494e4941000000000000000000000000000000000000000000000000",
		),
		(
			Crab::ascii_id(),
			"0x4352414200000000000000000000000000000000000000000000000000000000",
		),
		(
			Pangoro::ascii_id(),
			"0x50414e474f524f00000000000000000000000000000000000000000000000000",
		),
		(
			Pangolin::ascii_id(),
			"0x50414e474f4c494e000000000000000000000000000000000000000000000000",
		),
	]
	.iter()
	{
		assert_eq!(&array_bytes::bytes2hex("0x", network_id), network_id_hex);
	}

	// dbg!(convert(b"DARWINIA"));
	// dbg!(array_bytes::bytes2hex("0x", Darwinia.ascii_id()));
	// dbg!(convert(b"CRAB"));
	// dbg!(array_bytes::bytes2hex("0x", Crab.ascii_id()));
	// dbg!(convert(b"PANGORO"));
	// dbg!(array_bytes::bytes2hex("0x", Pangoro.ascii_id()));
	// dbg!(convert(b"PANGOLIN"));
	// dbg!(array_bytes::bytes2hex("0x", Pangolin.ascii_id()));
}
