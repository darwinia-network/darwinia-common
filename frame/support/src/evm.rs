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

// --- crates.io ---
use ethereum::TransactionMessage;
use sha3::{Digest, Keccak256};
// --- darwinia-network ---
use ethereum_primitives::{H160, H256};
// --- paritytech ---
use codec::{Decode, Encode};
use frame_support::PalletId;

pub const POW_9: u32 = 1_000_000_000;
pub const INTERNAL_CALLER: H160 = H160::zero();
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
