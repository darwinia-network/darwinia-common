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
	fn on_messages_delivered(lane: &LaneId, delivered_messages: &DeliveredMessages) -> Weight {
		for (i, nonce) in (delivered_messages.begin..=delivered_messages.end).enumerate() {
			if let Some(result) = delivered_messages.dispatch_results.get(i) {
				if let Some(message_sender) = Self::get_message_sender(*lane, nonce) {
					if let Ok(call_data) = Self::make_call_data(*lane, nonce, *result) {

						// Run solidity callback
						if let Err(e) = darwinia_ethereum::Pallet::<T2>::internal_transact(message_sender, call_data) {
							log::error!("Execute 'internal_transact' failed for messages delivered, {:?}", e.error);
						}

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

	fn get_message_sender(lane: LaneId, nonce: MessageNonce) -> Option<H160> {
		if let Some(data) = Pallet::<T, I>::outbound_message_data(lane, nonce) {
			if let Ok(payload) = MessagePayload::<T, I>::decode(&mut &data.payload[..]) {
				// TODO: SourceRoot, TargetAccount?
				if let CallOrigin::SourceAccount(account_id) = payload.origin {
					return Some(H160::from_slice(&account_id.encode()[11..31]));
				}
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
}
