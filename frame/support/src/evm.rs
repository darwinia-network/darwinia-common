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

// --- crates.io ---
use ethereum::TransactionMessage;
use sha3::{Digest, Keccak256};
// --- darwinia-network ---
use ethereum_primitives::{H160, H256, U256};
// --- paritytech ---
use codec::{Decode, Encode};
use frame_support::PalletId;
use sp_std::vec;
use sp_std::vec::Vec;

pub const POW_9: u32 = 1_000_000_000;
pub const INTERNAL_TX_GAS_LIMIT: u32 = 300_000_000;
pub const SELECTOR: usize = 4;
pub const TRANSFER_ADDR: &'static str = "0x0000000000000000000000000000000000000015";

pub trait IntoDvmAddress {
	fn into_dvm_address(&self) -> H160;
}

#[derive(Clone, Copy, Eq, PartialEq, Encode, Decode)]
pub struct ContractId(pub [u8; 8]);

/// Convert from contract id to dvm address
impl IntoDvmAddress for ContractId {
	fn into_dvm_address(&self) -> H160 {
		let mut bytes = vec![0u8; 12];
		bytes.append(&mut self.0.to_vec());
		H160::from_slice(&bytes)
	}
}

// Convert pallet id to dvm address
impl IntoDvmAddress for PalletId {
	fn into_dvm_address(&self) -> H160 {
		let mut bytes = vec![0u8; 12];
		bytes.append(&mut self.0.to_vec());
		H160::from_slice(&bytes)
	}
}

pub fn recover_signer(transaction: &ethereum::Transaction) -> Option<H160> {
	let mut sig = [0u8; 65];
	let mut msg = [0u8; 32];
	sig[0..32].copy_from_slice(&transaction.signature.r()[..]);
	sig[32..64].copy_from_slice(&transaction.signature.s()[..]);
	sig[64] = transaction.signature.standard_v();
	msg.copy_from_slice(&TransactionMessage::from(transaction.clone()).hash()[..]);

	let pubkey = sp_io::crypto::secp256k1_ecdsa_recover(&sig, &msg).ok()?;
	Some(H160::from(H256::from_slice(
		Keccak256::digest(&pubkey).as_slice(),
	)))
}

/// The dvm transaction used by inner pallets, such as ethereum-issuing.
pub struct DVMTransaction {
	/// source of the transaction
	pub source: H160,
	/// gas price wrapped by Option
	pub gas_price: Option<U256>,
	/// the transaction defined in ethereum lib
	pub tx: ethereum::Transaction,
}

impl DVMTransaction {
	/// the internal transaction usually used by pallets
	/// the source account is specified by pallet dvm account
	/// gas_price is None means no need for gas fee
	/// a default signature which will not be verified
	pub fn new_internal_transaction(
		source: H160,
		nonce: U256,
		target: H160,
		input: Vec<u8>,
	) -> Self {
		let transaction = ethereum::Transaction {
			nonce,
			// Not used, and will be overwritten by None later.
			gas_price: U256::zero(),
			gas_limit: U256::from(INTERNAL_TX_GAS_LIMIT),
			action: ethereum::TransactionAction::Call(target),
			value: U256::zero(),
			input,
			signature: ethereum::TransactionSignature::new(
				// Reference https://github.com/ethereum/EIPs/issues/155
				//
				// But this transaction is sent by darwinia-issuing system from `0x0`
				// So ignore signature checking, simply set `chain_id` to `1`
				1 * 2 + 36,
				H256::from_slice(&[55u8; 32]),
				H256::from_slice(&[55u8; 32]),
			)
			.unwrap(),
		};
		Self {
			source,
			gas_price: None,
			tx: transaction,
		}
	}
}
