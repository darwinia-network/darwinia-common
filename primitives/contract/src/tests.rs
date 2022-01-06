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

#[cfg(test)]
mod abi_tests {
	use crate::abi_util::{abi_encode_bytes4, abi_encode_u64};
	use array_bytes::hex2bytes_unchecked;
	use ethabi::{
		param_type::ParamType, token::Token, Bytes, Function, Param, Result, StateMutability,
	};

	#[test]
	fn test_abi_encode_bytes4() {
		assert_eq!(
			hex2bytes_unchecked(
				"0x0101010100000000000000000000000000000000000000000000000000000000"
			),
			abi_encode_bytes4([1; 4])
		);
	}
	#[test]
	fn test_abi_encode_u64() {
		assert_eq!(
			hex2bytes_unchecked(
				"0x0000000000000000000000000000000000000000000000000000000000000040"
			),
			abi_encode_u64(64)
		);
	}

	fn encode_func_with_input_params(param: Vec<u8>) -> Result<Bytes> {
		let inputs = vec![Param {
			name: "param".into(),
			kind: ParamType::FixedBytes(4),
			internal_type: Some("bytes4".into()),
		}];

		#[allow(deprecated)]
		Function {
			name: "test_input_error".into(),
			inputs,
			outputs: vec![],
			constant: false,
			state_mutability: StateMutability::NonPayable,
		}
		.encode_input(vec![Token::FixedBytes(param.to_vec())].as_slice())
	}

	#[test]
	fn test_encode_func_input_with_wrong_size() {
		assert!(encode_func_with_input_params([1; 8].to_vec()).is_err(),);
	}

	#[test]
	fn test_encode_func_input_with_right_size() {
		assert!(encode_func_with_input_params([1; 4].to_vec()).is_ok());
	}
}
