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
use codec::Encode;
// --- darwinia-network ---
use super::*;
use crate::{
	tests::{eip1559::*, legacy::*},
	Weight,
};
use bp_message_dispatch::{CallOrigin, MessageDispatch, MessagePayload, SpecVersion};
use bp_runtime::messages::DispatchFeePayment;
// --- paritytech ---
use sp_runtime::AccountId32;

const TEST_SPEC_VERSION: SpecVersion = 0;
const TEST_WEIGHT: Weight = 1_000_000_000_000;

fn prepare_message(
	origin: CallOrigin<AccountId32, TestAccountPublic, TestSignature>,
	call: Call,
) -> <pallet_bridge_dispatch::Pallet<Test> as MessageDispatch<
	AccountId32,
	<Test as pallet_bridge_dispatch::Config>::BridgeMessageId,
>>::Message {
	MessagePayload {
		spec_version: TEST_SPEC_VERSION,
		weight: TEST_WEIGHT,
		origin,
		dispatch_fee_payment: DispatchFeePayment::AtSourceChain,
		call: EncodedCall(call.encode()),
	}
}

fn prepare_source_message(
	call: Call,
) -> <pallet_bridge_dispatch::Pallet<Test> as MessageDispatch<
	AccountId32,
	<Test as pallet_bridge_dispatch::Config>::BridgeMessageId,
>>::Message {
	let origin = CallOrigin::SourceAccount(AccountId32::new([1; 32]));
	prepare_message(origin, call)
}

#[test]
fn test_dispatch_basic_system_call_works() {
	let (pairs, mut ext) = new_test_ext(1);
	let relayer_account = &pairs[0];

	ext.execute_with(|| {
		let id = [0; 4];
		let call = Call::System(frame_system::Call::remark { remark: vec![] });
		let message = prepare_source_message(call);

		System::set_block_number(1);
		let result = Dispatch::dispatch(
			SOURCE_CHAIN_ID,
			TARGET_CHAIN_ID,
			&relayer_account.account_id,
			id,
			Ok(message),
			|_, _| Ok(()),
		);
		assert!(!result.dispatch_fee_paid_during_dispatch);
		assert!(result.dispatch_result);

		System::assert_has_event(Event::Dispatch(
			pallet_bridge_dispatch::Event::MessageDispatched(SOURCE_CHAIN_ID, id, Ok(())),
		));
	});
}

#[test]
fn test_dispatch_legacy_ethereum_transaction_works() {
	let (pairs, mut ext) = new_test_ext(2);
	let alice = &pairs[0];
	let relayer_account = &pairs[1];

	ext.execute_with(|| {
		let id = [0; 4];
		let unsigned_tx = legacy_erc20_creation_unsigned_transaction();
		let t = unsigned_tx.sign(&alice.private_key);
		let call =
			TestRuntimeCall::Ethereum(EthereumTransactCall::message_transact { transaction: t });

		let message = prepare_source_message(call);

		System::set_block_number(1);
		let result = Dispatch::dispatch(
			SOURCE_CHAIN_ID,
			TARGET_CHAIN_ID,
			&relayer_account.account_id,
			id,
			Ok(message),
			|_, _| Ok(()),
		);

		assert!(result.dispatch_result);
		System::assert_has_event(Event::Dispatch(
			pallet_bridge_dispatch::Event::MessageDispatched(SOURCE_CHAIN_ID, id, Ok(())),
		));
	});
}
#[test]
fn test_dispatch_only_legacy_ethereum_transaction_works() {
	let (pairs, mut ext) = new_test_ext(2);
	let alice = &pairs[0];
	let relayer_account = &pairs[1];

	ext.execute_with(|| {
		let id = [0; 4];
		let unsigned_tx = eip1559_erc20_creation_unsigned_transaction();
		let t = unsigned_tx.sign(&alice.private_key, None);
		let call =
			TestRuntimeCall::Ethereum(EthereumTransactCall::message_transact { transaction: t });

		let message = prepare_source_message(call);

		System::set_block_number(1);
		let result = Dispatch::dispatch(
			SOURCE_CHAIN_ID,
			TARGET_CHAIN_ID,
			&relayer_account.account_id,
			id,
			Ok(message),
			|_, _| Ok(()),
		);

		assert!(!result.dispatch_result);
		System::assert_has_event(Event::Dispatch(
			pallet_bridge_dispatch::Event::MessageCallValidateFailed(
				SOURCE_CHAIN_ID,
				id,
				TransactionValidityError::Invalid(InvalidTransaction::Custom(1)),
			),
		));
	});
}

#[test]
fn test_dispatch_legacy_ethereum_transaction_weight_mismatch() {
	let (pairs, mut ext) = new_test_ext(2);
	let alice = &pairs[0];
	let relayer_account = &pairs[1];

	ext.execute_with(|| {
		let id = [0; 4];
		let mut unsigned_tx = legacy_erc20_creation_unsigned_transaction();
		// 62500001 * 16000 > 1_000_000_000_000
		unsigned_tx.gas_limit = U256::from(62500001);
		let t = unsigned_tx.sign(&alice.private_key);
		let call =
			TestRuntimeCall::Ethereum(EthereumTransactCall::message_transact { transaction: t });

		let message = prepare_source_message(call);

		System::set_block_number(1);
		let result = Dispatch::dispatch(
			SOURCE_CHAIN_ID,
			TARGET_CHAIN_ID,
			&relayer_account.account_id,
			id,
			Ok(message),
			|_, _| Ok(()),
		);

		assert!(!result.dispatch_result);
		System::assert_has_event(Event::Dispatch(
			pallet_bridge_dispatch::Event::MessageWeightMismatch(
				SOURCE_CHAIN_ID,
				id,
				1000000016000,
				1000000000000,
			),
		));
	});
}
