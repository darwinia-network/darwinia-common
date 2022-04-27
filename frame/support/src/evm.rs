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

/// A trait for converting from `AccountId` to H160.
pub trait IntoH160 {
	fn into_h160(&self) -> H160;
}
impl IntoH160 for PalletId {
	fn into_h160(&self) -> H160 {
		let account_id: AccountId32 = self.into_account();
		let bytes: &[u8] = account_id.as_ref();
		H160::from_slice(&bytes[0..20])
	}
}
impl IntoH160 for &[u8] {
	fn into_h160(&self) -> H160 {
		let mut address: [u8; 20] = Default::default();
		let size = sp_std::cmp::min(self.len(), 20);
		address[..size].copy_from_slice(&self[..size]);
		H160::from_slice(&address[0..20])
	}
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

		let checksum: u8 = raw_account[1..31].iter().fold(raw_account[0], |sum, &byte| sum ^ byte);

		raw_account[31] = checksum;

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
		}
		Transaction::EIP2930(t) => {
			sig[0..32].copy_from_slice(&t.r[..]);
			sig[32..64].copy_from_slice(&t.s[..]);
			sig[64] = t.odd_y_parity as u8;
			msg.copy_from_slice(&ethereum::EIP2930TransactionMessage::from(t.clone()).hash()[..]);
		}
		Transaction::EIP1559(t) => {
			sig[0..32].copy_from_slice(&t.r[..]);
			sig[32..64].copy_from_slice(&t.s[..]);
			sig[64] = t.odd_y_parity as u8;
			msg.copy_from_slice(&ethereum::EIP1559TransactionMessage::from(t.clone()).hash()[..]);
		}
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
		(&b"root"[..]).into_h160()
	);
	assert_eq!(
		(&b"longbytes..longbytes..longbytes..longbytes"[..]).into_h160(),
		(&b"longbytes..longbytes"[..]).into_h160()
	);
}
