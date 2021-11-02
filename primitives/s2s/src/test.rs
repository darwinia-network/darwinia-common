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

#[cfg(test)]
mod test {
	use crate::*;
	use dp_asset::token::TokenInfo;

	#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]

	pub enum PangoroRuntime<AccountId> {
		#[codec(index = 20)]
		Sub2SubBacking(S2SBackingCall<AccountId>),
	}
	#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
	#[allow(non_camel_case_types)]
	pub enum S2SBackingCall<AccountId> {
		#[codec(index = 2)]
		unlock_from_remote(AccountId, S2sRemoteUnlockInfo),
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
		register_from_remote(Token),
		#[codec(index = 1)]
		issue_from_remote(Token, H160),
	}

	pub struct MockPangoroPayloadCreator;
	impl PayloadCreate<u64, Vec<u8>> for MockPangoroPayloadCreator {
		fn payload(
			_spec_version: u32,
			_weight: u64,
			call_params: CallParams<u64>,
		) -> Result<Vec<u8>, &'static str> {
			Self::encode_call(20, call_params)
		}
	}

	pub struct MockPangolinPayloadCreator;
	impl PayloadCreate<u64, Vec<u8>> for MockPangolinPayloadCreator {
		fn payload(
			_spec_version: u32,
			_weight: u64,
			call_params: CallParams<u64>,
		) -> Result<Vec<u8>, &'static str> {
			Self::encode_call(49, call_params)
		}
	}

	#[test]
	fn test_pangoro_runtime_call_encode() {
		let unlock_info = S2sRemoteUnlockInfo {
			spec_version: 1,
			weight: 100,
			recipient: vec![1, 2, 3],
			token: Token::Erc20(TokenInfo::new(H160::zero(), None, None)),
		};

		let expected_encoded_call = <PangoroRuntime<u64>>::Sub2SubBacking(
			S2SBackingCall::unlock_from_remote(50, unlock_info.clone()),
		)
		.encode();

		let encoded = MockPangoroPayloadCreator::payload(
			0,
			0,
			<CallParams<u64>>::S2sBackingPalletUnlockFromRemote(50, unlock_info),
		)
		.unwrap();
		assert_eq!(encoded, expected_encoded_call);
	}

	#[test]
	fn test_pangolin_runtime_call_encode() {
		let mock_addr = H160::zero();
		let mock_token = Token::Erc20(TokenInfo::new(mock_addr, None, None));

		let expected_encoded_call = PangolinRuntime::Sub2SubIssuing(
			S2SIssuingCall::register_from_remote(mock_token.clone()),
		)
		.encode();
		let encoded = MockPangolinPayloadCreator::payload(
			0,
			0,
			<CallParams<u64>>::S2sIssuingPalletRegisterFromRemote(mock_token.clone()),
		)
		.unwrap();
		assert_eq!(expected_encoded_call, encoded);

		let expected_encoded_call = PangolinRuntime::Sub2SubIssuing(
			S2SIssuingCall::issue_from_remote(mock_token.clone(), mock_addr),
		)
		.encode();
		let encoded = MockPangolinPayloadCreator::payload(
			0,
			0,
			<CallParams<u64>>::S2sIssuingPalletIssueFromRemote(mock_token, mock_addr),
		)
		.unwrap();
		assert_eq!(expected_encoded_call, encoded);
	}
}
