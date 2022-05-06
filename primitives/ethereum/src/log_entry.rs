// This file is part of Darwinia.
//
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

// --- crates.io ---
#[cfg(any(feature = "full-codec", test))]
use codec::{Decode, Encode};
use ethbloom::Input;
#[cfg(any(feature = "full-rlp", test))]
use rlp_derive::{RlpDecodable, RlpEncodable};
#[cfg(any(feature = "full-codec", test))]
use scale_info::TypeInfo;
use sp_debug_derive::RuntimeDebug;
// --- darwinia-network ---
use crate::*;

#[cfg_attr(any(feature = "full-codec", test), derive(Encode, Decode, TypeInfo))]
#[cfg_attr(any(feature = "full-rlp", test), derive(RlpEncodable, RlpDecodable))]
#[derive(Clone, PartialEq, Eq, RuntimeDebug)]
pub struct LogEntry {
	/// The address of the contract executing at the point of the `LOG` operation.
	pub address: Address,
	/// The topics associated with the `LOG` operation.
	pub topics: Vec<H256>,
	/// The data associated with the `LOG` operation.
	pub data: Bytes,
}
impl LogEntry {
	/// Calculates the bloom of this log entry.
	pub fn bloom(&self) -> Bloom {
		self.topics.iter().fold(Bloom::from(Input::Raw(self.address.as_bytes())), |mut b, t| {
			b.accrue(Input::Raw(t.as_bytes()));
			b
		})
	}
}
