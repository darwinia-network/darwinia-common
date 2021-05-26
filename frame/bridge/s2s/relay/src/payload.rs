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

//! remote s2s backing Call module.

use codec::{Encode, Decode};
use sp_std::vec::Vec;
use darwinia_asset_primitives::token::Token;
use ethereum_primitives::EthereumAddress;

// here we must contruct a Backing Runtime Call to call backing pallet from the remote issuing
// pallet, because also we have the other direction call from backing pallet to this issuing
// pallet. Otherwise if we import the backing pallet and use the Call definiation. The must be a circular reference
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub enum MillauRuntime {
	/// s2s bridge backing pallet.
	#[codec(index = 49)]
	Millau2PangolinBacking(Millau2PangolinBackingCall),
}
//
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[allow(non_camel_case_types)]
pub enum Millau2PangolinBackingCall {
	#[codec(index = 0)]
    cross_receive((Token, EthereumAddress)),
}

pub fn encode_relay_message(
    index: i32,
    token: Token,
    recipient: EthereumAddress
    ) -> Result<Vec<u8>, ()> {
    match index {
        0 => {
            Ok(MillauRuntime::Millau2PangolinBacking(Millau2PangolinBackingCall::cross_receive((token, recipient))).encode())
        }
        _ => return Err(())
    }
}
