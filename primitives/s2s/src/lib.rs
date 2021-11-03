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

mod test;

// --- paritytech ---
use sp_core::{H160, U256};
use sp_std::{vec, vec::Vec};
// --- darwinia-network ---
use codec::{Decode, Encode};
use dp_asset::token::TokenMetadata;
use dp_contract::mapping_token_factory::s2s::S2sRemoteUnlockInfo;

/// The parameters box for the pallet runtime call.
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub enum CallParams {
	#[codec(index = 0)]
	S2sIssuingPalletRegisterFromRemote(TokenMetadata),
	#[codec(index = 1)]
	S2sIssuingPalletIssueFromRemote(H160, U256, H160),
	#[codec(index = 2)]
	S2sBackingPalletUnlockFromRemote(S2sRemoteUnlockInfo),
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
