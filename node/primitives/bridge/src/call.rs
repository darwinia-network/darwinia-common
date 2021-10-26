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
use bp_message_dispatch::CallOrigin;
use bp_runtime::messages::DispatchFeePayment;
use codec::{Decode, Encode};
use sp_core::H160;
use sp_std::vec::Vec;
// --- darwinia-network ---
use crate::{FromThisChainMessagePayload, MessageBridge};
use common_primitives::AccountId;
use darwinia_support::{s2s::ToEthAddress, to_bytes32};
use dp_asset::{token::Token, RecipientAccount};
use dp_contract::mapping_token_factory::s2s::S2sRemoteUnlockInfo;
use dp_s2s::{CallParams, EncodeRuntimeCall};

/// The bridged chain(Pangoro) dispatch info
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub enum PangoroRuntime {
	/// NOTE: The index must be the same as the backing pallet in the pangoro runtime
	#[codec(index = 20)]
	Sub2SubBacking(PangoroSub2SubBackingCall),
}

/// The backing call in the pangoro s2s backing pallet
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[allow(non_camel_case_types)]
pub enum PangoroSub2SubBackingCall {
	/// NOTE: The index depends on the call order in the s2s backing pallet.
	#[codec(index = 2)]
	unlock_from_remote(Token, AccountId),
}

/// Bridged chain pangolin call info
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub enum PangolinRuntime {
	/// Note: this index must be the same as the backing pallet in pangolin chain runtime
	#[codec(index = 49)]
	Sub2SubIssuing(PangolinSub2SubIssuingCall),
}

/// Something important to note:
/// The index below represent the call order in the pangolin issuing pallet call.
/// For example, `index = 1` point to the `register_from_remote` (second)call in pangolin runtime.
/// You must update the index here if you change the call order in Pangolin runtime.
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[allow(non_camel_case_types)]
pub enum PangolinSub2SubIssuingCall {
	#[codec(index = 0)]
	register_from_remote(Token),
	#[codec(index = 1)]
	issue_from_remote(Token, H160),
}

pub struct RuntimeCall;
impl EncodeRuntimeCall<AccountId> for RuntimeCall {
	fn encode_call(call_params: CallParams<AccountId>) -> Result<Vec<u8>, ()> {
		let call = match call_params {
			CallParams::RegisterFromRemote(token) => PangolinRuntime::Sub2SubIssuing(
				PangolinSub2SubIssuingCall::register_from_remote(token),
			)
			.encode(),
			CallParams::IssueFromRemote(token, address) => PangolinRuntime::Sub2SubIssuing(
				PangolinSub2SubIssuingCall::issue_from_remote(token, address),
			)
			.encode(),
			CallParams::UnlockFromRemote(account_id, unlock_info) => {
				if unlock_info.recipient.len() != 32 {
					return Err(());
				}

				let recipient_id: AccountId = to_bytes32(unlock_info.recipient.as_slice()).into();

				PangoroRuntime::Sub2SubBacking(PangoroSub2SubBackingCall::unlock_from_remote(
					unlock_info.token,
					recipient_id,
				))
				.encode()
			}
		};
		Ok(call)
	}
}
