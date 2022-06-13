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
use crate::{tests::legacy::*, Weight};
use bp_message_dispatch::{CallOrigin, MessageDispatch, MessagePayload, SpecVersion};
use bp_runtime::messages::DispatchFeePayment;
use darwinia_evm::AccountBasic;
use darwinia_support::evm::{decimal_convert, DeriveSubstrateAddress};
// --- paritytech ---
use frame_support::traits::OriginTrait;
use sp_runtime::AccountId32;

const TEST_SPEC_VERSION: SpecVersion = 0;
const TEST_WEIGHT: Weight = 1_000_000_000_000;

fn prepare_message(
	origin: CallOrigin<AccountId32, TestAccountPublic, TestSignature>,
	call: Call,
) -> <pallet_bridge_dispatch::Pallet<Test> as MessageDispatch<
	<Test as frame_system::Config>::Origin,
	<Test as pallet_bridge_dispatch::Config>::BridgeMessageId,
	<Test as pallet_bridge_dispatch::Config>::Call,
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
	<Test as frame_system::Config>::Origin,
	<Test as pallet_bridge_dispatch::Config>::BridgeMessageId,
	<Test as pallet_bridge_dispatch::Config>::Call,
>>::Message {
	let origin = CallOrigin::SourceAccount(AccountId32::new([1; 32]));
	prepare_message(origin, call)
}

#[test]
fn test_dispatch_basic_system_call_works() {
	let (_, mut ext) = new_test_ext(1);

	ext.execute_with(|| {
		let id = [0; 4];
		let call = Call::System(frame_system::Call::remark { remark: vec![] });
		let mut message = prepare_source_message(call);
		message.dispatch_fee_payment = DispatchFeePayment::AtTargetChain;

		System::set_block_number(1);
		let result =
			Dispatch::dispatch(SOURCE_CHAIN_ID, TARGET_CHAIN_ID, id, Ok(message), |_, _| Ok(()));
		assert!(result.dispatch_fee_paid_during_dispatch);
		assert!(result.dispatch_result);

		System::assert_has_event(Event::Dispatch(
			pallet_bridge_dispatch::Event::MessageDispatched(SOURCE_CHAIN_ID, id, Ok(())),
		));
	});
}

#[test]
fn test_dispatch_ethereum_transact_works() {
	let (pairs, mut ext) = new_test_ext(2);
	let alice = &pairs[0];
	let relayer_account = &pairs[1];

	ext.execute_with(|| {
		let id = [0; 4];
		let t = legacy_erc20_creation_transaction(alice);
		let call = TestRuntimeCall::Ethereum(EthereumTransactCall::transact { transaction: t });

		let mut message = prepare_source_message(call);
		message.dispatch_fee_payment = DispatchFeePayment::AtTargetChain;

		System::set_block_number(1);
		let result =
			Dispatch::dispatch(
                SOURCE_CHAIN_ID,
                TARGET_CHAIN_ID,
                id,
                Ok(message),
                |origin, call| match call {
                    // Filter Ethereum transact call
                    Call::Ethereum(crate::Call::transact { transaction: tx }) =>
                        match origin.caller() {
                            OriginCaller::Ethereum(RawOrigin::EthereumTransaction(id)) => match tx {
                                // Only support legacy transaction now
                                Transaction::Legacy(t) => {
                                    let fee = t.gas_limit.saturating_mul(t.gas_limit);
                                    let total_payment = fee.saturating_add(t.value);
                                    // Ensure the relayer has enough balance
                                    let derived_substrate_address = <Test as darwinia_evm::Config>::IntoAccountId::derive_substrate_address(*id);
                                    if <Test as darwinia_evm::Config>::RingAccountBasic::account_balance(&relayer_account.account_id) >= total_payment {
                                            // Ensure the derived ethereum address has enough balance to pay for the transaction
                                            let _ = <Test as darwinia_evm::Config>::RingAccountBasic::transfer(
                                                &relayer_account.account_id,
                                                &derived_substrate_address,
                                                total_payment
                                            );
                                            return Ok(());
                                        }
                                    Err(())
                                },
                                // Invalid Ethereum transaction type
                                _ => Err(()),
                            },
                            // Invalid call dispatch origin, should return Err.
                            _ => Err(()),
                        },
                    // Do nothing for other calls.
                    _ => Ok(()),
                },
            );
		assert!(result.dispatch_result);
		System::assert_has_event(Event::Dispatch(
			pallet_bridge_dispatch::Event::MessageDispatched(SOURCE_CHAIN_ID, id, Ok(())),
		));
	});
}

#[test]
fn test_dispatch_ethereum_transact_invalid_payment() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];
	ext.execute_with(|| {
		let id = [0; 4];
		let t = legacy_erc20_creation_transaction(alice);
		let call = TestRuntimeCall::Ethereum(EthereumTransactCall::transact { transaction: t });

		let mut message = prepare_source_message(call);
		message.dispatch_fee_payment = DispatchFeePayment::AtTargetChain;

		System::set_block_number(1);
		let result =
			Dispatch::dispatch(SOURCE_CHAIN_ID, TARGET_CHAIN_ID, id, Ok(message), |_, _| Ok(()));

		assert!(!result.dispatch_result);
		System::assert_has_event(Event::Dispatch(
			pallet_bridge_dispatch::Event::MessageCallRejected(SOURCE_CHAIN_ID, id),
		));
	});
}

#[test]
fn test_dispatch_ethereum_transact_invalid_nonce() {
	let (pairs, mut ext) = new_test_ext(1);
	let alice = &pairs[0];
	ext.execute_with(|| {
		let id = [0; 4];
		let mut unsigned_tx = legacy_erc20_creation_unsigned_transaction();
		unsigned_tx.nonce = U256::from(3);
		let t = unsigned_tx.sign(&alice.private_key);

		let call = TestRuntimeCall::Ethereum(EthereumTransactCall::transact { transaction: t });

		let mut message = prepare_source_message(call);
		message.dispatch_fee_payment = DispatchFeePayment::AtTargetChain;
		// Ensure the derive ethereum address has enough balance to cover fee.
		let origin = H160::from_str("0x308f55f1caf780c5f7a73e2b2b88cb61ee5bec9b").unwrap();
		RingAccount::mutate_account_basic_balance(&origin, decimal_convert(1000, None));

		System::set_block_number(1);
		let result =
			Dispatch::dispatch(SOURCE_CHAIN_ID, TARGET_CHAIN_ID, id, Ok(message), |_, _| Ok(()));

		assert!(!result.dispatch_result);
		System::assert_has_event(Event::Dispatch(
			pallet_bridge_dispatch::Event::MessageCallRejected(SOURCE_CHAIN_ID, id),
		));
	});
}
