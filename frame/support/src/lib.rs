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

#![cfg_attr(not(feature = "std"), no_std)]

pub mod macros;
pub mod structs;
pub mod testing;
pub mod traits;

pub mod balance {
	pub use crate::structs::{
		BalanceLock, FrozenBalance, LockFor, LockReasons, StakingLock, Unbonding,
	};
	pub use crate::traits::{BalanceInfo, DustCollector, LockableCurrency, OnUnbalancedKton};
}

pub mod evm {
	// --- darwinia ---
	use ethereum_primitives::H160;

	pub const POW_9: u32 = 1_000_000_000;
	pub const INTERNAL_CALLER: H160 = H160::zero();
}

pub mod s2s {
	use ethabi::{encode, Token};

	pub const RING_NAME: &[u8] = b"Darwinia Network Native Token";
	pub const RING_SYMBOL: &[u8] = b"RING";
	pub const RING_DECIMAL: u8 = 9;

	// S2S backing pallet
	pub const BACK_ERC20_RING: &'static str = "0x0000000000000000000000000000000000002048";

	pub fn to_bytes32(raw: &[u8]) -> [u8; 32] {
		let mut result = [0u8; 32];
		let encoded = encode(&[Token::FixedBytes(raw.to_vec())]);
		result.copy_from_slice(&encoded);
		result
	}
}

#[cfg(test)]
mod test {
	use crate::s2s::{to_bytes32, RING_NAME, RING_SYMBOL};
	use array_bytes::hex2array_unchecked;

	#[test]
	fn test_ring_symbol_encode() {
		// Get this info: https://etherscan.io/address/0x9469d013805bffb7d3debe5e7839237e535ec483#readContract
		let target_symbol = "0x52494e4700000000000000000000000000000000000000000000000000000000";
		assert_eq!(
			to_bytes32(RING_SYMBOL),
			hex2array_unchecked!(target_symbol, 32)
		);
	}

	#[test]
	fn test_ring_name_encode() {
		// Get this info: https://etherscan.io/address/0x9469d013805bffb7d3debe5e7839237e535ec483#readContract
		let target_name = "0x44617277696e6961204e6574776f726b204e617469766520546f6b656e000000";
		assert_eq!(to_bytes32(RING_NAME), hex2array_unchecked!(target_name, 32));
	}

	#[test]
	fn test_ring_name_decode() {
		let name = "44617277696e6961204e6574776f726b204e617469766520546f6b656e000000";
		let raw = hex::decode(name).unwrap();
		assert_eq!(RING_NAME, &raw[..29]);
	}
}
