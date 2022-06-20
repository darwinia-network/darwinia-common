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

// --- crates.io ---
use ethereum::TransactionV2 as Transaction;
use sha3::{Digest, Keccak256};
// --- darwinia-network ---
use ethereum_primitives::{H160, H256, U256};
// --- paritytech ---
use frame_support::PalletId;
use sp_runtime::{traits::AccountIdConversion, AccountId32};
use sp_std::marker::PhantomData;

pub const POW_9: u32 = 1_000_000_000;
/// The default gas limit for the internal transaction
pub const INTERNAL_TX_GAS_LIMIT: u32 = 300_000_000;
/// The action selector used in transfer pre-compile
pub const SELECTOR: usize = 4;
/// The transfer pre-compile address, also as the sender in the when KTON transfer to WKTON.
pub const TRANSFER_ADDR: &'static str = "0x0000000000000000000000000000000000000015";
/// The address prefix for dvm address
const ADDR_PREFIX: &[u8] = b"dvm:";

/// A trait for converting from Substrate account_id to Ethereum address.
pub trait DeriveEthereumAddress {
	fn derive_ethereum_address(&self) -> H160;
}

/// A trait for converting from Ethereum address to Substrate account_id.
pub trait DeriveSubstrateAddress<AccountId> {
	fn derive_substrate_address(address: H160) -> AccountId;
}

pub fn is_derived_from_eth(account_id: impl AsRef<[u8; 32]>) -> bool {
	let account_id: &[u8; 32] = account_id.as_ref();

	let mut account_id_prefix = [0u8; 11];
	account_id_prefix[0..4].copy_from_slice(ADDR_PREFIX);

	// Return true if prefix and checksum valid
	account_id[0..11] == account_id_prefix && account_id[31] == checksum_of(&account_id)
}

impl DeriveEthereumAddress for PalletId {
	fn derive_ethereum_address(&self) -> H160 {
		let account_id: AccountId32 = self.into_account();
		account_id.derive_ethereum_address()
	}
}

// If AccountId32 is derived from an Ethereum address before, this should return the orginal
// Ethereum address, otherwise a new Ethereum address should be generated.
impl DeriveEthereumAddress for AccountId32 {
	fn derive_ethereum_address(&self) -> H160 {
		let bytes: &[u8; 32] = self.as_ref();
		let is_derived_from_eth = is_derived_from_eth(&self);

		if is_derived_from_eth {
			H160::from_slice(&bytes[11..31])
		} else {
			H160::from_slice(&bytes[0..20])
		}
	}
}

fn checksum_of(account_id: &[u8; 32]) -> u8 {
	account_id[1..31].iter().fold(account_id[0], |sum, &byte| sum ^ byte)
}

/// Darwinia network address mapping.
pub struct ConcatConverter<AccountId>(PhantomData<AccountId>);
/// The ConcatConverter used to convert Ethereum address to Substrate account_id
/// The concat rule included three parts:
/// 1. AccountId Prefix: concat("dvm:", "0x00000000000000"), length: 11 bytes
/// 2. EVM address: the original evm address, length: 20 bytes
/// 3. CheckSum:  byte_xor(AccountId Prefix + EVM address), length: 1 bytes
impl<AccountId> DeriveSubstrateAddress<AccountId> for ConcatConverter<AccountId>
where
	AccountId: From<[u8; 32]>,
{
	fn derive_substrate_address(address: H160) -> AccountId {
		let mut raw_account = [0u8; 32];

		raw_account[0..4].copy_from_slice(ADDR_PREFIX);
		raw_account[11..31].copy_from_slice(&address[..]);
		raw_account[31] = checksum_of(&raw_account);

		raw_account.into()
	}
}

pub fn recover_signer(transaction: &Transaction) -> Option<H160> {
	let mut sig = [0u8; 65];
	let mut msg = [0u8; 32];
	match transaction {
		Transaction::Legacy(t) => {
			sig[0..32].copy_from_slice(&t.signature.r()[..]);
			sig[32..64].copy_from_slice(&t.signature.s()[..]);
			sig[64] = t.signature.standard_v();
			msg.copy_from_slice(&ethereum::LegacyTransactionMessage::from(t.clone()).hash()[..]);
		},
		Transaction::EIP2930(t) => {
			sig[0..32].copy_from_slice(&t.r[..]);
			sig[32..64].copy_from_slice(&t.s[..]);
			sig[64] = t.odd_y_parity as u8;
			msg.copy_from_slice(&ethereum::EIP2930TransactionMessage::from(t.clone()).hash()[..]);
		},
		Transaction::EIP1559(t) => {
			sig[0..32].copy_from_slice(&t.r[..]);
			sig[32..64].copy_from_slice(&t.s[..]);
			sig[64] = t.odd_y_parity as u8;
			msg.copy_from_slice(&ethereum::EIP1559TransactionMessage::from(t.clone()).hash()[..]);
		},
	}

	let pubkey = sp_io::crypto::secp256k1_ecdsa_recover(&sig, &msg).ok()?;
	Some(H160::from(H256::from_slice(Keccak256::digest(&pubkey).as_slice())))
}

/// Decimal conversion from RING/KTON to Ethereum decimal format.
pub fn decimal_convert(main_balance: u128, remaining_balance: Option<u128>) -> U256 {
	if let Some(rb) = remaining_balance {
		return U256::from(main_balance)
			.saturating_mul(U256::from(POW_9))
			.saturating_add(U256::from(rb));
	}
	U256::from(main_balance).saturating_mul(U256::from(POW_9))
}

#[cfg(test)]
mod tests {
	use super::*;
	use ethereum_primitives::H160;
	use std::str::FromStr;

	#[test]
	fn const_pow_9_should_work() {
		assert_eq!(U256::from(10).checked_pow(9.into()).unwrap(), POW_9.into())
	}

	#[test]
	fn test_bytes_to_substrate_id() {
		assert_eq!(
			(&b"root"[..]).derive_ethereum_address(),
			H160::from_str("726f6f7400000000000000000000000000000000").unwrap()
		);
		assert_eq!(
			(&b"longbytes..longbytes..longbytes..longbytes"[..]).derive_ethereum_address(),
			(&b"longbytes..longbytes"[..]).derive_ethereum_address()
		);
	}

	#[test]
	fn test_derive_eth_address_from_subaccount_id() {
		let account_id_1 = AccountId32::from_str(
			"0x64766d3a000000000000006be02d1d3665660d22ff9624b7be0551ee1ac91bd2",
		)
		.unwrap();
		let eth_addr1 = account_id_1.derive_ethereum_address();
		assert_eq!(H160::from_str("6Be02d1d3665660d22FF9624b7BE0551ee1Ac91b").unwrap(), eth_addr1);
		assert_eq!(
			ConcatConverter::<AccountId32>::derive_substrate_address(eth_addr1),
			account_id_1
		);

		let account_id_2 = AccountId32::from_str(
			"0x02497755176da60a69586af4c5ea5f5de218eb84011677722646b602eb2d240e",
		)
		.unwrap();
		let eth_addr2 = account_id_2.derive_ethereum_address();
		assert_eq!(H160::from_str("02497755176da60a69586af4c5ea5f5de218eb84").unwrap(), eth_addr2);
		assert_ne!(
			ConcatConverter::<AccountId32>::derive_substrate_address(eth_addr2),
			account_id_2
		);
	}

	#[test]
	fn test_is_derived_from_eth_works() {
		let account_id_1 = AccountId32::from_str(
			"0x64766d3a000000000000006be02d1d3665660d22ff9624b7be0551ee1ac91bd2",
		)
		.unwrap();
		assert!(is_derived_from_eth(account_id_1));
		let account_id_1 = AccountId32::from_str(
			"0x02497755176da60a69586af4c5ea5f5de218eb84011677722646b602eb2d240e",
		)
		.unwrap();
		assert!(!is_derived_from_eth(account_id_1));
	}

	#[test]
	fn test_eth_address_derive() {
		let eth_addr1 = H160::from_str("1234500000000000000000000000000000000000").unwrap();
		let derive_account_id_1 =
			ConcatConverter::<AccountId32>::derive_substrate_address(eth_addr1);

		assert_eq!(derive_account_id_1.derive_ethereum_address(), eth_addr1);
	}
}
