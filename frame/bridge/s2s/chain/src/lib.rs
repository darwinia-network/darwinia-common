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
use ethereum_primitives::EthereumAddress;
use sp_std::vec::Vec;

const MILLAU_BACKING_CROSS_RECEIVE: &[u8] = b"millau_backing_cross_receive(address,address)";
const Pangolin_BACKING_CROSS_RECEIVE: &[u8] = b"pangolin_issuing_cross_receive(address,address)";

// here we must contruct a Backing Runtime Call to call backing pallet from the remote issuing
// pallet, because also we have the other direction call from backing pallet to this issuing
// pallet. Otherwise if we import the backing pallet and use the Call definiation. The must be a circular reference
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub enum MillauRuntime {
	/// s2s bridge backing pallet.
	#[codec(index = 49)]
	Sub2SubBacking(MillauSub2SubBackingCall),
}
//
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[allow(non_camel_case_types)]
pub enum MillauSub2SubBackingCall {
	#[codec(index = 0)]
	cross_receive((Token, EthereumAddress)),
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub enum PangolinRuntime {
	Sub2SubIssuing(PangolinSub2SubIssuingCall),
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[allow(non_camel_case_types)]
pub enum PangolinSub2SubIssuingCall {
	#[codec(index = 0)]
	cross_receive((Token, EthereumAddress)),
}

pub enum RelayMessage {
	MillauBacking,
	PangolinIssuing,
}

fn which_relay(selector: [u8; 4]) -> RelayMessage {
	let millau_relay = &sha3::Keccak256::digest(&MILLAU_BACKING_CROSS_RECEIVE)[0..4];
	let pangolin_relay = &sha3::Keccak256::digest(&Pangolin_BACKING_CROSS_RECEIVE)[0..4];

	if selector == millau_relay {
		return RelayMessage::MillauBacking;
	}
	RelayMessage::PangolinIssuing
}

pub fn encode_relay_message<AccountId>(
	selector: [u8; 4],
	token: Token,
	recipient: RelayAccount<AccountId>,
) -> Result<Vec<u8>, ()> {
	match recipient {
		RelayAccount::<AccountId>::EthereumAccount(r) => match which_relay(selector) {
			RelayMessage::MillauBacking => Ok(MillauRuntime::Sub2SubBacking(
				MillauSub2SubBackingCall::cross_receive((token, r)),
			)
			.encode()),
			RelayMessage::PangolinIssuing => Ok(PangolinRuntime::Sub2SubIssuing(
				PangolinSub2SubIssuingCall::cross_receive((token, r)),
			)
			.encode()),
			_ => Err(()),
		},
		_ => Err(()),
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use sha3::Digest;

	#[test]
	fn test_chain_selector() {
		let m_action = &sha3::Keccak256::digest(&MILLAU_BACKING_CROSS_RECEIVE)[0..4];
		let p_action = &sha3::Keccak256::digest(&Pangolin_BACKING_CROSS_RECEIVE)[0..4];

		eprintln!("m {:?}", m_action);
		eprintln!("p {:?}", p_action);
	}
}
