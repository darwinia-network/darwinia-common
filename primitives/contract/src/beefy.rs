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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

// --- crates.io ---
use ethabi::{Bytes, Function, Param, ParamType, Result, StateMutability};
use sp_std::vec;

pub fn commitment() -> Result<Bytes> {
	#[allow(deprecated)]
	Function {
		name: "commitment".into(),
		inputs: vec![],
		outputs: vec![Param {
			name: "hash".into(),
			kind: ParamType::FixedBytes(32),
			internal_type: Some("bytes4".into()),
		}],
		constant: true,
		state_mutability: StateMutability::View,
	}
	.encode_input(&[])
}
