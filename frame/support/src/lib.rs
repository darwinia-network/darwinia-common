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

#[cfg(all(feature = "std", test))]
pub mod tests;

pub mod evm;
pub mod macros;
pub mod structs;
pub mod testing;
pub mod traits;

pub mod balance {
	pub use crate::{
		structs::{BalanceLock, FrozenBalance, LockFor, LockReasons, StakingLock, Unbonding},
		traits::{BalanceInfo, DustCollector, LockableCurrency},
	};
}
use ethabi::{encode, Token};
use sp_std::{vec, vec::Vec};

// TODO: Should we move this to `s2s-primitives`?
pub mod s2s {
	pub use crate::TokenMessageId;
	// --- crates.io ---
	use codec::Encode;
	// --- darwinia-network ---
	use ethereum_primitives::{H160, H256};
	// --- paritytech ---
	use bp_runtime::{derive_account_id, ChainId, SourceAccount};
	use frame_support::{ensure, pallet_prelude::Weight, weights::PostDispatchInfo};
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

		fn latest_token_message_id() -> TokenMessageId;
	}

	pub trait MessageConfirmer {
		fn on_messages_confirmed(message_id: TokenMessageId, result: bool) -> Weight;
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

	pub fn nonce_to_message_id(lane_id: &[u8], nonce: u64) -> TokenMessageId {
		let mut message_id: TokenMessageId = Default::default();
		message_id[4..8].copy_from_slice(&lane_id[..4]);
		message_id[8..].copy_from_slice(&nonce.to_be_bytes());
		message_id
	}
}

pub mod mapping_token {
	use super::*;
	pub fn mapping_token_name(original_name: Vec<u8>, backing_chain_name: Vec<u8>) -> Vec<u8> {
		let mut mapping_name = original_name.clone();
		mapping_name.push(b'[');
		mapping_name.extend(backing_chain_name);
		mapping_name.push(b'>');
		mapping_name
	}

	pub fn mapping_token_symbol(original_symbol: Vec<u8>) -> Vec<u8> {
		let mut mapping_symbol = vec![b'x'];
		mapping_symbol.extend(original_symbol);
		mapping_symbol
	}
}

pub fn to_bytes32(raw: &[u8]) -> [u8; 32] {
	let mut result = [0u8; 32];
	let encoded = encode(&[Token::FixedBytes(raw.to_vec())]);
	result.copy_from_slice(&encoded);
	result
}

/// 128 bit or 16 bytes to identify an unique s2s message
/// [0..4]  bytes ---- reserved
/// [4..8]  bytes ---- laneID
/// [8..16] bytes ---- message nonce
pub type TokenMessageId = [u8; 16];

pub type ChainName = Vec<u8>;

#[cfg(test)]
mod test {
	use crate::{
		s2s::{RING_NAME, RING_SYMBOL},
		to_bytes32,
	};
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
