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

// --- paritytech ---
use sp_debug_derive::RuntimeDebug;

#[derive(Clone, PartialEq, Eq, RuntimeDebug)]
#[repr(u8)]
pub enum TypedTxId {
	EIP1559Transaction = 0x02,
	AccessList = 0x01,
	Legacy = 0x00,
}
impl TypedTxId {
	pub fn try_from_wire_byte(n: u8) -> Result<Self, &'static str> {
		match n {
			x if x == TypedTxId::EIP1559Transaction as u8 => Ok(TypedTxId::EIP1559Transaction),
			x if x == TypedTxId::AccessList as u8 => Ok(TypedTxId::AccessList),
			x if (x & 0x80) != 0x00 => Ok(TypedTxId::Legacy),
			_ => Err("Invalid Tx Id"),
		}
	}
}
