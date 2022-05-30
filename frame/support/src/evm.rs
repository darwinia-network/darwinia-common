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
use sp_std::vec::Vec;

pub const POW_9: u32 = 1_000_000_000;
/// The default gas limit for the internal transaction
pub const INTERNAL_TX_GAS_LIMIT: u32 = 300_000_000;
/// The action selector used in transfer pre-compile
pub const SELECTOR: usize = 4;
/// The transfer pre-compile address, also as the sender in the when KTON transfer to WKTON.
pub const TRANSFER_ADDR: &'static str = "0x0000000000000000000000000000000000000015";

/// A trait for converting from `AccountId` to H160.
pub trait DeriveEtheruemAddress {
	fn derive_ethereum_address(&self) -> H160;
}

// https://github.com/darwinia-network/darwinia-common/issues/1231
impl DeriveEtheruemAddress for [u8; 32] {
	fn derive_ethereum_address(&self) -> H160 {
		if is_derived_substrate_address(self.clone()) {
			H160::from_slice(&self[11..31])
		} else {
			H160::from_slice(&self[0..20])
		}
	}
}

impl DeriveEtheruemAddress for &[u8] {
	fn derive_ethereum_address(&self) -> H160 {
		let mut account_id: [u8; 32] = Default::default();
		let size = sp_std::cmp::min(self.len(), 32);
		account_id[..size].copy_from_slice(&self[..size]);

		account_id.derive_ethereum_address()
	}
}

impl DeriveEtheruemAddress for Vec<u8> {
	fn derive_ethereum_address(&self) -> H160 {
		self.as_slice().derive_ethereum_address()
	}
}

impl DeriveEtheruemAddress for AccountId32 {
	fn derive_ethereum_address(&self) -> H160 {
		let account_id: &[u8; 32] = self.as_ref();
		account_id.derive_ethereum_address()
	}
}

impl DeriveEtheruemAddress for PalletId {
	fn derive_ethereum_address(&self) -> H160 {
		let account_id: AccountId32 = self.into_account();
		account_id.derive_ethereum_address()
	}
}

// "dvm:" + 0x00000000000000 + Ethereum_Address + checksum
pub fn is_derived_substrate_address<T>(account_id: T) -> bool
where
	T: Into<[u8; 32]>,
{
	let account_id: [u8; 32] = account_id.into();

	// check prefix
	let mut account_id_prefix = [0u8; 11];
	account_id_prefix[0..4].copy_from_slice(b"dvm:");
	let correct_prefix = account_id[0..11] == account_id_prefix;

	// check checksum
	let correct_checksum = account_id[31] == checksum_of(&account_id);

	correct_prefix && correct_checksum
}

fn checksum_of(account_id: &[u8; 32]) -> u8 {
	account_id[1..31].iter().fold(account_id[0], |sum, &byte| sum ^ byte)
}

/// A trait for converting from H160 to `AccountId`.
pub trait IntoAccountId<AccountId> {
	fn into_account_id(address: H160) -> AccountId;
}

/// Darwinia network address mapping.
pub struct ConcatConverter<AccountId>(PhantomData<AccountId>);
/// The ConcatConverter used for transfer from evm 20-length to substrate 32-length address
/// The concat rule included three parts:
/// 1. AccountId Prefix: concat("dvm", "0x00000000000000"), length: 11 byetes
/// 2. EVM address: the original evm address, length: 20 bytes
/// 3. CheckSum:  byte_xor(AccountId Prefix + EVM address), length: 1 bytes
impl<AccountId> IntoAccountId<AccountId> for ConcatConverter<AccountId>
where
	AccountId: From<[u8; 32]>,
{
	fn into_account_id(address: H160) -> AccountId {
		let mut raw_account = [0u8; 32];

		raw_account[0..4].copy_from_slice(b"dvm:");
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

#[test]
fn const_pow_9_should_work() {
	assert_eq!(U256::from(10).checked_pow(9.into()).unwrap(), POW_9.into())
}

#[test]
fn test_into_dvm_account() {
	// --- std ---
	use std::str::FromStr;

	assert_eq!(
		H160::from_str("726f6f7400000000000000000000000000000000").unwrap(),
		(&b"root"[..]).derive_ethereum_address()
	);
	assert_eq!(
		(&b"longbytes..longbytes..longbytes..longbytes"[..]).derive_ethereum_address(),
		(&b"longbytes..longbytes"[..]).derive_ethereum_address()
	);
}

#[test]
fn test_derive_ethereum_address_from_dvm_account_id() {
	use std::str::FromStr;

	let account_id =
		AccountId32::from_str("0x64766d3a000000000000006be02d1d3665660d22ff9624b7be0551ee1ac91bd2")
			.unwrap();
	let derived_ethereum_address = account_id.derive_ethereum_address();

	assert_eq!(
		H160::from_str("6Be02d1d3665660d22FF9624b7BE0551ee1Ac91b").unwrap(),
		derived_ethereum_address
	);
}

#[test]
fn test_derive_ethereum_address_from_normal_account_id() {
	use std::str::FromStr;

	let account_id =
		AccountId32::from_str("0x02497755176da60a69586af4c5ea5f5de218eb84011677722646b602eb2d240e")
			.unwrap();
	let derived_ethereum_address = account_id.derive_ethereum_address();

	assert_eq!(
		H160::from_str("02497755176da60a69586af4c5ea5f5de218eb84").unwrap(),
		derived_ethereum_address
	);
}
