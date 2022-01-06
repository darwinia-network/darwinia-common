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
use sp_core::{H160, U256};
// --- darwinia-network ---
use crate::{evm::IntoH160, *};
use std::str::FromStr;

#[test]
fn const_pow_9_should_work() {
	assert_eq!(
		U256::from(10).checked_pow(U256::from(9)).unwrap(),
		evm::POW_9.into()
	)
}

#[test]
fn test_into_dvm_account() {
	assert_eq!(
		H160::from_str("726f6f7400000000000000000000000000000000").unwrap(),
		(&b"root"[..]).into_h160()
	);
	assert_eq!(
		(&b"longbytes..longbytes..longbytes..longbytes"[..]).into_h160(),
		(&b"longbytes..longbytes"[..]).into_h160()
	);
}
