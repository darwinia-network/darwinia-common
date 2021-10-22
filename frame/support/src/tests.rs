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

// --- paritytech ---
use sp_core::{H160, U256};
// --- darwinia-network ---
use crate::{
	evm::IntoH160,
	s2s::{RING_NAME, RING_SYMBOL},
	*,
};
use array_bytes::{hex2array, hex2bytes};
use std::str::FromStr;

#[test]
fn const_pow_9_should_work() {
	assert_eq!(
		U256::from(10).checked_pow(U256::from(9)).unwrap(),
		evm::POW_9.into()
	)
}

#[test]
fn test_into_dvm_account() {
	assert_eq!(
		H160::from_str("726f6f7400000000000000000000000000000000").unwrap(),
		(&b"root"[..]).into_h160()
	);
	assert_eq!(
		(&b"longbytes..longbytes..longbytes..longbytes"[..]).into_h160(),
		(&b"longbytes..longbytes"[..]).into_h160()
	);
}

#[test]
fn test_ring_symbol_encode() {
	// Get this info: https://etherscan.io/address/0x9469d013805bffb7d3debe5e7839237e535ec483#readContract
	let target_symbol = "0x52494e4700000000000000000000000000000000000000000000000000000000";
	assert_eq!(to_bytes32(RING_SYMBOL), hex2array(target_symbol).unwrap());
}

#[test]
fn test_ring_name_encode() {
	// Get this info: https://etherscan.io/address/0x9469d013805bffb7d3debe5e7839237e535ec483#readContract
	let target_name = "0x44617277696e6961204e6574776f726b204e617469766520546f6b656e000000";
	assert_eq!(to_bytes32(RING_NAME), hex2array(target_name).unwrap());
}

#[test]
fn test_ring_name_decode() {
	let name = "0x44617277696e6961204e6574776f726b204e617469766520546f6b656e000000";
	let raw = hex2bytes(name).unwrap();
	assert_eq!(RING_NAME, &raw[..29]);
}
