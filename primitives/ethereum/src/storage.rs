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

// --- alloc ---
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
// --- crates.io ---
#[cfg(any(feature = "full-codec", test))]
use codec::{Decode, Encode};
#[cfg(any(feature = "full-rlp", test))]
use rlp::{Decodable, DecoderError, Rlp};
use sp_debug_derive::RuntimeDebug;
// --- darwinia-network ---
#[cfg(any(feature = "full-rlp", test))]
use crate::error::*;
use crate::{H160, H256, U256};
#[cfg(any(feature = "full-rlp", test))]
use keccak_hash::keccak_256;
#[cfg(any(feature = "full-rlp", test))]
use merkle_patricia_trie::{trie::Trie, MerklePatriciaTrie, Proof};

#[cfg_attr(any(feature = "full-codec", test), derive(Encode, Decode))]
#[derive(Clone, PartialEq, Eq, RuntimeDebug)]
pub struct Account {
	pub nonce: u64,
	pub balance: U256,
	pub root: H256,
	pub codehash: Vec<u8>,
}

impl Account {
	#[cfg(any(feature = "full-rlp", test))]
	pub fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
		if rlp.item_count()? != 4 {
			return Err(DecoderError::RlpIncorrectListLen);
		}

		Ok(Self {
			nonce: rlp.val_at(0)?,
			balance: rlp.val_at(1)?,
			root: rlp.val_at(2)?,
			codehash: rlp.val_at(3)?,
		})
	}

	#[cfg(any(feature = "full-rlp", test))]
	pub fn verify_account_proof(
		state_root: &H256,
		account_hash: &H256,
		account_proof: Proof,
	) -> Result<Self, Error> {
		let value = MerklePatriciaTrie::verify_proof(
			state_root.0.to_vec(),
			&account_hash.0.to_vec(),
			account_proof,
		)?
		.ok_or(ProofError::TrieKeyNotExist)?;
		let rlp = Rlp::new(&value);
		let account = Self::decode(&rlp).map_err(RlpError::from)?;
		Ok(account)
	}
}

#[cfg_attr(any(feature = "full-codec", test), derive(Encode, Decode))]
#[derive(Clone, PartialEq, Eq, RuntimeDebug)]
pub struct EthereumStorageProof {
	pub address: H160,
	pub key: H256,
	pub account_proof: Vec<Vec<u8>>,
	pub storage_proof: Vec<Vec<u8>>,
}

impl EthereumStorageProof {
	pub fn new(
		address: H160,
		key: H256,
		account_proof: Vec<Vec<u8>>,
		storage_proof: Vec<Vec<u8>>,
	) -> Self {
		Self {
			address,
			key,
			account_proof,
			storage_proof,
		}
	}
}

#[cfg(any(feature = "full-rlp", test))]
pub struct EthereumStorage<T: Decodable>(pub T);

#[cfg(any(feature = "full-rlp", test))]
impl<T: Decodable> EthereumStorage<T> {
	#[cfg(any(feature = "full-rlp", test))]
	pub fn verify_storage_proof(
		state_root: H256,
		proof: &EthereumStorageProof,
	) -> Result<Self, Error> {
		let mut address_hash = [0u8; 32];
		keccak_256(&proof.address.0, &mut address_hash);
		let account = Account::verify_account_proof(
			&state_root,
			&address_hash.into(),
			proof.account_proof.clone().into(),
		)?;
		let mut key_hash = [0u8; 32];
		keccak_256(&proof.key.0, &mut key_hash);
		let value = MerklePatriciaTrie::verify_proof(
			account.root.0.to_vec(),
			&key_hash,
			proof.storage_proof.clone().into(),
		)?
		.ok_or(ProofError::TrieKeyNotExist)?;
		let rlp = Rlp::new(&value);
		let storage = T::decode(&rlp).map_err(RlpError::from)?;
		Ok(Self(storage))
	}
}
