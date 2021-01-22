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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

// --- darwinia ---
use crate::*;

/// Extract value from JSON response
#[test]
fn test_extract_value_from_json_response() {
	let result_part = literal_procesor::extract_from_json_str(
		&SUPPOSED_SHADOW_FAKE_RESPONSE[..],
		b"result" as &[u8],
	)
	.unwrap();
	assert_eq!(
		result_part,
		br#""eth_header":{eth-content},"proof":[proof-content]"# as &[u8]
	);
	let eth_header_part =
		literal_procesor::extract_from_json_str(result_part, b"eth_header" as &[u8]).unwrap();
	assert_eq!(eth_header_part, br#"eth-content"# as &[u8]);
	let proof_header_part =
		literal_procesor::extract_from_json_str(result_part, b"proof" as &[u8]).unwrap();
	assert_eq!(proof_header_part, br#"proof-content"# as &[u8]);
}

/// Extract value from JSON response with not alphabetical order
#[test]
fn test_extract_value_from_non_order_json_response() {
	let result_part = literal_procesor::extract_from_json_str(
		&SUPPOSED_SHADOW_NON_ORDER_RESPONSE[..],
		b"result" as &[u8],
	)
	.unwrap();
	assert_eq!(
		result_part,
		br#""proof":[proof-content],"eth_header":{eth-content}"# as &[u8]
	);
	let eth_header_part =
		literal_procesor::extract_from_json_str(result_part, b"eth_header" as &[u8]).unwrap();
	assert_eq!(eth_header_part, br#"eth-content"# as &[u8]);
	let proof_header_part =
		literal_procesor::extract_from_json_str(result_part, b"proof" as &[u8]).unwrap();
	assert_eq!(proof_header_part, br#"proof-content"# as &[u8]);
}

const SUPPOSED_SHADOW_FAKE_RESPONSE: &'static [u8] =
	br#"{"jsonrpc":"2.0","id":1,"result":{"eth_header":{eth-content},"proof":[proof-content]}}"#;
const SUPPOSED_SHADOW_NON_ORDER_RESPONSE: &'static [u8] =
	br#"{"id":1,"result":{"proof":[proof-content],"eth_header":{eth-content}},"jsonrpc":"2.0"}"#;
