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

//! Test utilities

// --- crates.io ---
use ethereum::{TransactionAction, TransactionSignature};
use rlp::RlpStream;
use sha3::{Digest, Keccak256};
// --- darwinia-network ---
use darwinia_ethereum::Transaction;
// --- paritytech ---
use sp_core::{H160, H256, U256};
use sp_runtime::AccountId32;

#[derive(Clone, Debug)]
pub struct AccountInfo {
	pub address: H160,
	pub account_id: AccountId32,
	pub private_key: H256,
}

pub fn address_build(seed: u8) -> AccountInfo {
	let raw_private_key = [seed + 1; 32];
	let secret_key = libsecp256k1::SecretKey::parse_slice(&raw_private_key).unwrap();
	let raw_public_key = &libsecp256k1::PublicKey::from_secret_key(&secret_key).serialize()[1..65];
	let raw_address = {
		let mut s = [0; 20];
		s.copy_from_slice(&Keccak256::digest(raw_public_key)[12..]);
		s
	};
	let raw_account = {
		let mut s = [0; 32];
		s[..20].copy_from_slice(&raw_address);
		s
	};

	AccountInfo {
		private_key: raw_private_key.into(),
		account_id: raw_account.into(),
		address: raw_address.into(),
	}
}

pub struct LegacyUnsignedTransaction {
	pub nonce: U256,
	pub gas_price: U256,
	pub gas_limit: U256,
	pub action: TransactionAction,
	pub value: U256,
	pub input: Vec<u8>,
}

impl LegacyUnsignedTransaction {
	pub fn new(
		nonce: u64,
		gas_price: u64,
		gas_limit: u64,
		action: TransactionAction,
		value: u64,
		input: Vec<u8>,
	) -> Self {
		Self {
			nonce: U256::from(nonce),
			gas_price: U256::from(gas_price),
			gas_limit: U256::from(gas_limit),
			action,
			value: U256::from(value),
			input,
		}
	}

	fn signing_rlp_append(&self, s: &mut RlpStream, chain_id: u64) {
		s.begin_list(9);
		s.append(&self.nonce);
		s.append(&self.gas_price);
		s.append(&self.gas_limit);
		s.append(&self.action);
		s.append(&self.value);
		s.append(&self.input);
		s.append(&chain_id);
		s.append(&0u8);
		s.append(&0u8);
	}

	fn signing_hash(&self, chain_id: u64) -> H256 {
		let mut stream = RlpStream::new();
		self.signing_rlp_append(&mut stream, chain_id);
		H256::from_slice(Keccak256::digest(&stream.out()).as_slice())
	}

	pub fn sign_with_chain_id(&self, key: &H256, chain_id: u64) -> Transaction {
		let hash = self.signing_hash(chain_id);
		let msg = libsecp256k1::Message::parse(hash.as_fixed_bytes());
		let s = libsecp256k1::sign(&msg, &libsecp256k1::SecretKey::parse_slice(&key[..]).unwrap());
		let sig = s.0.serialize();

		let sig = TransactionSignature::new(
			s.1.serialize() as u64 % 2 + chain_id * 2 + 35,
			H256::from_slice(&sig[0..32]),
			H256::from_slice(&sig[32..64]),
		)
		.unwrap();

		Transaction::Legacy(ethereum::LegacyTransaction {
			nonce: self.nonce,
			gas_price: self.gas_price,
			gas_limit: self.gas_limit,
			action: self.action,
			value: self.value,
			input: self.input.clone(),
			signature: sig,
		})
	}
}
