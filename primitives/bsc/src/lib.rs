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

#![cfg_attr(not(feature = "std"), no_std)]

pub use primitive_types::{H160 as Address, H256 as Hash};

// --- crates.io ---
use codec::{Decode, Encode};
use ethbloom::{Bloom, Input};
use hash_db::Hasher;
use primitive_types::U256;
use rlp::RlpStream;
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
// --- paritytech ---
use sp_io::hashing;
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

pub type Bytes = Vec<u8>;

/// Raw (RLP-encoded) ethereum transaction.
pub type RawTransaction = Vec<u8>;

/// Protocol constants
/// The KECCAK of the RLP encoding of empty data.
pub const KECCAK_NULL_RLP: Hash = Hash([
	0x56, 0xe8, 0x1f, 0x17, 0x1b, 0xcc, 0x55, 0xa6, 0xff, 0x83, 0x45, 0xe6, 0x92, 0xc0, 0xf8, 0x6e,
	0x5b, 0x48, 0xe0, 0x1b, 0x99, 0x6c, 0xad, 0xc0, 0x01, 0x62, 0x2f, 0xb5, 0xe3, 0x63, 0xb4, 0x21,
]);

/// The KECCAK of the RLP encoding of empty list.
pub const KECCAK_EMPTY_LIST_RLP: Hash = Hash([
	0x1d, 0xcc, 0x4d, 0xe8, 0xde, 0xc7, 0x5d, 0x7a, 0xab, 0x85, 0xb5, 0x67, 0xb6, 0xcc, 0xd4, 0x1a,
	0xd3, 0x12, 0x45, 0x1b, 0x94, 0x8a, 0x74, 0x13, 0xf0, 0xa1, 0x42, 0xfd, 0x40, 0xd4, 0x93, 0x47,
]);

/// Fixed number of extra-data prefix bytes reserved for signer vanity
pub const VANITY_LENGTH: usize = 32;
/// Fixed number of extra-data suffix bytes reserved for signer signature
pub const SIGNATURE_LENGTH: usize = 65;
/// Address length of signer
pub const ADDRESS_LENGTH: usize = 20;
/// Difficulty for INTURN block
pub const DIFF_INTURN: U256 = U256([2, 0, 0, 0]);
/// Difficulty for NOTURN block
pub const DIFF_NOTURN: U256 = U256([1, 0, 0, 0]);
/// Default noturn block wiggle factor defined in spec.
pub const SIGNING_DELAY_NOTURN_MS: u64 = 500;

/// Complete header id.
#[derive(Clone, Default, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct HeaderId {
	/// Header number.
	pub number: u64,
	/// Header hash.
	pub hash: Hash,
}

/// An BSC(Binance Smart Chain) header.
#[derive(Clone, Default, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
#[cfg_attr(
	feature = "std",
	derive(Serialize, Deserialize),
	serde(rename_all = "camelCase")
)]
pub struct BscHeader {
	/// Parent block hash.
	pub parent_hash: Hash,
	/// Block uncles hash.
	#[cfg_attr(feature = "std", serde(rename = "sha3Uncles"))]
	pub uncle_hash: Hash,
	/// validator address
	#[cfg_attr(feature = "std", serde(rename = "miner"))]
	pub coinbase: Address,
	/// State root.
	pub state_root: Hash,
	/// Transactions root.
	pub transactions_root: Hash,
	/// Block receipts root.
	pub receipts_root: Hash,
	/// Block bloom.
	#[cfg_attr(feature = "std", serde(rename = "logsBloom"))]
	pub log_bloom: Bloom,
	/// Block difficulty.
	pub difficulty: U256,
	/// Block number.
	#[cfg_attr(feature = "std", serde(deserialize_with = "array_bytes::de_hex2num"))]
	pub number: u64,
	/// Block gas limit.
	pub gas_limit: U256,
	/// Gas used for contracts execution.
	pub gas_used: U256,
	/// Block timestamp.
	#[cfg_attr(feature = "std", serde(deserialize_with = "array_bytes::de_hex2num"))]
	pub timestamp: u64,
	/// Block extra data.
	#[cfg_attr(feature = "std", serde(deserialize_with = "array_bytes::de_hex2bytes"))]
	pub extra_data: Bytes,
	/// MixDigest
	#[cfg_attr(feature = "std", serde(rename = "mixHash"))]
	pub mix_digest: Hash,
	/// Nonce(64 bit in ethereum)
	#[cfg_attr(feature = "std", serde(deserialize_with = "array_bytes::de_hex2bytes"))]
	pub nonce: Bytes,
}
impl BscHeader {
	/// Compute id of this header.
	pub fn compute_id(&self) -> HeaderId {
		HeaderId {
			number: self.number,
			hash: self.compute_hash(),
		}
	}

	/// Compute hash of this header (keccak of the RLP with seal).
	pub fn compute_hash(&self) -> Hash {
		hashing::keccak_256(&self.rlp()).into()
	}

	pub fn compute_hash_with_chain_id(&self, chain_id: u64) -> Hash {
		hashing::keccak_256(&self.rlp_chain_id(chain_id)).into()
	}

	/// Get id of this header' parent. Returns None if this is genesis header.
	pub fn parent_id(&self) -> Option<HeaderId> {
		self.number.checked_sub(1).map(|parent_number| HeaderId {
			number: parent_number,
			hash: self.parent_hash,
		})
	}

	/// Check if passed transactions are matching transactions root in this header.
	/// Returns Ok(computed-root) if check succeeds.
	/// Returns Err(computed-root) if check fails.
	pub fn check_transactions_root<'a>(
		&self,
		transactions: impl IntoIterator<Item = &'a RawTransaction>,
	) -> Result<Hash, Hash> {
		check_merkle_proof(self.transactions_root, transactions.into_iter())
	}

	/// Returns header RLP
	fn rlp(&self) -> Bytes {
		let mut s = RlpStream::new();

		s.begin_list(15);
		s.append(&self.parent_hash);
		s.append(&self.uncle_hash);
		s.append(&self.coinbase);
		s.append(&self.state_root);
		s.append(&self.transactions_root);
		s.append(&self.receipts_root);
		s.append(&self.log_bloom);
		s.append(&self.difficulty);
		s.append(&self.number);
		s.append(&self.gas_limit);
		s.append(&self.gas_used);
		s.append(&self.timestamp);
		s.append(&self.extra_data);
		s.append(&self.mix_digest);
		s.append(&self.nonce);

		s.out().to_vec()
	}

	fn rlp_chain_id(&self, chain_id: u64) -> Bytes {
		let mut s = RlpStream::new();

		s.begin_list(16);
		s.append(&chain_id);
		s.append(&self.parent_hash);
		s.append(&self.uncle_hash);
		s.append(&self.coinbase);
		s.append(&self.state_root);
		s.append(&self.transactions_root);
		s.append(&self.receipts_root);
		s.append(&self.log_bloom);
		s.append(&self.difficulty);
		s.append(&self.number);
		s.append(&self.gas_limit);
		s.append(&self.gas_used);
		s.append(&self.timestamp);
		s.append(&self.extra_data);
		s.append(&self.mix_digest);
		s.append(&self.nonce);

		s.out().to_vec()
	}
}

/// A record of execution for a `LOG` operation.
#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug)]
pub struct LogEntry {
	/// The address of the contract executing at the point of the `LOG` operation.
	pub address: Address,
	/// The topics associated with the `LOG` operation.
	pub topics: Vec<Hash>,
	/// The data associated with the `LOG` operation.
	pub data: Bytes,
}
impl LogEntry {
	/// Calculates the bloom of this log entry.
	pub fn bloom(&self) -> Bloom {
		self.topics.iter().fold(
			Bloom::from(Input::Raw(self.address.as_bytes())),
			|mut b, t| {
				b.accrue(Input::Raw(t.as_bytes()));
				b
			},
		)
	}
}

/// Convert public key into corresponding ethereum address.
pub fn public_to_address(public: &[u8; 64]) -> Address {
	let hash = hashing::keccak_256(public);
	let mut result = Address::zero();

	result.as_bytes_mut().copy_from_slice(&hash[12..]);

	result
}

/// Check ethereum merkle proof.
/// Returns Ok(computed-root) if check succeeds.
/// Returns Err(computed-root) if check fails.
pub fn check_merkle_proof<T: AsRef<[u8]>>(
	expected_root: Hash,
	items: impl Iterator<Item = T>,
) -> Result<Hash, Hash> {
	let computed_root = compute_merkle_root(items);

	if computed_root == expected_root {
		Ok(computed_root)
	} else {
		Err(computed_root)
	}
}

/// Compute ethereum merkle root.
pub fn compute_merkle_root<T: AsRef<[u8]>>(items: impl Iterator<Item = T>) -> Hash {
	struct Keccak256Hasher;
	impl Hasher for Keccak256Hasher {
		type Out = Hash;
		type StdHasher = plain_hasher::PlainHasher;

		const LENGTH: usize = 32;

		fn hash(x: &[u8]) -> Self::Out {
			hashing::keccak_256(x).into()
		}
	}

	triehash::ordered_trie_root::<Keccak256Hasher, _>(items)
}

#[cfg(test)]
mod tests {
	use crate::*;

	#[test]
	fn deserialize_bsc_header_should_work() {
		let header = serde_json::from_str::<BscHeader>(r#"{
			"difficulty": "0x2",
			"extraData": "0xd883010100846765746888676f312e31352e35856c696e7578000000fc3ca6b72465176c461afb316ebc773c61faee85a6515daa295e26495cef6f69dfa69911d9d8e4f3bbadb89b29a97c6effb8a411dabc6adeefaa84f5067c8bbe2d4c407bbe49438ed859fe965b140dcf1aab71a93f349bbafec1551819b8be1efea2fc46ca749aa14430b3230294d12c6ab2aac5c2cd68e80b16b581685b1ded8013785d6623cc18d214320b6bb6475970f657164e5b75689b64b7fd1fa275f334f28e1872b61c6014342d914470ec7ac2975be345796c2b7ae2f5b9e386cd1b50a4550696d957cb4900f03a8b6c8fd93d6f4cea42bbb345dbc6f0dfdb5bec739bb832254baf4e8b4cc26bd2b52b31389b56e98b9f8ccdafcc39f3c7d6ebf637c9151673cbc36b88a6f79b60359f141df90a0c745125b131caaffd12b8f7166496996a7da21cf1f1b04d9b3e26a3d077be807dddb074639cd9fa61b47676c064fc50d62cce2fd7544e0b2cc94692d4a704debef7bcb61328e2d3a739effcd3a99387d015e260eefac72ebea1e9ae3261a475a27bb1028f140bc2a7c843318afdea0a6e3c511bbd10f4519ece37dc24887e11b55dee226379db83cffc681495730c11fdde79ba4c0c0670403d7dfc4c816a313885fe04b850f96f27b2e9fd88b147c882ad7caf9b964abfe6543625fcca73b56fe29d3046831574b0681d52bf5383d6f2187b6276c100",
			"gasLimit": "0x38ff37a",
			"gasUsed": "0x1364017",
			"logsBloom": "0x2c30123db854d838c878e978cd2117896aa092e4ce08f078424e9ec7f2312f1909b35e579fb2702d571a3be04a8f01328e51af205100a7c32e3dd8faf8222fcf03f3545655314abf91c4c0d80cea6aa46f122c2a9c596c6a99d5842786d40667eb195877bbbb128890a824506c81a9e5623d4355e08a16f384bf709bf4db598bbcb88150abcd4ceba89cc798000bdccf5cf4d58d50828d3b7dc2bc5d8a928a32d24b845857da0b5bcf2c5dec8230643d4bec452491ba1260806a9e68a4a530de612e5c2676955a17400ce1d4fd6ff458bc38a8b1826e1c1d24b9516ef84ea6d8721344502a6c732ed7f861bb0ea017d520bad5fa53cfc67c678a2e6f6693c8ee",
			"miner": "0xe9ae3261a475a27bb1028f140bc2a7c843318afd",
			"mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
			"nonce": "0x0000000000000000",
			"number": "0x7594c8",
			"parentHash": "0x5cb4b6631001facd57be810d5d1383ee23a31257d2430f097291d25fc1446d4f",
			"receiptsRoot": "0x1bfba16a9e34a12ff7c4b88be484ccd8065b90abea026f6c1f97c257fdb4ad2b",
			"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
			"stateRoot": "0xa6cd7017374dfe102e82d2b3b8a43dbe1d41cc0e4569f3dc45db6c4e687949ae",
			"timestamp": "0x60ac7137",
			"transactionsRoot": "0x657f5876113ac9abe5cf0460aa8d6b3b53abfc336cea4ab3ee594586f8b584ca"
		}"#).unwrap();

		assert_eq!(
			array_bytes::hex_into_unchecked::<_, Hash, 32>(
				"0x7e1db1179427e17c11a42019f19a3dddf326b6177b0266749639c85c78c607bb"
			),
			header.compute_hash()
		);
	}
}
