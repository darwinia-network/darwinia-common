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

pub use Header as EthereumHeader;

// --- core ---
#[cfg(any(feature = "full-serde", test))]
use core::fmt;
// --- alloc ---
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
// --- crates.io ---
#[cfg(any(feature = "full-serde", test))]
use array_bytes::TryFromHex;
#[cfg(any(feature = "full-codec", test))]
use codec::{Decode, Encode};
use ethbloom::Bloom;
#[cfg(any(feature = "full-rlp", test))]
use keccak_hash::keccak;
use keccak_hash::{KECCAK_EMPTY_LIST_RLP, KECCAK_NULL_RLP};
#[cfg(any(feature = "full-rlp", test))]
use rlp::{DecoderError, Encodable, Rlp, RlpStream};
#[cfg(any(feature = "full-codec", test))]
use scale_info::TypeInfo;
#[cfg(any(feature = "full-serde", test))]
use serde::{
	de::{Error, IgnoredAny, MapAccess, Visitor},
	Deserialize, Deserializer,
};
use sp_debug_derive::RuntimeDebug;
// --- darwinia-network ---
#[cfg(any(feature = "full-serde", test))]
use crate::H64;
use crate::{Address, BlockNumber, Bytes, H256, U256};

#[cfg(any(feature = "full-rlp", test))]
#[cfg_attr(any(feature = "full-codec", test), derive(Encode, Decode))]
#[derive(Clone, Copy, PartialEq, Eq, RuntimeDebug)]
enum Seal {
	/// The seal/signature is included.
	With,
	/// The seal/signature is not included.
	Without,
}

#[cfg_attr(any(feature = "full-codec", test), derive(Encode, Decode, TypeInfo))]
#[derive(Clone, Eq, RuntimeDebug)]
pub struct Header {
	pub parent_hash: H256,
	pub timestamp: u64,
	pub number: BlockNumber,
	pub author: Address,

	pub transactions_root: H256,
	pub uncles_hash: H256,
	pub extra_data: Bytes,

	pub state_root: H256,
	pub receipts_root: H256,
	pub log_bloom: Bloom,
	/// Gas used for contracts execution.
	pub gas_used: U256,
	pub gas_limit: U256,
	pub difficulty: U256,
	/// Vector of post-RLP-encoded fields.
	pub seal: Vec<Bytes>,

	/// Base fee per gas. Introduced by EIP1559.
	pub base_fee_per_gas: Option<U256>,

	/// Memoized hash of that header and the seal.
	pub hash: Option<H256>,
}
#[cfg(any(feature = "full-serde", test))]
impl<'de> Deserialize<'de> for Header {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		#[allow(non_camel_case_types)]
		#[derive(Debug, Deserialize)]
		#[serde(field_identifier)]
		enum Field {
			baseFeePerGas,
			difficulty,
			extraData,
			gasLimit,
			gasUsed,
			hash,
			logsBloom,
			miner,
			mixHash,
			nonce,
			number,
			parentHash,
			receiptsRoot,
			sha3Uncles,
			// size,
			stateRoot,
			timestamp,
			// totalDifficulty,
			transactionsRoot,
		}

		struct HeaderVisitor;
		impl<'de> Visitor<'de> for HeaderVisitor {
			type Value = Header;

			fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
				write!(formatter, "a infura API like header spec")
			}

			fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
			where
				V: MapAccess<'de>,
			{
				macro_rules! check_and_set_option {
					(check: $field:ident) => {
						if $field.is_some() {
							return Err(Error::duplicate_field(stringify!($field)));
						}
					};
					($field:ident) => {{
						check_and_set_option!(check: $field);

						$field = Some(map.next_value()?);
					}};
					($field:ident, $hex2:expr) => {{
						check_and_set_option!(check: $field);

						$field = Some(
							$hex2(map.next_value::<&str>()?)
								.map_err(|e| Error::custom(format_args!("{:?}", e)))?,
						);
					}};
				}

				macro_rules! check_missing_field {
					($field:ident) => {{
						$field.ok_or_else(|| Error::missing_field(stringify!($field)))?
					}};
				}

				#[allow(non_snake_case)]
				let mut baseFeePerGas = None;
				let mut difficulty = None;
				#[allow(non_snake_case)]
				let mut extraData = None;
				#[allow(non_snake_case)]
				let mut gasLimit = None;
				#[allow(non_snake_case)]
				let mut gasUsed = None;
				let mut hash = None;
				#[allow(non_snake_case)]
				let mut logsBloom = None;
				let mut miner = None;
				#[allow(non_snake_case)]
				let mut mixHash = <Option<H256>>::None;
				let mut nonce = <Option<H64>>::None;
				let mut number = None;
				#[allow(non_snake_case)]
				let mut parentHash = None;
				#[allow(non_snake_case)]
				let mut receiptsRoot = None;
				#[allow(non_snake_case)]
				let mut sha3Uncles = None;
				// let mut size = None;
				#[allow(non_snake_case)]
				let mut stateRoot = None;
				let mut timestamp = None;
				// #[allow(non_snake_case)]
				// let mut totalDifficulty = None;
				#[allow(non_snake_case)]
				let mut transactionsRoot = None;

				loop {
					match map.next_key() {
						Ok(Some(key)) => match key {
							Field::baseFeePerGas => check_and_set_option!(baseFeePerGas),
							Field::difficulty => check_and_set_option!(difficulty),
							Field::extraData => {
								check_and_set_option!(extraData, array_bytes::hex2bytes)
							}
							Field::gasLimit => check_and_set_option!(gasLimit),
							Field::gasUsed => check_and_set_option!(gasUsed),
							Field::hash => check_and_set_option!(hash),
							Field::logsBloom => check_and_set_option!(logsBloom),
							Field::miner => check_and_set_option!(miner),
							Field::mixHash => check_and_set_option!(mixHash),
							Field::nonce => check_and_set_option!(nonce),
							Field::number => {
								check_and_set_option!(number, TryFromHex::try_from_hex)
							}
							Field::parentHash => check_and_set_option!(parentHash),
							Field::receiptsRoot => check_and_set_option!(receiptsRoot),
							Field::sha3Uncles => check_and_set_option!(sha3Uncles),
							// Field::size => {}
							Field::stateRoot => check_and_set_option!(stateRoot),
							Field::timestamp => {
								check_and_set_option!(timestamp, TryFromHex::try_from_hex)
							}
							// Field::totalDifficulty => {}
							Field::transactionsRoot => check_and_set_option!(transactionsRoot),
						},
						Ok(None) => break,
						Err(_) => {
							map.next_value::<IgnoredAny>()?;
						}
					}
				}

				Ok(Header {
					parent_hash: check_missing_field!(parentHash),
					timestamp: check_missing_field!(timestamp),
					number: check_missing_field!(number),
					author: check_missing_field!(miner),
					transactions_root: check_missing_field!(transactionsRoot),
					uncles_hash: check_missing_field!(sha3Uncles),
					extra_data: check_missing_field!(extraData),
					state_root: check_missing_field!(stateRoot),
					receipts_root: check_missing_field!(receiptsRoot),
					log_bloom: check_missing_field!(logsBloom),
					gas_used: check_missing_field!(gasUsed),
					gas_limit: check_missing_field!(gasLimit),
					difficulty: check_missing_field!(difficulty),
					seal: vec![
						rlp::encode(&check_missing_field!(mixHash)).to_vec(),
						rlp::encode(&check_missing_field!(nonce)).to_vec(),
					],
					base_fee_per_gas: baseFeePerGas,
					hash,
					..Default::default()
				})
			}
		}

		const FIELDS: &'static [&'static str] = &[
			"parent_hash",
			"timestamp",
			"number",
			"author",
			"transactions_root",
			"uncles_hash",
			"extra_data",
			"state_root",
			"receipts_root",
			"log_bloom",
			"gas_used",
			"gas_limit",
			"difficulty",
			"seal",
			"base_fee_per_gas",
			"hash",
		];

		deserializer.deserialize_struct("Header", FIELDS, HeaderVisitor)
	}
}
impl Header {
	#[cfg(any(feature = "full-rlp", test))]
	pub fn decode_rlp(r: &Rlp, eip1559_transition: BlockNumber) -> Result<Self, DecoderError> {
		let mut header = Header {
			parent_hash: r.val_at(0)?,
			uncles_hash: r.val_at(1)?,
			author: r.val_at(2)?,
			state_root: r.val_at(3)?,
			transactions_root: r.val_at(4)?,
			receipts_root: r.val_at(5)?,
			log_bloom: r.val_at(6)?,
			difficulty: r.val_at(7)?,
			number: r.val_at(8)?,
			gas_limit: r.val_at(9)?,
			gas_used: r.val_at(10)?,
			timestamp: r.val_at(11)?,
			extra_data: r.val_at(12)?,
			seal: Vec::new(),
			base_fee_per_gas: None,
			hash: keccak(r.as_raw()).into(),
		};

		if header.number >= eip1559_transition {
			for i in 13..r.item_count()? - 1 {
				header.seal.push(r.at(i)?.as_raw().to_vec())
			}
			header.base_fee_per_gas = Some(r.val_at(r.item_count()? - 1)?);
		} else {
			for i in 13..r.item_count()? {
				header.seal.push(r.at(i)?.as_raw().to_vec())
			}
		}

		Ok(header)
	}

	#[cfg(any(feature = "full-rlp", test))]
	pub fn decode_rlp_list(
		rlp: &Rlp,
		eip1559_transition: BlockNumber,
	) -> Result<Vec<Self>, DecoderError> {
		if !rlp.is_list() {
			// at least one byte needs to be present
			return Err(DecoderError::RlpIncorrectListLen);
		}

		let mut output = Vec::with_capacity(rlp.item_count()?);

		for h in rlp.iter() {
			output.push(Self::decode_rlp(&h, eip1559_transition)?);
		}

		Ok(output)
	}
}
impl Default for Header {
	fn default() -> Self {
		Header {
			parent_hash: H256::zero(),
			timestamp: 0,
			number: 0,
			author: Address::zero(),
			transactions_root: KECCAK_NULL_RLP,
			uncles_hash: KECCAK_EMPTY_LIST_RLP,
			extra_data: Vec::new(),
			state_root: KECCAK_NULL_RLP,
			receipts_root: KECCAK_NULL_RLP,
			log_bloom: Bloom::default(),
			gas_used: U256::default(),
			gas_limit: U256::default(),
			difficulty: U256::default(),
			seal: Vec::new(),
			base_fee_per_gas: None,
			hash: None,
		}
	}
}
impl PartialEq for Header {
	fn eq(&self, c: &Header) -> bool {
		if let (&Some(ref h1), &Some(ref h2)) = (&self.hash, &c.hash) {
			// More strict check even if hashes equal since Header could be decoded from dispatch call by external
			// Note that this is different implementation compared to Open Ethereum
			// Refer: https://github.com/openethereum/openethereum/blob/v3.0.0-alpha.1/ethcore/types/src/header.rs#L93
			if h1 != h2 {
				return false;
			}
		}

		self.parent_hash == c.parent_hash
			&& self.timestamp == c.timestamp
			&& self.number == c.number
			&& self.author == c.author
			&& self.transactions_root == c.transactions_root
			&& self.uncles_hash == c.uncles_hash
			&& self.extra_data == c.extra_data
			&& self.state_root == c.state_root
			&& self.receipts_root == c.receipts_root
			&& self.log_bloom == c.log_bloom
			&& self.gas_used == c.gas_used
			&& self.gas_limit == c.gas_limit
			&& self.difficulty == c.difficulty
			&& self.base_fee_per_gas == c.base_fee_per_gas
			&& self.seal == c.seal
	}
}
#[cfg(any(feature = "full-rlp", test))]
impl Encodable for Header {
	fn rlp_append(&self, s: &mut RlpStream) {
		self.stream_rlp(s, Seal::With);
	}
}

/// Alter value of given field, reset memoised hash if changed.
fn change_field<T>(hash: &mut Option<H256>, field: &mut T, value: T)
where
	T: PartialEq<T>,
{
	if field != &value {
		*field = value;
		*hash = None;
	}
}

impl Header {
	/// Create a new, default-valued, header.
	pub fn new() -> Self {
		Self::default()
	}

	/// Get the parent_hash field of the header.
	pub fn parent_hash(&self) -> &H256 {
		&self.parent_hash
	}

	/// Get the timestamp field of the header.
	pub fn timestamp(&self) -> u64 {
		self.timestamp
	}

	/// Get the number field of the header.
	pub fn number(&self) -> BlockNumber {
		self.number
	}

	/// Get the author field of the header.
	pub fn author(&self) -> &Address {
		&self.author
	}

	/// Get the extra data field of the header.
	pub fn extra_data(&self) -> &Bytes {
		&self.extra_data
	}

	/// Get the state root field of the header.
	pub fn state_root(&self) -> &H256 {
		&self.state_root
	}

	/// Get the receipts root field of the header.
	pub fn receipts_root(&self) -> &H256 {
		&self.receipts_root
	}

	/// Get the log bloom field of the header.
	pub fn log_bloom(&self) -> &Bloom {
		&self.log_bloom
	}

	/// Get the transactions root field of the header.
	pub fn transactions_root(&self) -> &H256 {
		&self.transactions_root
	}

	/// Get the uncles hash field of the header.
	pub fn uncles_hash(&self) -> &H256 {
		&self.uncles_hash
	}

	/// Get the gas used field of the header.
	pub fn gas_used(&self) -> &U256 {
		&self.gas_used
	}

	/// Get the gas limit field of the header.
	pub fn gas_limit(&self) -> &U256 {
		&self.gas_limit
	}

	/// Get the difficulty field of the header.
	pub fn difficulty(&self) -> &U256 {
		&self.difficulty
	}

	/// Get the seal field of the header.
	pub fn seal(&self) -> &[Bytes] {
		&self.seal
	}

	/// Get the base fee field of the header.
	pub fn base_fee(&self) -> Option<U256> {
		self.base_fee_per_gas
	}

	/// Set the seal field of the header.
	pub fn set_seal(&mut self, a: Vec<Bytes>) {
		change_field(&mut self.hash, &mut self.seal, a)
	}

	/// Set the difficulty field of the header.
	pub fn set_difficulty(&mut self, a: U256) {
		change_field(&mut self.hash, &mut self.difficulty, a);
	}

	/// Get & memoize the hash of this header (keccak of the RLP with seal).
	#[cfg(any(feature = "full-rlp", test))]
	pub fn compute_hash(&mut self) -> H256 {
		let hash = self.hash();
		self.hash = Some(hash);
		hash
	}

	#[cfg(any(feature = "full-rlp", test))]
	pub fn re_compute_hash(&self) -> H256 {
		keccak_hash::keccak(self.rlp(Seal::With))
	}

	/// Get the hash of this header (keccak of the RLP with seal).
	#[cfg(any(feature = "full-rlp", test))]
	pub fn hash(&self) -> H256 {
		self.hash
			.unwrap_or_else(|| keccak_hash::keccak(self.rlp(Seal::With)))
	}

	/// Get the hash of the header excluding the seal
	#[cfg(any(feature = "full-rlp", test))]
	pub fn bare_hash(&self) -> H256 {
		keccak_hash::keccak(self.rlp(Seal::Without))
	}

	/// Get the RLP representation of this Header.
	#[cfg(any(feature = "full-rlp", test))]
	fn rlp(&self, with_seal: Seal) -> Bytes {
		let mut s = RlpStream::new();
		self.stream_rlp(&mut s, with_seal);
		s.out().to_vec()
	}

	/// Place this header into an RLP stream `s`, optionally `with_seal`.
	#[cfg(any(feature = "full-rlp", test))]
	fn stream_rlp(&self, s: &mut RlpStream, with_seal: Seal) {
		let stream_length_without_seal = if self.base_fee_per_gas.is_some() {
			14
		} else {
			13
		};

		if let Seal::With = with_seal {
			s.begin_list(stream_length_without_seal + self.seal.len());
		} else {
			s.begin_list(stream_length_without_seal);
		}

		s.append(&self.parent_hash);
		s.append(&self.uncles_hash);
		s.append(&self.author);
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

		if let Seal::With = with_seal {
			for b in &self.seal {
				s.append_raw(b, 1);
			}
		}

		if self.base_fee_per_gas.is_some() {
			s.append(&self.base_fee_per_gas.unwrap());
		}
	}
}

#[cfg(test)]
mod tests {
	#[cfg(feature = "dag")]
	mod dag {
		// --- github ---
		use ethash::{EthereumPatch, LightDAG};
		// --- darwinia-network ---
		use crate::{header::Header, pow::Seal};

		type DAG = LightDAG<EthereumPatch>;

		#[test]
		fn mix_hash_should_work_for_mainnet_block_0x1() {
			let header = serde_json::from_str::<Header>(r#"{
				"difficulty": "0x3ff800000",
				"extraData": "0x476574682f76312e302e302f6c696e75782f676f312e342e32",
				"gasLimit": "0x1388",
				"gasUsed": "0x0",
				"hash": "0x88e96d4537bea4d9c05d12549907b32561d3bf31f45aae734cdc119f13406cb6",
				"logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
				"miner": "0x05a56e2d52c817161883f50c441c3228cfe54d9f",
				"mixHash": "0x969b900de27b6ac6a67742365dd65f55a0526c41fd18e1b16f1a1215c2e66f59",
				"nonce": "0x539bd4979fef1ec4",
				"number": "0x1",
				"parentHash": "0xd4e56740f876aef8c010b86a40d5f56745a118d0906a34e69aec8c0db1cb8fa3",
				"receiptsRoot": "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
				"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
				"size": "0x219",
				"stateRoot": "0xd67e4d450343046425ae4271474353857ab860dbc0a1dde64b41b5cd3a532bf3",
				"timestamp": "0x55ba4224",
				"totalDifficulty": "0x7ff800000",
				"transactions": [],
				"transactionsRoot": "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
				"uncles": []
			}"#).unwrap();

			let seal = Seal::parse_seal(header.seal()).unwrap();
			let light_dag = DAG::new(header.number.into());
			let partial_header_hash = header.bare_hash();
			let mix_hash = light_dag.hashimoto(partial_header_hash, seal.nonce).0;

			assert_eq!(mix_hash, seal.mix_hash);
		}

		#[test]
		fn mix_hash_should_work_for_mainnet_block_0x93806d() {
			let header = serde_json::from_str::<Header>(r#"{
				"difficulty": "0x7db1e47bc4cb4",
				"extraData": "0x505059452d65746865726d696e652d6575312d32",
				"gasLimit": "0x9895d1",
				"gasUsed": "0x989042",
				"hash": "0x5eccf3a95d2ae352a05ced7de02b6b41b99a780c680af67162f7673b9bc9a00f",
				"logsBloom": "0x0002000005400020000004000040100000000020000010080280a000800008100000100100000000000040021000010100000000005000000000000000001000000000000000400048100008004000000006000801040000010000001000000009000004082200000001c0002000000900000020100000000000001040020000008440000080001108100000000000000000012801000080040004002010001000002401400020002000089200000002000000020080000001100000000100000400010200400410800010200000000400000820000002000100000000004280400040001060000400000080a001280008002000000140004800120000000022",
				"miner": "0xea674fdde714fd979de3edf0f56aa9716b898ec8",
				"mixHash": "0x7daba05fcefc814682e0caf337800780de3f9737fac71826d90eddcedd89b1da",
				"nonce": "0x726446620418cc02",
				"number": "0x93806d",
				"parentHash": "0x6ec166e9a9700acaa59573d5a4874f5a28c6665938a7ca824abd6e011cf73c38",
				"receiptsRoot": "0xf4e94c772cddfea2e94eea2eb3381385b1477ca887adf4da6d1b7b92fdac68cc",
				"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
				"size": "0x1580",
				"stateRoot": "0x63a7b415d8f67152fa7fcf25e919638bd44083c7e8c95497f15b9819ea8acb81",
				"timestamp": "0x5e6c35d2",
				"totalDifficulty": "0x313df92f05f4c80afcf",
				"transactions": [],
				"transactionsRoot": "0xd252a961e83513313ea0b51ee1937e75c3bb31e6290de1fc1a4e0d22eeaa58e9",
				"uncles": []
			}"#).unwrap();

			let seal = Seal::parse_seal(header.seal()).unwrap();
			let light_dag = DAG::new(header.number.into());
			let partial_header_hash = header.bare_hash();
			let mix_hash = light_dag.hashimoto(partial_header_hash, seal.nonce).0;

			assert_eq!(mix_hash, seal.mix_hash);
		}
	}

	// --- std ---
	use std::str::FromStr;
	// --- darwinia-network ---
	use crate::{
		error::{BlockError, Error},
		header::Header,
		pow::EthashPartial,
		H256, H64, U256,
	};

	fn sequential_header() -> (Header, Header) {
		(
			serde_json::from_str(r#"{
				"difficulty": "0x92ac28cbc4930",
				"extraData": "0x5050594520686976656f6e2d6574682d6672",
				"gasLimit": "0x989631",
				"gasUsed": "0x986d77",
				"hash": "0xb80bf91d6f459227a9c617c5d9823ff0b07f1098ea16788676f0b804ecd42f3b",
				"logsBloom": "0x0c7b091bc8ec02401ad12491004e3014e8806390031950181c118580ac61c9a00409022c418162002710a991108a11ca5383d4921d1da46346edc3eb8068481118b005c0b20700414c13916c54011a0922904aa6e255406a33494c84a1426410541819070e04852042410b30030d4c88a5103082284c7d9bd42090322ae883e004224e18db4d858a0805d043e44a855400945311cb253001412002ea041a08e30394fc601440310920af2192dc4194a03302191cf2290ac0c12000815324eb96a08000aad914034c1c8eb0cb39422e272808b7a4911989c306381502868820b4b95076fc004b14dd48a0411024218051204d902b80d004c36510400ccb123084",
				"miner": "0x4c549990a7ef3fea8784406c1eecc98bf4211fa5",
				"mixHash": "0x543bc0769f7d5df30e7633f4a01552c2cee7baace8a6da37fddaa19e49e81209",
				"nonce": "0xa5d3d0ccc8bb8a29",
				"number": "0x8947a9",
				"parentHash": "0x0b2d720b8d3b6601e4207ef926b0c228735aa1d58301a23d58f9cb51ac2288d8",
				"receiptsRoot": "0x5968afe6026e673df3b9745d925a5648282d2195a46c22771fec48210daf8e23",
				"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
				"size": "0x93f7",
				"stateRoot": "0x4ba0fb3e6f4c1af32a799df667d304bcdb7f8154e6f86831f92f5a354c2baf70",
				"timestamp": "0x5ddb67a0",
				"totalDifficulty": "0x2c10c70159db491d5d8",
				"transactions": [],
				"transactionsRoot": "0x07d44fadb4aca78c81698710211c5399c1408bb3f0aa3a687d091d230fcaddc6",
				"uncles": []
			}"#).unwrap(),
			serde_json::from_str(r#"{
				"difficulty": "0x92c07e50de0b9",
				"extraData": "0x7575706f6f6c2e636e2d3163613037623939",
				"gasLimit": "0x98700d",
				"gasUsed": "0x98254e",
				"hash": "0xb972df738904edb8adff9734eebdcb1d3b58fdfc68a48918720a4a247170f15e",
				"logsBloom": "0x0c0110a00144a0082057622381231d842b8977a98d1029841000a1c21641d91946594605e902a5432000159ad24a0300428d8212bf4d1c81c0f8478402a4a818010011437c07a112080e9a4a14822311a6840436f26585c84cc0d50693c148bf9830cf3e0a08970788a4424824b009080d52372056460dec808041b68ea04050bf116c041f25a3329d281068740ca911c0d4cd7541a1539005521694951c286567942d0024852080268d29850000954188f25151d80e4900002122c01ad53b7396acd34209c24110b81b9278642024603cd45387812b0696d93992829090619cf0b065a201082280812020000430601100cb08a3808204571c0e564d828648fb",
				"miner": "0xd224ca0c819e8e97ba0136b3b95ceff503b79f53",
				"mixHash": "0x0ea8027f96c18f474e9bc74ff71d29aacd3f485d5825be0a8dde529eb82a47ed",
				"nonce": "0x55859dc00728f99a",
				"number": "0x8947aa",
				"parentHash": "0xb80bf91d6f459227a9c617c5d9823ff0b07f1098ea16788676f0b804ecd42f3b",
				"receiptsRoot": "0x3fbd99e253ff45045eec1e0011ac1b45fa0bccd641a356727defee3b166dd3bf",
				"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
				"size": "0x8a17",
				"stateRoot": "0x5dfc6357dda61a7f927292509afacd51453ff158342eb9628ccb419fbe91c638",
				"timestamp": "0x5ddb67a3",
				"totalDifficulty": "0x2c10c7941a5999fb691",
				"transactions": [],
				"transactionsRoot": "0xefebac0e71cc2de04cf2f509bb038a82bbe92a659e010061b49b5387323b5ea6",
				"uncles": []
			}"#).unwrap(),
		)
	}

	fn ropsten_sequential_header() -> (Header, Header) {
		(
			serde_json::from_str(r#"{
				"difficulty": "0xf4009f4b",
				"extraData": "0xd983010906846765746889676f312e31312e3133856c696e7578",
				"gasLimit": "0x7a1200",
				"gasUsed": "0x769975",
				"hash": "0x1dafbf6a9825241ea5dfa7c3a54781c0784428f2ef3b588748521f83209d3caa",
				"logsBloom": "0x0420000400000018000400400402044000088100000088000000010000040800202000002000a0000000000200004000800100000200000000000020003400000000000004002000000000080102004400000000010400008001000000000020000000009200100000000000004408040100000010000010022002130002000600048200000000000000004000002410000008000000000008021800100000000704010008080000200081000000004002000000009010c000010082000040400104020200000000040180000000000a803000000000002212000000000061000010000001010000400020000000002000020008008100040000005200000000",
				"miner": "0x4ccfb3039b78d3938588157564c9ad559bafab94",
				"mixHash": "0xc4b28f4b671b2e675634f596840d3115ce3df0ab38b6608a69371da16a3455aa",
				"nonce": "0x7afbefa403b138fa",
				"number": "0x69226b",
				"parentHash": "0x8a18726cacb45b078bfe6491510cfa2dd578a70be2a217f416253cf3e94adbd2",
				"receiptsRoot": "0x9c9eb20b6f9176864630f84aa11f33969a355efa85b2eb1e386a5b1ea3599089",
				"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
				"size": "0x83f4",
				"stateRoot": "0xde1df18f7da776a86119d17373d252d3591b5a4270e14113701d27c852d25313",
				"timestamp": "0x5de5246c",
				"totalDifficulty": "0x66728874bd82ce",
				"transactions": [],
				"transactionsRoot": "0xe3ab46e9eeb65fea6b0b1ffd07587f3ee7741b66f16a0b63a3b0c01900387833",
				"uncles": []
			}"#).unwrap(),
			serde_json::from_str(r#"{
				"difficulty": "0xf3c49f25",
				"extraData": "0xd983010906846765746889676f312e31312e3133856c696e7578",
				"gasLimit": "0x7a1200",
				"gasUsed": "0x702566",
				"hash": "0x21fe7ebfb3639254a0867995f3d490e186576b42aeea8c60f8e3360c256f7974",
				"logsBloom": "0x8211a0050000250240000000010200402002800012890000600004000208230500042a400000000001000040c00080001001100000002000001004004012000010006200800900a03002510844010014a0000000010408600444200000200080000410001a00140004008000150108108000003010126a0110828010810000000200010000800011001000062040221422249420c1040a940002000000400840080000810000800000400000010408000002001018002200020040000000a00000804002800008000000000080800020082002000000002810054100500020000288240880290000510020000204c0304000000000000820088c800200000000",
				"miner": "0x4ccfb3039b78d3938588157564c9ad559bafab94",
				"mixHash": "0x5a85e328a8bb041a386ffb25db029b7f0df4665a8a55b331b30a576761404fa6",
				"nonce": "0x650ea83006bb108d",
				"number": "0x69226c",
				"parentHash": "0x1dafbf6a9825241ea5dfa7c3a54781c0784428f2ef3b588748521f83209d3caa",
				"receiptsRoot": "0xb2f020ce6615246a711bed61f2f485833943adb734d8e1cddd93d7ae8a641451",
				"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
				"size": "0x75e4",
				"stateRoot": "0xee6ad25ad26e79004f15b8d423a9952859983ad740924fd13165d6e20953ff3e",
				"timestamp": "0x5de52488",
				"totalDifficulty": "0x667289688221f3",
				"transactions": [],
				"transactionsRoot": "0xcd2672df775af7bcb2b93a478666d500dee3d78e6970c71071dc79642db24719",
				"uncles": []
			}"#).unwrap(),
		)
	}

	#[test]
	fn test_mainnet_header_bare_hash() {
		let header = serde_json::from_str::<Header>(r#"{
			"difficulty": "0x92ac28cbc4930",
			"extraData": "0x5050594520686976656f6e2d6574682d6672",
			"gasLimit": "0x989631",
			"gasUsed": "0x986d77",
			"hash": "0xb80bf91d6f459227a9c617c5d9823ff0b07f1098ea16788676f0b804ecd42f3b",
			"logsBloom": "0x0c7b091bc8ec02401ad12491004e3014e8806390031950181c118580ac61c9a00409022c418162002710a991108a11ca5383d4921d1da46346edc3eb8068481118b005c0b20700414c13916c54011a0922904aa6e255406a33494c84a1426410541819070e04852042410b30030d4c88a5103082284c7d9bd42090322ae883e004224e18db4d858a0805d043e44a855400945311cb253001412002ea041a08e30394fc601440310920af2192dc4194a03302191cf2290ac0c12000815324eb96a08000aad914034c1c8eb0cb39422e272808b7a4911989c306381502868820b4b95076fc004b14dd48a0411024218051204d902b80d004c36510400ccb123084",
			"miner": "0x4c549990a7ef3fea8784406c1eecc98bf4211fa5",
			"mixHash": "0x543bc0769f7d5df30e7633f4a01552c2cee7baace8a6da37fddaa19e49e81209",
			"nonce": "0xa5d3d0ccc8bb8a29",
			"number": "0x8947a9",
			"parentHash": "0x0b2d720b8d3b6601e4207ef926b0c228735aa1d58301a23d58f9cb51ac2288d8",
			"receiptsRoot": "0x5968afe6026e673df3b9745d925a5648282d2195a46c22771fec48210daf8e23",
			"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
			"size": "0x93f7",
			"stateRoot": "0x4ba0fb3e6f4c1af32a799df667d304bcdb7f8154e6f86831f92f5a354c2baf70",
			"timestamp": "0x5ddb67a0",
			"totalDifficulty": "0x2c10c70159db491d5d8",
			"transactions": [],
			"transactionsRoot": "0x07d44fadb4aca78c81698710211c5399c1408bb3f0aa3a687d091d230fcaddc6",
			"uncles": []
		}"#).unwrap();

		assert_eq!(
			header.hash(),
			array_bytes::hex_into_unchecked(
				"0xb80bf91d6f459227a9c617c5d9823ff0b07f1098ea16788676f0b804ecd42f3b",
			)
		);

		let partial_header_hash = header.bare_hash();
		assert_eq!(
			partial_header_hash,
			array_bytes::hex_into_unchecked(
				"0x3c2e6623b1de8862a927eeeef2b6b25dea6e1d9dad88dca3c239be3959dc384a",
			)
		);
	}

	#[test]
	fn test_ropsten_header_bare_hash() {
		let header = serde_json::from_str::<Header>(r#"{
			"difficulty": "0x6648e9e",
			"extraData": "0xd783010503846765746887676f312e372e33856c696e7578",
			"gasLimit": "0x47d629",
			"gasUsed": "0x182a8",
			"hash": "0xa83130084c3570d9e0432bbfd656b0fe6088d8837967ef552974de5e8dc1fad5",
			"logsBloom": "0x00000100000000100000000000000000000000000000000000000000000000000000008000000000000000000000000004000000000000000000000000000000000000000000000400400000000000000000000000000000000000000010000000000000000000000000000000000000200000000000010000000000000000000000000000000000000000000008000000000000000000000000800000000000000000000000000000000000000000000200000000000000000000000000000000000040000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000002000000000000000000000",
			"miner": "0x1ad857f27200aec56ebb68283f91e6ac1086ad62",
			"mixHash": "0x341e3bcf01c921963933253e0cf937020db69206f633e31e0d1c959cdd1188f5",
			"nonce": "0x475ddd90b151f305",
			"number": "0x11170",
			"parentHash": "0xe7a8c03a03f7c055599def00f21686d3b9179d272c8110162f012c191d303dad",
			"receiptsRoot": "0xfbbc5695aac7a42699da58878f0a8bb8c096ed95a9b087989c0903114650ca70",
			"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
			"size": "0x35d",
			"stateRoot": "0x76565e67622936b6b9eac50f3a9ad940270f1c6d1d9f203fc6af4e0eb67b20fa",
			"timestamp": "0x583f2778",
			"totalDifficulty": "0x69708a12010",
			"transactions": [],
			"transactionsRoot": "0x35ecd6e29d0b8d161bd7863cfa3198e979b451fa637834b96b0da3d8d5d081cf",
			"uncles": []
		}"#).unwrap();

		assert_eq!(
			header.bare_hash(),
			array_bytes::hex_into_unchecked(
				"0xbb698ea6e304a7a88a6cd8238f0e766b4f7bf70dc0869bd2e4a76a8e93fffc80",
			)
		);
	}

	#[test]
	fn can_do_proof_of_work_verification_fail() {
		let mut header: Header = Header::default();

		header.set_seal(vec![
			rlp::encode(&H256::zero()).to_vec(),
			rlp::encode(&H64::zero()).to_vec(),
		]);
		header.set_difficulty(
			U256::from_str("ffffffffffffffffffffffffffffffffffffffffffffaaaaaaaaaaaaaaaaaaaa")
				.unwrap(),
		);

		let ethash_params = EthashPartial::expanse();
		let verify_result = ethash_params.verify_block_basic(&header);

		match verify_result {
			Err(Error::Block(BlockError::InvalidProofOfWork(_))) => {}
			_ => panic!("Expected `InvalidProofOfWork` but got {:?}", verify_result),
		}
	}

	#[test]
	fn can_verify_basic_difficulty() {
		let header = sequential_header().0;
		let ethash_params = EthashPartial::expanse();

		assert!(ethash_params.verify_block_basic(&header).is_ok());
	}

	#[test]
	fn can_calculate_difficulty_ropsten() {
		let (header1, header2) = ropsten_sequential_header();
		let expected = U256::from_str("f3c49f25").unwrap();
		let ethash_params = EthashPartial::ropsten_testnet();

		//		ethash_params.set_difficulty_bomb_delays(0xc3500, 5000000);

		assert_eq!(
			ethash_params.calculate_difficulty(&header2, &header1),
			expected
		);
	}

	#[test]
	fn can_calculate_difficulty_production() {
		let (header1, header2) = sequential_header();
		let expected = U256::from_str("92c07e50de0b9").unwrap();
		let ethash_params = EthashPartial::production();

		assert_eq!(
			ethash_params.calculate_difficulty(&header2, &header1),
			expected
		);
	}

	#[test]
	fn can_verify_basic_difficulty_production() {
		let header = sequential_header().0;
		let ethash_params = EthashPartial::production();

		assert!(ethash_params.verify_block_basic(&header).is_ok());
	}

	#[test]
	fn re_compute_hash_for_eip1559_should_work() {
		let header = serde_json::from_str::<Header>(r#"{
			"baseFeePerGas": "0x536106b",
			"difficulty": "0x1e074a90",
			"extraData": "0xd883010a05846765746888676f312e31362e35856c696e7578",
			"gasLimit": "0x7a1200",
			"gasUsed": "0x0",
			"hash": "0x326c0a30d77b78d91595a4b68ace0f1c0d08d9cf80f98ab9abec5eb12adbd372",
			"logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
			"miner": "0x9ffed2297c7b81293413550db675073ab46980b2",
			"mixHash": "0x6fef2c5eded7b7d98dbf81bc40be1b5595eca162e2e98b2f8ae28a96c8bde1b1",
			"nonce": "0x21d771488758293c",
			"number": "0xa30533",
			"parentHash": "0x0f1524bd3bb6e84ec397fa65cc7edb1effb7ef66945523f6f9f22e9ec5867586",
			"receiptsRoot": "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
			"sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
			"size": "0x221",
			"stateRoot": "0x12673e3f6dd58a1adf6d6ffab70374b53862064ba23de736e2f0ca8fa4aa35d5",
			"timestamp": "0x60f91637",
			"totalDifficulty": "0x799b46acb904b7",
			"transactions": [],
			"transactionsRoot": "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
			"uncles": []
		}"#).unwrap();

		assert_eq!(header.hash.unwrap(), header.re_compute_hash());
	}
}
