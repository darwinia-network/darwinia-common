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

use evm::ExitRevert;
use fp_evm::PrecompileFailure;
use sha3::{Digest, Keccak256};

// Expanded to:
// pub enum Action {
//     A = 230582047,
//     B = 1308091344,
// }
// impl Action {
//     pub fn from_u32(value: u32) -> Result<Self, PrecompileFailure> {
//         match value {
//             230582047 => Ok(Action::A),
//             1308091344 => Ok(Action::B),
//             _ => Err(PrecompileFailure::Revert {
//                 exit_status: ExitRevert::Reverted,
//                 output: b"Unknown action".to_vec(),
//                 cost: 0,
//             }),
//         }
//     }
// }

#[darwinia_evm_precompile_utils_macro::selector]
#[derive(Debug, PartialEq)]
pub enum Action {
	A = "a()",
	B = "b()",
}

#[test]
fn test_selector_macro() {
	assert_eq!(&(Action::A as u32).to_be_bytes()[..], &Keccak256::digest(b"a()")[0..4],);
	assert_eq!(&(Action::B as u32).to_be_bytes()[..], &Keccak256::digest(b"b()")[0..4],);
	assert_ne!(Action::A as u32, Action::B as u32);
}

#[test]
fn test_from_u32() {
	assert_eq!(Action::A, Action::from_u32(230582047).unwrap());
	assert_eq!(Action::B, Action::from_u32(1308091344).unwrap());

	assert!(Action::from_u32(230582047 + 1).is_err());
}
