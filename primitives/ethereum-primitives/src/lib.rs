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

extern crate alloc;

pub mod error;
pub mod ethashproof;
pub mod header;
pub mod pow;
pub mod receipt;

pub use ethereum_types::{Address as EthereumAddress, *};

// --- alloc ---
use alloc::vec::Vec;
// --- crates.io ---
#[cfg(any(feature = "full-codec", test))]
use codec::{Decode, Encode};

pub type Bytes = Vec<u8>;
pub type EthereumBlockNumber = u64;

#[cfg_attr(any(feature = "full-codec", test), derive(Encode, Decode))]
#[derive(Clone, PartialEq)]
pub enum EthereumNetworkType {
	Mainnet,
	Ropsten,
}
impl Default for EthereumNetworkType {
	fn default() -> EthereumNetworkType {
		EthereumNetworkType::Mainnet
	}
}
