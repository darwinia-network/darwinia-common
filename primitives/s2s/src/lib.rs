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
use sp_core::H160;
use sp_std::vec::Vec;
// --- darwinia-network ---
use dp_asset::token::Token;
use dp_contract::mapping_token_factory::s2s::S2sRemoteUnlockInfo;

/// The parameters box for the pallet runtime call.
#[derive(Clone)]
pub enum CallParams<AccountId> {
	RegisterFromRemote(Token),
	IssueFromRemote(Token, H160),
	UnlockFromRemote(AccountId, S2sRemoteUnlockInfo),
}

/// Encoding the call parameters to dispatch call binary.
pub trait EncodeCall<AccountId> {
	fn encode_call(call_params: CallParams<AccountId>) -> Result<Vec<u8>, ()>;
}

/// Creating a concrete message payload which would be relay to target chain.
pub trait PayloadCreate<AccountId, MessagePayload> {
	fn payload(
		spec_version: u32,
		weight: u64,
		call_params: CallParams<AccountId>,
	) -> Result<MessagePayload, ()>;
}
