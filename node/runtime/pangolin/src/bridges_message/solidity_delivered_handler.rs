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
use codec::{Decode, Encode};
use ethabi::{
	param_type::ParamType, token::Token, Function, Param, Result as AbiResult, StateMutability,
};
use ethereum_types::H160;
// --- darwinia-network ---
use crate::bm_pangoro;
use bm_pangoro::ToPangoroMessagePayload as MessagePayload;
use bp_message_dispatch::{CallOrigin, Weight};
use bp_messages::{
	source_chain::OnDeliveryConfirmed, target_chain::MessageDispatch, DeliveredMessages, LaneId,
	MessageNonce,
};
use bridge_runtime_common::messages::{source::FromThisChainMessagePayload, MessageBridge};
use darwinia_ethereum::InternalTransactHandler;
use darwinia_support::evm::DeriveEthereumAddress;
use pallet_bridge_messages::Pallet;
// --- paritytech ---
use frame_support::traits::Get;
use sp_std::{borrow::ToOwned, marker::PhantomData, vec, vec::Vec};

// You might want to know the dispatch result or do something else based on the cross-china message
// state in the bridged chain from smart contract side, this would be helpful in this case.
// Adding  a specific interface in your contract side, the handler will call it once the message has
// been delivered in the bridges chain.
pub struct SmartContractDeliveredHandler<Runtime, MessagesPalletInstance, BridgeConfig>(
	PhantomData<(Runtime, MessagesPalletInstance, BridgeConfig)>,
);

impl<Runtime, MessagesPalletInstance, BridgeConfig> OnDeliveryConfirmed
	for SmartContractDeliveredHandler<Runtime, MessagesPalletInstance, BridgeConfig>
where
	Runtime: pallet_bridge_messages::Config<MessagesPalletInstance> + darwinia_ethereum::Config,
	MessagesPalletInstance: 'static,
	BridgeConfig: MessageBridge,
{
	fn on_messages_delivered(lane: &LaneId, messages: &DeliveredMessages) -> Weight {
		let mut writes = 0;
		let mut reads = 0;
		for nonce in messages.begin..=messages.end {
			let result = messages.message_dispatch_result(nonce);
			// TODO: Is it possible to check if the `onMessageDelivered` exist in the contract side?
			// Extraace the message sender from payload
			if let Some(message_sender) = Self::extract_message_sender(*lane, nonce) {
				if let Ok(call_data) = make_call_data(*lane, nonce, result) {
					// Run solidity callback
					if let Err(e) = darwinia_ethereum::Pallet::<Runtime>::internal_transact(
						message_sender,
						call_data,
					) {
						log::error!(
							"Execute 'internal_transact' failed for messages delivered, {:?}",
							e.error
						);
					}
				}
			}
		}

		<Runtime as frame_system::Config>::DbWeight::get().reads_writes(reads, writes)
	}
}

impl<Runtime, MessagesPalletInstance, BridgeConfig>
	SmartContractDeliveredHandler<Runtime, MessagesPalletInstance, BridgeConfig>
where
	Runtime: pallet_bridge_messages::Config<MessagesPalletInstance> + darwinia_ethereum::Config,
	MessagesPalletInstance: 'static,
	BridgeConfig: MessageBridge,
{
	fn extract_message_sender(lane: LaneId, nonce: MessageNonce) -> Option<H160> {
		if let Some(data) =
			Pallet::<Runtime, MessagesPalletInstance>::outbound_message_data(lane, nonce)
		{
			let decoded_payload =
				FromThisChainMessagePayload::<BridgeConfig>::decode(&mut &data.payload[..]).ok()?;
			let origin = match decoded_payload.origin {
				CallOrigin::SourceRoot => None,
				CallOrigin::TargetAccount(account_id, _, _) => Some(account_id),
				CallOrigin::SourceAccount(account_id) => Some(account_id),
			};

			if let Some(account_id) = origin {
				let derive_eth_address = account_id.encode().as_slice().derive_ethereum_address();
				return Some(derive_eth_address);
			}
		}

		None
	}
}

fn make_call_data(lane: LaneId, nonce: MessageNonce, result: bool) -> AbiResult<Vec<u8>> {
	#[allow(deprecated)]
	let func = Function {
		name: "onMessageDelivered".into(),
		inputs: vec![
			Param {
				name: "lane".to_owned(),
				kind: ParamType::FixedBytes(4),
				internal_type: Some("bytes4".into()),
			},
			Param {
				name: "nonce".to_owned(),
				kind: ParamType::Uint(64),
				internal_type: Some("uint64".into()),
			},
			Param {
				name: "result".to_owned(),
				kind: ParamType::Bool,
				internal_type: Some("bool".into()),
			},
		],
		outputs: vec![],
		constant: false,
		state_mutability: StateMutability::NonPayable,
	};

	func.encode_input(&[
		Token::FixedBytes(lane.to_vec()),
		Token::Uint(nonce.into()),
		Token::Bool(result),
	])
}

#[cfg(test)]
mod tests {
	use codec::Encode;
	use core::str::FromStr;

	use super::*;
	use sp_runtime::AccountId32;

	#[test]
	fn send_account_id_to_h160_works() {
		let account_id = AccountId32::from_str(
			"0x64766d3a000000000000006be02d1d3665660d22ff9624b7be0551ee1ac91bd2",
		)
		.unwrap();

		let address = H160::from_slice(&account_id.encode()[11..31]);
		assert_eq!(address, H160::from_str("0x6be02d1d3665660d22ff9624b7be0551ee1ac91b").unwrap());
	}

	#[test]
	fn decode_message_payload_works() {
		type MessagePayload = bp_message_dispatch::MessagePayload<AccountId32, (), (), Vec<u8>>;
		let hex = "b06d000080d3309e000000000264766d3a000000000000002b9b61ce0c92db05304f6ba433f7c29a159aefb7e1005d0114026d6f646c64612f6272696e670000000000000000a08601000000000000000000000000000000000000000000000000000000000080d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d00d0ed902e0000000000000000000000";
		let bytes = array_bytes::hex2array_unchecked::<&str, 151>(hex).to_vec();

		let payload = MessagePayload::decode(&mut &bytes[..]).unwrap();
		if let CallOrigin::SourceAccount(account_id) = payload.origin {
			let str: &[u8] = &account_id.encode()[11..31];
			let h160 = H160::from_slice(str);
			assert_eq!(h160, H160::from_str("0x2b9b61ce0c92db05304f6ba433f7c29a159aefb7").unwrap());
		}
	}

	#[test]
	fn call_data_is_right() {
		let call_data = make_call_data([1u8; 4], 12, true).unwrap();
		let call_data = array_bytes::bytes2hex("0x", call_data);

		// expected result is from remix
		let expected = "0x871f7c500101010100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000000000001";
		assert_eq!(call_data, expected);
	}
}
