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

use super::*;
use crate::Config;
use array_bytes::hex2bytes_unchecked;
use codec::Decode;
use darwinia_evm::AccountBasic;
use darwinia_support::evm::{decimal_convert, TRANSFER_ADDR};
use sp_runtime::DispatchError;

const WITH_DRAW_INPUT: &str = "723908ee9dc8e509d4b93251bd57f68c09bd9d04471c193fabd8f26c54284a4b";
fn withdraw_unsigned_transaction() -> LegacyUnsignedTransaction {
	LegacyUnsignedTransaction {
		nonce: U256::zero(),
		gas_price: U256::from(1),
		gas_limit: U256::from(0x100000),
		action: ethereum::TransactionAction::Call(H160::from_str(TRANSFER_ADDR).unwrap()),
		value: decimal_convert(30_000_000_000, None),
		input: hex2bytes_unchecked(WITH_DRAW_INPUT),
	}
}

fn withdraw_creation_transaction(account: &AccountInfo) -> Transaction {
	withdraw_unsigned_transaction().sign(&account.private_key)
}

#[test]
fn ring_currency_withdraw_with_enough_balance() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let t = withdraw_creation_transaction(alice);
		assert_ok!(Ethereum::execute(alice.address, &t.into(), None,));

		// Check caller balance
		assert_eq!(
			RingAccount::account_basic(&alice.address).balance,
			// gas fee: 41512
			decimal_convert(70_000_000_000, None).saturating_sub(U256::from(41512))
		);
		// Check the dest balance
		let input_bytes: Vec<u8> = hex2bytes_unchecked(WITH_DRAW_INPUT);
		let dest =
			<Test as frame_system::Config>::AccountId::decode(&mut &input_bytes[..]).unwrap();
		assert_eq!(
			<Test as Config>::RingCurrency::free_balance(dest),
			30_000_000_000
		);
	});
}

#[test]
fn ring_currency_withdraw_not_enough_balance_should_fail() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];

	ext.execute_with(|| {
		let mut transaction = withdraw_unsigned_transaction();
		transaction.value = decimal_convert(120_000_000_000, None);
		let t = transaction.sign(&alice.private_key);
		assert_err!(
			Ethereum::execute(alice.address, &t.into(), None,),
			DispatchError::Module {
				index: 4,
				error: 0,
				message: Some("BalanceLow")
			}
		);

		// Check caller balance
		assert_eq!(
			RingAccount::account_basic(&alice.address).balance,
			decimal_convert(100_000_000_000, None),
		);
		// Check target balance
		let input_bytes: Vec<u8> = hex2bytes_unchecked(WITH_DRAW_INPUT);
		let dest =
			<Test as frame_system::Config>::AccountId::decode(&mut &input_bytes[..]).unwrap();
		assert_eq!(<Test as Config>::RingCurrency::free_balance(&dest), 0);
	});
}
