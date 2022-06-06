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

// --- darwinia-network ---

use super::*;
use crate::{mock::*, tests, Config, InternalTransactHandler, Pallet, Weight};
use bp_message_dispatch::{CallOrigin, MessageDispatch, MessagePayload, SpecVersion};
use bp_runtime::messages::DispatchFeePayment;
use codec::{Decode, Encode, MaxEncodedLen};
use frame_system::{EventRecord, Phase};
use sp_runtime::AccountId32;

const TEST_SPEC_VERSION: SpecVersion = 0;
const TEST_WEIGHT: Weight = 1_000_000_000;

fn prepare_message(
	origin: CallOrigin<AccountId32, TestAccountPublic, TestSignature>,
	call: Call,
) -> <pallet_bridge_dispatch::Pallet<Test> as MessageDispatch<
	<Test as frame_system::Config>::AccountId,
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
	<Test as frame_system::Config>::AccountId,
	<Test as pallet_bridge_dispatch::Config>::BridgeMessageId,
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

		assert_eq!(
			System::events(),
			vec![EventRecord {
				phase: Phase::Initialization,
				event: Event::Dispatch(pallet_bridge_dispatch::Event::<Test>::MessageDispatched(
					SOURCE_CHAIN_ID,
					id,
					Ok(())
				)),
				topics: vec![],
			}],
		);
	});
}
