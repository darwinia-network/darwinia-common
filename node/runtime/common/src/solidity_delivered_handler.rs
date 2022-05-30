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

use sp_std::marker::PhantomData;
use sp_std::vec::Vec;
use sp_std::vec;
use sp_std::borrow::ToOwned;
// --- darwinia-network ---
use pallet_bridge_messages::{Config as PalletBridgeMessagesConfig, Pallet};
use bp_message_dispatch::{CallOrigin, Weight};
use darwinia_ethereum::{InternalTransactHandler, Config as DarwiniaEthereumConfig };
// --- paritytech ---
use codec::{Encode, Decode};
use bp_messages::{
	source_chain::OnDeliveryConfirmed,
	DeliveredMessages, LaneId, MessageNonce,
};
use ethereum_types::H160;
use ethabi::{
	param_type::ParamType, token::Token, Function, Param, Result as AbiResult,
	StateMutability,
};
use frame_support::traits::Get;


type AccountIdOfSourceChain<T> = <T as frame_system::Config>::AccountId;
type AccountIdOfTargetChain<T, I> = <T as PalletBridgeMessagesConfig<I>>::InboundRelayer;
type TargetChainSignature = sp_runtime::MultiSignature;
type Call = Vec<u8>;

type MessagePayload<T, I> = bp_message_dispatch::MessagePayload<
	AccountIdOfSourceChain<T>,
	AccountIdOfTargetChain<T, I>,
	TargetChainSignature,
	Call,
>;

pub struct SolidityDeliveredHandler<T, I, T2>(PhantomData<(T, I, T2)>);

impl<T: PalletBridgeMessagesConfig<I>, I: 'static, T2: DarwiniaEthereumConfig> OnDeliveryConfirmed for SolidityDeliveredHandler<T, I, T2> {
	fn on_messages_delivered(lane: &LaneId, messages: &DeliveredMessages) -> Weight {

		for nonce in messages.begin..=messages.end {
			let result = messages.message_dispatch_result(nonce);
			if let Some(message_sender) = Self::get_message_sender(*lane, nonce) {
				if let Ok(call_data) = Self::make_call_data(*lane, nonce, result) {

					// Run solidity callback
					if let Err(e) = darwinia_ethereum::Pallet::<T2>::internal_transact(message_sender, call_data) {
						log::error!("Execute 'internal_transact' failed for messages delivered, {:?}", e.error);
					}

				}
			}

		}

		<T as frame_system::Config>::DbWeight::get().reads_writes(1, 1)
	}
}

impl<T: PalletBridgeMessagesConfig<I>, I: 'static, T2: DarwiniaEthereumConfig> SolidityDeliveredHandler<T, I, T2> {
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

	fn 	get_message_sender(lane: LaneId, nonce: MessageNonce) -> Option<H160> {
		if let Some(data) = Pallet::<T, I>::outbound_message_data(lane, nonce) {
			return Self::get_origin_from_message_payload_data(data.payload);
		}

		return None;
	}

	pub fn get_origin_from_message_payload_data(payload_data: Vec<u8>) -> Option<H160> {
		if let Ok(payload) = MessagePayload::<T, I>::decode(&mut &payload_data[..]) {
			// TODO: SourceRoot?
			let account_id = match payload.origin {
				CallOrigin::SourceRoot => None,
				CallOrigin::TargetAccount(account_id, _, _) => Some(account_id),
				CallOrigin::SourceAccount(account_id) => Some(account_id),
			};
			if let Some(account_id) = account_id {
				// TODO: use derive_ethereum_address instead
				return Some(H160::from_slice(&account_id.encode()[11..31]));
			}
		}
		return None;
	}
}

#[cfg(test)]
mod tests {
	use core::str::FromStr;
	use codec::Encode;

	use super::*;
	use sp_runtime::AccountId32;

	#[test]
	fn send_account_id_to_h160_works() {
		let account_id = AccountId32::from_str("0x64766d3a000000000000006be02d1d3665660d22ff9624b7be0551ee1ac91bd2").unwrap();

		let address = H160::from_slice(&account_id.encode()[11..31]);
		assert_eq!(address, H160::from_str("0x6be02d1d3665660d22ff9624b7be0551ee1ac91b").unwrap());
	}

	#[test]
	fn decode_message_payload_works() {
		type MessagePayload = bp_message_dispatch::MessagePayload<
			AccountId32,
			(),
			(),
			Vec<u8>,
		>;
		let hex = "b06d000080d3309e000000000264766d3a000000000000002b9b61ce0c92db05304f6ba433f7c29a159aefb7e1005d0114026d6f646c64612f6272696e670000000000000000a08601000000000000000000000000000000000000000000000000000000000080d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d00d0ed902e0000000000000000000000";
		let bytes  = array_bytes::hex2array_unchecked::<&str, 151>(hex).to_vec();

		let payload = MessagePayload::decode(&mut &bytes[..]).unwrap();
		if let CallOrigin::SourceAccount(account_id) = payload.origin {
			let str: &[u8] = &account_id.encode()[11..31];
			let h160 = H160::from_slice(str);
			assert_eq!(h160, H160::from_str("0x2b9b61ce0c92db05304f6ba433f7c29a159aefb7").unwrap());
		}
	}
}
