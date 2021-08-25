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

pub mod evm;
pub mod macros;
pub mod structs;
pub mod testing;
pub mod traits;

pub mod balance {
	pub use crate::structs::{
		BalanceLock, FrozenBalance, LockFor, LockReasons, StakingLock, Unbonding,
	};
	pub use crate::traits::{BalanceInfo, DustCollector, LockableCurrency};
}

// TODO: Should we move this to `s2s-primitives`?
pub mod s2s {
	// --- crates.io ---
	use codec::Encode;
	use ethabi::{encode, Token};
	// --- darwinia-network ---
	use ethereum_primitives::{H160, H256};
	// --- paritytech ---
	use bp_runtime::{derive_account_id, ChainId, SourceAccount};
	use frame_support::{ensure, weights::PostDispatchInfo};
	use sp_runtime::{
		traits::{BadOrigin, Convert},
		DispatchError, DispatchErrorWithPostInfo,
	};
	use sp_std::cmp::PartialEq;

	pub const RING_NAME: &[u8] = b"Darwinia Network Native Token";
	pub const RING_SYMBOL: &[u8] = b"RING";
	pub const RING_DECIMAL: u8 = 9;

	pub trait ToEthAddress<A> {
		fn into_ethereum_id(address: &A) -> H160;
	}

	// RelayMessageCaller send message to pallet-messages
	pub trait RelayMessageCaller<P, F> {
		fn send_message(
			payload: P,
			fee: F,
		) -> Result<PostDispatchInfo, DispatchErrorWithPostInfo<PostDispatchInfo>>;
	}

	pub fn ensure_source_root<AccountId, Converter>(
		chain_id: ChainId,
		account: &AccountId,
	) -> Result<(), DispatchError>
	where
		AccountId: PartialEq + Encode,
		Converter: Convert<H256, AccountId>,
	{
		let hex_id = derive_account_id::<AccountId>(chain_id, SourceAccount::Root);
		let target_id = Converter::convert(hex_id);
		ensure!(&target_id == account, BadOrigin);
		Ok(())
	}

	pub fn to_bytes32(raw: &[u8]) -> [u8; 32] {
		let mut result = [0u8; 32];
		let encoded = encode(&[Token::FixedBytes(raw.to_vec())]);
		result.copy_from_slice(&encoded);
		result
	}
}

pub type PalletDigest = [u8; 4];

#[cfg(test)]
mod test {
	use crate::s2s::{to_bytes32, RING_NAME, RING_SYMBOL};
	use array_bytes::{hex2array, hex2bytes};

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
}
