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
// --- paritytech ---
use bp_message_dispatch::CallOrigin;
use bp_runtime::messages::DispatchFeePayment;
use sp_core::{H160, U256};
use sp_std::{vec, vec::Vec};
// --- darwinia-network ---
use dp_asset::TokenMetadata;

/// Creating a concrete message payload which would be relay to target chain.
pub trait CreatePayload<SourceChainAccountId, TargetChainAccountPublic, TargetChainSignature> {
	type Payload: Encode;

	fn encode_call(pallet_index: u8, call_params: CallParams) -> Result<Vec<u8>, &'static str> {
		let mut encoded = vec![pallet_index];
		encoded.append(&mut call_params.encode());
		Ok(encoded)
	}

	fn create(
		origin: CallOrigin<SourceChainAccountId, TargetChainAccountPublic, TargetChainSignature>,
		spec_version: u32,
		weight: u64,
		call_params: CallParams,
		dispatch_fee_payment: DispatchFeePayment,
	) -> Result<Self::Payload, &'static str>;
}
impl<AccountId, Signer, Signature> CreatePayload<AccountId, Signer, Signature> for () {
	type Payload = ();

	fn create(
		_: CallOrigin<AccountId, Signer, Signature>,
		_: u32,
		_: u64,
		_: CallParams,
		_: DispatchFeePayment,
	) -> Result<Self::Payload, &'static str> {
		Ok(())
	}
}

/// The parameters box for the pallet runtime call.
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub enum CallParams {
	#[codec(index = 0)]
	S2sIssuingPalletRegisterFromRemote(TokenMetadata),
	#[codec(index = 1)]
	S2sIssuingPalletIssueFromRemote(H160, U256, H160),
	#[codec(index = 2)]
	S2sBackingPalletUnlockFromRemote(H160, U256, Vec<u8>),
	RawCall(Vec<u8>),
}

#[cfg(test)]
mod test {
	// --- darwinia-network ---
	use super::*;
	use dp_asset::TokenMetadata;

	#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]

	pub enum PangoroRuntime {
		#[codec(index = 20)]
		Sub2SubBacking(S2SBackingCall),
	}
	#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
	#[allow(non_camel_case_types)]
	pub enum S2SBackingCall {
		#[codec(index = 2)]
		unlock_from_remote(H160, U256, Vec<u8>),
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
	impl CreatePayload<u64, (), ()> for MockPangoroPayloadCreator {
		type Payload = Vec<u8>;
		fn create(
			_origin: CallOrigin<u64, (), ()>,
			_spec_version: u32,
			_weight: u64,
			call_params: CallParams,
			_dispatch_fee_payment: DispatchFeePayment,
		) -> Result<Vec<u8>, &'static str> {
			Self::encode_call(20, call_params)
		}
	}

	pub struct MockPangolinPayloadCreator;
	impl CreatePayload<u64, (), ()> for MockPangolinPayloadCreator {
		type Payload = Vec<u8>;
		fn create(
			_origin: CallOrigin<u64, (), ()>,
			_spec_version: u32,
			_weight: u64,
			call_params: CallParams,
			_dispatch_fee_payment: DispatchFeePayment,
		) -> Result<Vec<u8>, &'static str> {
			Self::encode_call(49, call_params)
		}
	}

	#[test]
	fn test_pangoro_runtime_call_encode() {
		let expected_encoded_call = PangoroRuntime::Sub2SubBacking(
			S2SBackingCall::unlock_from_remote(H160::zero(), U256::zero(), vec![1; 32]),
		)
		.encode();

		let encoded = MockPangoroPayloadCreator::create(
			CallOrigin::SourceRoot,
			0,
			0,
			CallParams::S2sBackingPalletUnlockFromRemote(H160::zero(), U256::zero(), vec![1; 32]),
			DispatchFeePayment::AtSourceChain,
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
		let encoded = MockPangolinPayloadCreator::create(
			CallOrigin::SourceRoot,
			0,
			0,
			CallParams::S2sIssuingPalletRegisterFromRemote(mock_token.clone()),
			DispatchFeePayment::AtSourceChain,
		)
		.unwrap();
		assert_eq!(expected_encoded_call, encoded);

		let expected_encoded_call = PangolinRuntime::Sub2SubIssuing(
			S2SIssuingCall::issue_from_remote(H160::zero(), U256::zero(), H160::zero()),
		)
		.encode();
		let encoded = MockPangolinPayloadCreator::create(
			CallOrigin::SourceRoot,
			0,
			0,
			CallParams::S2sIssuingPalletIssueFromRemote(H160::zero(), U256::zero(), H160::zero()),
			DispatchFeePayment::AtSourceChain,
		)
		.unwrap();
		assert_eq!(expected_encoded_call, encoded);
	}
}
