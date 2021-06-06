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

pub use primitive_types::{H160 as Address, H256 as Hash};

// --- crates.io ---
use codec::{Decode, Encode};
use ethbloom::{Bloom, Input};
use primitive_types::U256;
use rlp::{DecoderError, Rlp, RlpStream};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
// --- substrate ---
use sp_io::hashing;
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

pub type Bytes = Vec<u8>;

/// Raw (RLP-encoded) ethereum transaction.
pub type RawTransaction = Vec<u8>;

#[cfg(feature = "std")]
use serde_big_array::big_array;
#[cfg(feature = "std")]
serde_big_array::big_array! { BigArray; }

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
#[derive(Encode, Decode, Default, RuntimeDebug, PartialEq, Clone, Copy)]
pub struct HeaderId {
	/// Header number.
	pub number: u64,
	/// Header hash.
	pub hash: Hash,
}

/// An BSC header.
#[derive(Clone, Default, Encode, Decode, PartialEq, RuntimeDebug)]
#[cfg_attr(
	feature = "std",
	derive(Serialize, Deserialize),
	serde(rename_all = "camelCase")
)]
pub struct BSCHeader {
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
	#[cfg_attr(feature = "std", serde(deserialize_with = "array_bytes::hexd2num"))]
	pub number: u64,
	/// Block gas limit.
	pub gas_limit: U256,
	/// Gas used for contracts execution.
	pub gas_used: U256,
	/// Block timestamp.
	#[cfg_attr(feature = "std", serde(deserialize_with = "array_bytes::hexd2num"))]
	pub timestamp: u64,
	/// Block extra data.
	#[cfg_attr(feature = "std", serde(deserialize_with = "array_bytes::hexd2bytes"))]
	pub extra_data: Bytes,
	/// MixDigest
	#[cfg_attr(feature = "std", serde(rename = "mixHash"))]
	pub mix_digest: Hash,
	/// Nonce(64 bit in ethereum)
	#[cfg_attr(feature = "std", serde(deserialize_with = "array_bytes::hexd2bytes"))]
	pub nonce: Bytes,
}
impl BSCHeader {
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
		s.append(&Bloom::from(self.log_bloom.0));
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
		s.append(&Bloom::from(self.log_bloom.0));
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

/// Parsed ethereum transaction.
#[derive(PartialEq, RuntimeDebug)]
pub struct Transaction {
	/// Sender address.
	pub sender: Address,
	/// Unsigned portion of ethereum transaction.
	pub unsigned: UnsignedTransaction,
}

/// Unsigned portion of ethereum transaction.
#[derive(Clone, PartialEq, RuntimeDebug)]
pub struct UnsignedTransaction {
	/// Sender nonce.
	pub nonce: U256,
	/// Gas price.
	pub gas_price: U256,
	/// Gas limit.
	pub gas: U256,
	/// Transaction destination address. None if it is contract creation transaction.
	pub to: Option<Address>,
	/// Value.
	pub value: U256,
	/// Associated data.
	pub payload: Bytes,
}
impl UnsignedTransaction {
	/// Decode unsigned portion of raw transaction RLP.
	pub fn decode_rlp(raw_tx: &[u8]) -> Result<Self, DecoderError> {
		let tx_rlp = Rlp::new(raw_tx);
		let to = tx_rlp.at(3)?;
		Ok(UnsignedTransaction {
			nonce: tx_rlp.val_at(0)?,
			gas_price: tx_rlp.val_at(1)?,
			gas: tx_rlp.val_at(2)?,
			to: match to.is_empty() {
				false => Some(to.as_val()?),
				true => None,
			},
			value: tx_rlp.val_at(4)?,
			payload: tx_rlp.val_at(5)?,
		})
	}

	/// Returns message that has to be signed to sign this transaction.
	pub fn message(&self, chain_id: Option<u64>) -> Hash {
		hashing::keccak_256(&self.rlp(chain_id)).into()
	}

	/// Returns unsigned transaction RLP.
	pub fn rlp(&self, chain_id: Option<u64>) -> Bytes {
		let mut stream = RlpStream::new_list(if chain_id.is_some() { 9 } else { 6 });

		self.rlp_to(chain_id, &mut stream);

		stream.out().to_vec()
	}

	/// Encode to given rlp stream.
	pub fn rlp_to(&self, chain_id: Option<u64>, stream: &mut RlpStream) {
		stream.append(&self.nonce);
		stream.append(&self.gas_price);
		stream.append(&self.gas);

		match self.to {
			Some(to) => stream.append(&to),
			None => stream.append(&""),
		};

		stream.append(&self.value);
		stream.append(&self.payload);

		if let Some(chain_id) = chain_id {
			stream.append(&chain_id);
			stream.append(&0u8);
			stream.append(&0u8);
		}
	}
}

/// Transaction outcome store in the receipt.
#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug)]
pub enum TransactionOutcome {
	/// Status and state root are unknown under EIP-98 rules.
	Unknown,
	/// State root is known. Pre EIP-98 and EIP-658 rules.
	StateRoot(Hash),
	/// Status code is known. EIP-658 rules.
	StatusCode(u8),
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

/// Decode Ethereum transaction.
pub fn transaction_decode_rlp(raw_tx: &[u8]) -> Result<Transaction, DecoderError> {
	// parse transaction fields
	let unsigned = UnsignedTransaction::decode_rlp(raw_tx)?;
	let tx_rlp = Rlp::new(raw_tx);
	let v: u64 = tx_rlp.val_at(6)?;
	let r: U256 = tx_rlp.val_at(7)?;
	let s: U256 = tx_rlp.val_at(8)?;
	// reconstruct signature
	let mut signature = [0u8; 65];
	let (chain_id, v) = match v {
		v if v == 27u64 => (None, 0),
		v if v == 28u64 => (None, 1),
		v if v >= 35u64 => (Some((v - 35) / 2), ((v - 1) % 2) as u8),
		_ => (None, 4),
	};

	r.to_big_endian(&mut signature[0..32]);
	s.to_big_endian(&mut signature[32..64]);
	signature[64] = v;

	// reconstruct message that has been signed
	let message = unsigned.message(chain_id);
	// recover tx sender
	let sender_public =
		sp_io::crypto::secp256k1_ecdsa_recover(&signature, &message.as_fixed_bytes())
			.map_err(|_| rlp::DecoderError::Custom("Failed to recover transaction sender"))?;
	let sender_address = public_to_address(&sender_public);

	Ok(Transaction {
		sender: sender_address,
		unsigned,
	})
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
fn check_merkle_proof<T: AsRef<[u8]>>(
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

	impl hash_db::Hasher for Keccak256Hasher {
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
		let header_json = r#"{
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
		}"#;
		let header = serde_json::from_str::<BSCHeader>(header_json).unwrap();

		assert_eq!(
			array_bytes::hex_into_unchecked::<_, Hash, 32>(
				"0x7e1db1179427e17c11a42019f19a3dddf326b6177b0266749639c85c78c607bb"
			),
			header.compute_hash()
		);
	}

	#[test]
	fn transfer_transaction_decode_works() {
		// value transfer transaction
		// https://etherscan.io/tx/0xb9d4ad5408f53eac8627f9ccd840ba8fb3469d55cd9cc2a11c6e049f1eef4edd
		// https://etherscan.io/getRawTx?tx=0xb9d4ad5408f53eac8627f9ccd840ba8fb3469d55cd9cc2a11c6e049f1eef4edd
		let raw_tx = array_bytes::hex2bytes_unchecked("f86c0a85046c7cfe0083016dea94d1310c1e038bc12865d3d3997275b3e4737c6302880b503be34d9fe80080269fc7eaaa9c21f59adf8ad43ed66cf5ef9ee1c317bd4d32cd65401e7aaca47cfaa0387d79c65b90be6260d09dcfb780f29dd8133b9b1ceb20b83b7e442b4bfc30cb");
		assert_eq!(
			transaction_decode_rlp(&raw_tx),
			Ok(Transaction {
				sender: array_bytes::hex_into_unchecked("67835910d32600471f388a137bbff3eb07993c04"),
				unsigned: UnsignedTransaction {
					nonce: 10.into(),
					gas_price: 19000000000u64.into(),
					gas: 93674.into(),
					to: Some(array_bytes::hex_into_unchecked(
						"d1310c1e038bc12865d3d3997275b3e4737c6302"
					)),
					value: 815217380000000000_u64.into(),
					payload: Default::default(),
				}
			}),
		);

		// Kovan value transfer transaction
		// https://kovan.etherscan.io/tx/0x3b4b7bd41c1178045ccb4753aa84c1ef9864b4d712fa308b228917cd837915da
		// https://kovan.etherscan.io/getRawTx?tx=0x3b4b7bd41c1178045ccb4753aa84c1ef9864b4d712fa308b228917cd837915da
		let raw_tx = array_bytes::hex2bytes_unchecked("f86a822816808252089470c1ccde719d6f477084f07e4137ab0e55f8369f8930cf46e92063afd8008078a00e4d1f4d8aa992bda3c105ff3d6e9b9acbfd99facea00985e2131029290adbdca028ea29a46a4b66ec65b454f0706228e3768cb0ecf755f67c50ddd472f11d5994");
		assert_eq!(
			transaction_decode_rlp(&raw_tx),
			Ok(Transaction {
				sender: array_bytes::hex_into_unchecked("faadface3fbd81ce37b0e19c0b65ff4234148132"),
				unsigned: UnsignedTransaction {
					nonce: 10262.into(),
					gas_price: 0.into(),
					gas: 21000.into(),
					to: Some(array_bytes::hex_into_unchecked(
						"70c1ccde719d6f477084f07e4137ab0e55f8369f",
					)),
					value: 900379597077600000000_u128.into(),
					payload: Default::default(),
				},
			}),
		);
	}

	#[test]
	fn payload_transaction_decode_works() {
		// contract call transaction
		// https://etherscan.io/tx/0xdc2b996b4d1d6922bf6dba063bfd70913279cb6170967c9bb80252aeb061cf65
		// https://etherscan.io/getRawTx?tx=0xdc2b996b4d1d6922bf6dba063bfd70913279cb6170967c9bb80252aeb061cf65
		let raw_tx = array_bytes::hex2bytes_unchecked("f8aa76850430e234008301500094dac17f958d2ee523a2206206994597c13d831ec780b844a9059cbb000000000000000000000000e08f35f66867a454835b25118f1e490e7f9e9a7400000000000000000000000000000000000000000000000000000000004c4b4025a0964e023999621dc3d4d831c43c71f7555beb6d1192dee81a3674b3f57e310f21a00f229edd86f841d1ee4dc48cc16667e2283817b1d39bae16ced10cd206ae4fd4");
		assert_eq!(
			transaction_decode_rlp(&raw_tx),
			Ok(Transaction {
				sender: array_bytes::hex_into_unchecked("2b9a4d37bdeecdf994c4c9ad7f3cf8dc632f7d70"),
				unsigned: UnsignedTransaction {
					nonce: 118.into(),
					gas_price: 18000000000u64.into(),
					gas: 86016.into(),
					to: Some(array_bytes::hex_into_unchecked("dac17f958d2ee523a2206206994597c13d831ec7")),
					value: 0.into(),
					payload: array_bytes::hex2bytes_unchecked("a9059cbb000000000000000000000000e08f35f66867a454835b25118f1e490e7f9e9a7400000000000000000000000000000000000000000000000000000000004c4b40"),
				},
			}),
		);

		// Kovan contract call transaction
		// https://kovan.etherscan.io/tx/0x2904b4451d23665492239016b78da052d40d55fdebc7304b38e53cf6a37322cf
		// https://kovan.etherscan.io/getRawTx?tx=0x2904b4451d23665492239016b78da052d40d55fdebc7304b38e53cf6a37322cf
		let raw_tx = array_bytes::hex2bytes_unchecked("f8ac8302200b843b9aca00830271009484dd11eb2a29615303d18149c0dbfa24167f896680b844a9059cbb00000000000000000000000001503dfc5ad81bf630d83697e98601871bb211b600000000000000000000000000000000000000000000000000000000000027101ba0ce126d2cca81f5e245f292ff84a0d915c0a4ac52af5c51219db1e5d36aa8da35a0045298b79dac631907403888f9b04c2ab5509fe0cc31785276d30a40b915fcf9");
		assert_eq!(
			transaction_decode_rlp(&raw_tx),
			Ok(Transaction {
				sender: array_bytes::hex_into_unchecked("617da121abf03d4c1af572f5a4e313e26bef7bdc"),
				unsigned: UnsignedTransaction {
					nonce: 139275.into(),
					gas_price: 1000000000.into(),
					gas: 160000.into(),
					to: Some(array_bytes::hex_into_unchecked("84dd11eb2a29615303d18149c0dbfa24167f8966")),
					value: 0.into(),
					payload: array_bytes::hex2bytes_unchecked("a9059cbb00000000000000000000000001503dfc5ad81bf630d83697e98601871bb211b60000000000000000000000000000000000000000000000000000000000002710"),
				},
			}),
		);
	}
}
