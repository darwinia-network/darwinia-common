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

pub mod relay_authorities;
pub mod relayer_game;

// --- darwinia ---
use ethereum_primitives::EthereumAddress;
pub use relay_authorities::*;
pub use relayer_game::*;

use codec::{Decode, Encode};
use sp_runtime::DispatchError;

pub trait Relay {
	type RelayOrigin: Clone + PartialOrd;
	type RelayMessage: Encode + Decode + Clone;

	fn verify_origin(proof: &Self::RelayOrigin) -> Result<EthereumAddress, DispatchError>;
	fn relay_message(message: &Self::RelayMessage) -> Result<(), DispatchError>;
	fn digest() -> RelayDigest;
}

#[derive(Encode, Decode, Clone, Debug, Eq, PartialEq)]
pub enum RelayAccount<AccountId> {
	EthereumAccount(EthereumAddress),
	DarwiniaAccount(AccountId),
}

pub type RelayDigest = [u8; 4];
