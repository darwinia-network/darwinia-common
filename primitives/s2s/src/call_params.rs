// This file is part of Darwinia.
//
// Copyright (C) 2018-2021 Darwinia Network
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

#![cfg_attr(not(feature = "std"), no_std)]

// --- paritytech ---
use sp_core::{H160, U256};
use sp_std::{vec, vec::Vec};
// --- darwinia-network ---
use codec::{Decode, Encode};
use dp_asset::token::TokenMetadata;

/// The parameters box for the pallet runtime call.
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub enum CallParams {
	#[codec(index = 0)]
	S2sIssuingPalletRegisterFromRemote(TokenMetadata),
	#[codec(index = 1)]
	S2sIssuingPalletIssueFromRemote(H160, U256, H160),
	#[codec(index = 2)]
	S2sBackingPalletUnlockFromRemote(H160, U256, Vec<u8>),
}

/// Creating a concrete message payload which would be relay to target chain.
pub trait PayloadCreate<AccountId, MessagePayload>
where
	AccountId: Encode + Decode,
{
	fn encode_call(pallet_index: u8, call_params: CallParams) -> Result<Vec<u8>, &'static str> {
		let mut encoded = vec![pallet_index];
		encoded.append(&mut call_params.encode());
		Ok(encoded)
	}

	fn payload(
		submitter: AccountId,
		spec_version: u32,
		weight: u64,
		call_params: CallParams,
	) -> Result<MessagePayload, &'static str>;
}

/// Creating a concrete message payload which would be relay to target chain.
pub trait CreatePayload<AccountId>
where
	AccountId: Encode + Decode,
	Self: Sized,
{
	type payload;

	fn encode_call(pallet_index: u8, call_params: CallParams) -> Result<Vec<u8>, &'static str> {
		let mut encoded = vec![pallet_index];
		encoded.append(&mut call_params.encode());
		Ok(encoded)
	}

	fn create(
		submitter: AccountId,
		spec_version: u32,
		weight: u64,
		call_params: CallParams,
	) -> Result<Self, &'static str>;
}

#[cfg(test)]
mod test {
	use super::*;
	use dp_asset::token::TokenMetadata;

	#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]

	pub enum PangoroRuntime {
		#[codec(index = 20)]
		Sub2SubBacking(S2SBackingCall),
	}
	#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
	#[allow(non_camel_case_types)]
	pub enum S2SBackingCall {
		#[codec(index = 2)]
		unlock_from_remote(H160, U256, AccountId32),
	}

	#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
	pub enum PangolinRuntime {
		#[codec(index = 49)]
		Sub2SubIssuing(S2SIssuingCall),
	}

	#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
	#[allow(non_camel_case_types)]
	pub enum S2SIssuingCall {
		#[codec(index = 0)]
		register_from_remote(TokenMetadata),
		#[codec(index = 1)]
		issue_from_remote(H160, U256, H160),
	}

	pub struct MockPangoroPayloadCreator;
	impl PayloadCreate<u64, Vec<u8>> for MockPangoroPayloadCreator {
		fn payload(
			_submitter: u64,
			_spec_version: u32,
			_weight: u64,
			call_params: CallParams,
		) -> Result<Vec<u8>, &'static str> {
			Self::encode_call(20, call_params)
		}
	}

	pub struct MockPangolinPayloadCreator;
	impl PayloadCreate<u64, Vec<u8>> for MockPangolinPayloadCreator {
		fn payload(
			_submitter: u64,
			_spec_version: u32,
			_weight: u64,
			call_params: CallParams,
		) -> Result<Vec<u8>, &'static str> {
			Self::encode_call(49, call_params)
		}
	}

	#[test]
	fn test_pangoro_runtime_call_encode() {
		let expected_encoded_call =
			PangoroRuntime::Sub2SubBacking(S2SBackingCall::unlock_from_remote(
				H160::zero(),
				U256::zero(),
				AccountId32::new([1; 32]),
			))
			.encode();

		let encoded = MockPangoroPayloadCreator::payload(
			1,
			0,
			0,
			CallParams::S2sBackingPalletUnlockFromRemote(
				H160::zero(),
				U256::zero(),
				AccountId32::new([1; 32]),
			),
		)
		.unwrap();
		assert_eq!(encoded, expected_encoded_call);
	}

	#[test]
	fn test_pangolin_runtime_call_encode() {
		let mock_token = TokenMetadata::new(1, H160::zero(), vec![1, 2, 3], vec![1, 2, 3], 9);

		let expected_encoded_call = PangolinRuntime::Sub2SubIssuing(
			S2SIssuingCall::register_from_remote(mock_token.clone()),
		)
		.encode();
		let encoded = MockPangolinPayloadCreator::payload(
			1,
			0,
			0,
			CallParams::S2sIssuingPalletRegisterFromRemote(mock_token.clone()),
		)
		.unwrap();
		assert_eq!(expected_encoded_call, encoded);

		let expected_encoded_call = PangolinRuntime::Sub2SubIssuing(
			S2SIssuingCall::issue_from_remote(H160::zero(), U256::zero(), H160::zero()),
		)
		.encode();
		let encoded = MockPangolinPayloadCreator::payload(
			1,
			0,
			0,
			CallParams::S2sIssuingPalletIssueFromRemote(H160::zero(), U256::zero(), H160::zero()),
		)
		.unwrap();
		assert_eq!(expected_encoded_call, encoded);
	}
}
