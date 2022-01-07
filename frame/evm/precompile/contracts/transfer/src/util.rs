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

pub fn e2s_address(eth_address: ethereum_types::H160) -> sp_core::H160 {
	let eth_address_bytes = eth_address.to_fixed_bytes();
	sp_core::H160::from_slice(&eth_address_bytes)
}

pub fn s2e_address(sp_address: sp_core::H160) -> ethereum_types::H160 {
	let sp_address_bytes = sp_address.to_fixed_bytes();
	ethereum_types::H160::from_slice(&sp_address_bytes)
}

pub fn e2s_u256(eth_value: ethereum_types::U256) -> sp_core::U256 {
	let mut value_bytes = [0u8; 32];
	eth_value.to_big_endian(&mut value_bytes);
	sp_core::U256::from_big_endian(&value_bytes)
}

pub fn s2e_u256(sp_value: sp_core::U256) -> ethereum_types::U256 {
	let mut value_bytes = [0u8; 32];
	sp_value.to_big_endian(&mut value_bytes);
	ethereum_types::U256::from_big_endian(&value_bytes)
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::str::FromStr;

	#[test]
	fn test_address() {
		let eth_address =
			ethereum_types::H160::from_str("Aa01a1bEF0557fa9625581a293F3AA7770192632").unwrap();
		let sp_address =
			sp_core::H160::from_str("Aa01a1bEF0557fa9625581a293F3AA7770192632").unwrap();
		let output = e2s_address(eth_address);
		assert_eq!(output.0, sp_address.0);

		let output = s2e_address(sp_address);
		assert_eq!(output.0, eth_address.0);
	}

	#[test]
	fn test_value() {
		let eth_value = ethereum_types::U256::from(200);
		let sp_value = sp_core::U256::from(200);
		let output = e2s_u256(eth_value);
		assert_eq!(output, sp_value);

		let output = s2e_u256(sp_value);
		assert_eq!(output, eth_value);
	}
}
