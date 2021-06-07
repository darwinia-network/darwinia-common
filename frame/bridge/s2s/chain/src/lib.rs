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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

//! s2s chain info include runtime and call method.
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use darwinia_asset_primitives::token::Token;
use darwinia_relay_primitives::RelayAccount;
use darwinia_support::s2s::ChainSelector;
use ethereum_primitives::EthereumAddress;
use sha3::Digest;
use sp_std::vec::Vec;

const MILLAU_BACKING: &[u8] = b"millau_backing_cross_receive(address,address)";
const PANGOLIN_ISSUING: &[u8] = b"pangolin_issuing_cross_receive(address,address)";

// here we must contruct a Backing Runtime Call to call backing pallet from the remote issuing
// pallet, because also we have the other direction call from backing pallet to this issuing
// pallet. Otherwise if we import the backing pallet and use the Call definiation. The must be a circular reference
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub enum MillauRuntime {
	/// s2s bridge backing pallet.
	#[codec(index = 49)]
	S2SBacking(MillauS2SBackingCall),
}
//
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[allow(non_camel_case_types)]
pub enum MillauS2SBackingCall {
	#[codec(index = 0)]
	// TODO: Maybe this call should be: `cross_receive_and_unlock` from backing pallet call
	cross_receive((Token, EthereumAddress)),
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub enum PangolinRuntime {
	S2SIssuing(PangolinS2SIssuingCall),
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[allow(non_camel_case_types)]
pub enum PangolinS2SIssuingCall {
	#[codec(index = 0)]
	cross_receive((Token, EthereumAddress)),
}

pub enum RelayMessage {
	MillauBacking,
	PangolinIssuing,
}

fn which_relay(selector: ChainSelector) -> Result<RelayMessage, ()> {
	let millau_relay = &sha3::Keccak256::digest(&MILLAU_BACKING)[0..4];
	let pangolin_relay = &sha3::Keccak256::digest(&PANGOLIN_ISSUING)[0..4];

	if selector == millau_relay {
		return Ok(RelayMessage::MillauBacking);
	} else if selector == pangolin_relay {
		return Ok(RelayMessage::PangolinIssuing);
	}
	// FIXME: add this error return
	Err(())
}

pub fn encode_relay_message<AccountId>(
	selector: [u8; 4],
	token: Token,
	recipient: RelayAccount<AccountId>,
) -> Result<Vec<u8>, ()> {
	match recipient {
		RelayAccount::<AccountId>::EthereumAccount(r) => match which_relay(selector) {
			Ok(RelayMessage::MillauBacking) => Ok(MillauRuntime::S2SBacking(
				MillauS2SBackingCall::cross_receive((token, r)),
			)
			.encode()),
			Ok(RelayMessage::PangolinIssuing) => Ok(PangolinRuntime::S2SIssuing(
				PangolinS2SIssuingCall::cross_receive((token, r)),
			)
			.encode()),
			Err(_) => todo!(),
		},
		RelayAccount::DarwiniaAccount(_) => unimplemented!(),
	}
}
