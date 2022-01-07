// This file is part of Darwinia.
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

//! Token Primitives
#![cfg_attr(not(feature = "std"), no_std)]

// --- crates.io ---
use codec::{Decode, Encode};
use scale_info::TypeInfo;
// --- darwinia-network ---
use ethereum_types::H160;
use sp_std::vec::Vec;

pub const NATIVE_TOKEN_TYPE: u32 = 0;
pub const ERC20_TOKEN_TYPE: u32 = 1;

/// the token extra options
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo)]
pub struct TokenMetadata {
	pub token_type: u32,
	pub address: H160,
	pub name: Vec<u8>,
	pub symbol: Vec<u8>,
	pub decimal: u8,
}

impl TokenMetadata {
	pub fn new(
		token_type: u32,
		address: H160,
		name: Vec<u8>,
		symbol: Vec<u8>,
		decimal: u8,
	) -> Self {
		Self {
			token_type,
			address,
			name,
			symbol,
			decimal,
		}
	}
}
