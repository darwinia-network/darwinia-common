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

pub use Receipt as EthereumReceipt;
pub use ReceiptProof as EthereumReceiptProof;
pub use TransactionIndex as EthereumTransactionIndex;

// --- alloc ---
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
// --- crates.io ---
#[cfg(any(feature = "full-codec", test))]
use codec::{Decode, Encode};
use ethbloom::{Bloom, Input};
#[cfg(any(feature = "full-rlp", test))]
use rlp::{Decodable, DecoderError, Encodable, Rlp, RlpStream};
#[cfg(any(feature = "full-rlp", test))]
use rlp_derive::{RlpDecodable, RlpEncodable};
use sp_debug_derive::RuntimeDebug;
// --- darwinia-network ---
#[cfg(any(feature = "full-rlp", test))]
use crate::error::*;
use crate::{H256, U256, *};
#[cfg(any(feature = "full-rlp", test))]
use merkle_patricia_trie::{trie::Trie, MerklePatriciaTrie, Proof};

pub type TransactionIndex = (H256, u64);

#[cfg_attr(any(feature = "full-codec", test), derive(Encode, Decode))]
#[derive(Clone, PartialEq, Eq, RuntimeDebug)]
pub enum TransactionOutcome {
	/// Status and state root are unknown under EIP-98 rules.
	Unknown,
	/// State root is known. Pre EIP-98 and EIP-658 rules.
	StateRoot(H256),
	/// Status code is known. EIP-658 rules.
	StatusCode(u8),
}

#[cfg_attr(any(feature = "full-codec", test), derive(Encode, Decode))]
#[cfg_attr(any(feature = "full-rlp", test), derive(RlpEncodable, RlpDecodable))]
#[derive(Clone, PartialEq, Eq, RuntimeDebug)]
pub struct LogEntry {
	/// The address of the contract executing at the point of the `LOG` operation.
	pub address: Address,
	/// The topics associated with the `LOG` operation.
	pub topics: Vec<H256>,
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

#[cfg_attr(any(feature = "full-codec", test), derive(Encode, Decode))]
#[derive(Clone, PartialEq, Eq, RuntimeDebug)]
pub struct Receipt {
	/// The total gas used in the block following execution of the transaction.
	pub gas_used: U256,
	/// The OR-wide combination of all logs' blooms for this transaction.
	pub log_bloom: Bloom,
	/// The logs stemming from this transaction.
	pub logs: Vec<LogEntry>,
	/// Transaction outcome.
	pub outcome: TransactionOutcome,
}
impl Receipt {
	/// Create a new receipt.
	pub fn new(outcome: TransactionOutcome, gas_used: U256, logs: Vec<LogEntry>) -> Self {
		Self {
			gas_used,
			log_bloom: logs.iter().fold(Bloom::default(), |mut b, l| {
				b.accrue_bloom(&l.bloom());
				b
			}),
			logs,
			outcome,
		}
	}

	#[cfg(any(feature = "full-rlp", test))]
	pub fn verify_proof_and_generate(
		receipt_root: &H256,
		proof_record: &ReceiptProof,
	) -> Result<Self, Error> {
		let proof = rlp::decode::<Proof>(&proof_record.proof).map_err(RlpError::from)?;
		let key = rlp::encode(&proof_record.index);
		let value = MerklePatriciaTrie::verify_proof(receipt_root.0.to_vec(), &key, proof)?
			.ok_or(ProofError::TrieKeyNotExist)?;
		let receipt = rlp::decode(&value).map_err(RlpError::from)?;

		Ok(receipt)
	}
}
#[cfg(any(feature = "full-rlp", test))]
impl Encodable for Receipt {
	fn rlp_append(&self, s: &mut RlpStream) {
		match self.outcome {
			TransactionOutcome::Unknown => {
				s.begin_list(3);
			}
			TransactionOutcome::StateRoot(ref root) => {
				s.begin_list(4);
				s.append(root);
			}
			TransactionOutcome::StatusCode(ref status_code) => {
				s.begin_list(4);
				s.append(status_code);
			}
		}
		s.append(&self.gas_used);
		s.append(&self.log_bloom);
		s.append_list(&self.logs);
	}
}
#[cfg(any(feature = "full-rlp", test))]
impl Decodable for Receipt {
	fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
		if rlp.item_count()? == 3 {
			Ok(Receipt {
				outcome: TransactionOutcome::Unknown,
				gas_used: rlp.val_at(0)?,
				log_bloom: rlp.val_at(1)?,
				logs: rlp.list_at(2)?,
			})
		} else {
			Ok(Receipt {
				gas_used: rlp.val_at(1)?,
				log_bloom: rlp.val_at(2)?,
				logs: rlp.list_at(3)?,
				outcome: {
					let first = rlp.at(0)?;
					if first.is_data() && first.data()?.len() <= 1 {
						TransactionOutcome::StatusCode(first.as_val()?)
					} else {
						TransactionOutcome::StateRoot(first.as_val()?)
					}
				},
			})
		}
	}
}

#[cfg_attr(any(feature = "full-codec", test), derive(Encode, Decode))]
#[cfg_attr(any(feature = "full-serde", test), derive(serde::Deserialize))]
#[derive(Clone, PartialEq, Eq, RuntimeDebug)]
pub struct ReceiptProof {
	pub index: u64,
	#[cfg_attr(
		any(feature = "full-serde", test),
		serde(deserialize_with = "array_bytes::hexd2bytes")
	)]
	pub proof: Bytes,
	pub header_hash: H256,
}

#[cfg(test)]
mod tests {
	// --- std ---
	use std::str::FromStr;
	// --- crates ---
	use keccak_hasher::KeccakHasher;
	// --- darwinia ---
	use super::*;

	#[inline]
	fn construct_receipts(
		root: Option<H256>,
		gas_used: U256,
		status: Option<u8>,
		log_entries: Vec<LogEntry>,
	) -> Receipt {
		Receipt::new(
			if root.is_some() {
				TransactionOutcome::StateRoot(root.unwrap())
			} else {
				TransactionOutcome::StatusCode(status.unwrap())
			},
			gas_used,
			log_entries,
		)
	}

	/// ropsten tx hash: 0xce62c3d1d2a43cfcc39707b98de53e61a7ef7b7f8853e943d85e511b3451aa7e
	#[test]
	fn test_basic() {
		// https://ropsten.etherscan.io/tx/0xce62c3d1d2a43cfcc39707b98de53e61a7ef7b7f8853e943d85e511b3451aa7e#eventlog
		let log_entries = vec![LogEntry {
			address: Address::from_str("ad52e0f67b6f44cd5b9a6f4fbc7c0f78f37e094b").unwrap(),
			topics: vec![
				array_bytes::hex_into_unchecked(
					"0x6775ce244ff81f0a82f87d6fd2cf885affb38416e3a04355f713c6f008dd126a",
				),
				array_bytes::hex_into_unchecked(
					"0x0000000000000000000000000000000000000000000000000000000000000006",
				),
				array_bytes::hex_into_unchecked(
					"0x0000000000000000000000000000000000000000000000000000000000000000",
				)
			],
			data: array_bytes::hex2bytes_unchecked("0x00000000000000000000000074241db5f3ebaeecf9506e4ae9881860933416048eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48000000000000000000000000000000000000000000000000002386f26fc10000"),
		}];
		let r = construct_receipts(None, 1123401.into(), Some(1), log_entries);

		// TODO: Check the log bloom generation logic
		assert_eq!(r.log_bloom, Bloom::from_str(
			"00000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000000000000000000820000000000000020000000000000000000800000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000000200000000020000000000000000000000000000080000000000000800000000000000000000000"
		).unwrap());
	}

	/// kovan tx hash: 0xaaf52845694258509cbdd30ea21894b4e685eb4cdbb13dd298f925fe6e51db35
	/// block number: 13376543 (only a tx in this block, which is above)
	/// from: 0x4aea6cfc5bd14f2308954d544e1dc905268357db
	/// to: 0xa24df0420de1f3b8d740a52aaeb9d55d6d64478e (a contract)
	/// receipts_root in block#13376543: 0xc789eb8b7f5876f4df4f8ae16f95c9881eabfb700ee7d8a00a51fb4a71afbac9
	/// to check if receipts_root in block-header can be pre-computed.
	#[test]
	fn check_receipts() {
		let expected_root = array_bytes::hex_into_unchecked(
			"0xc789eb8b7f5876f4df4f8ae16f95c9881eabfb700ee7d8a00a51fb4a71afbac9",
		);
		let log_entries = vec![LogEntry {
			address: Address::from_str("a24df0420de1f3b8d740a52aaeb9d55d6d64478e").unwrap(),
			topics: vec![array_bytes::hex_into_unchecked(
				"0xf36406321d51f9ba55d04e900c1d56caac28601524e09d53e9010e03f83d7d00",
			)],
			data: array_bytes::hex2bytes_unchecked("0x0000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000363384a3868b9000000000000000000000000000000000000000000000000000000005d75f54f0000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000e53504f5450582f4241542d455448000000000000000000000000000000000000"),
		}];
		let receipts = vec![Receipt::new(
			TransactionOutcome::StatusCode(1),
			73705.into(),
			log_entries,
		)];
		let receipts_root = H256(triehash::ordered_trie_root::<KeccakHasher, _>(
			receipts.iter().map(|x| ::rlp::encode(x)),
		));

		assert_eq!(receipts_root, expected_root);
	}
}
