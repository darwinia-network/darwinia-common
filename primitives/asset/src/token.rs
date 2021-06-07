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

//! Token Primitives

// --- core ---
use codec::{Decode, Encode};
use ethereum_primitives::{EthereumAddress, U256};

/// used by token name and symbol
pub type Bytes32 = [u8; 32];

/// the token extra options
#[derive(Encode, Decode, Clone, Debug, Eq, PartialEq)]
pub struct TokenOption {
	pub name: Bytes32,
	pub symbol: Bytes32,
	pub decimal: u8,
}

/// the token metadata
#[derive(Encode, Decode, Clone, Debug, Eq, PartialEq)]
pub struct TokenInfo {
	pub address: EthereumAddress,
	pub value: Option<U256>,
	pub option: Option<TokenOption>,
}

/// The token Definition, Native token or ERC20
#[derive(Encode, Decode, Clone, Debug, Eq, PartialEq)]
pub enum Token {
	Native(TokenInfo),
	Erc20(TokenInfo),
}
