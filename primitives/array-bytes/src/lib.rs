// This file is part of Darwinia.
//
// Copyright (C) 2018-2020 Darwinia Network
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

//! Minimal bytes operation for runtime.

#![cfg_attr(not(feature = "std"), no_std)]

// --- substrate ---
use sp_std::prelude::*;

#[macro_export]
macro_rules! fixed_hex_bytes_unchecked {
	($str:expr, $len:expr) => {{
		let mut bytes: [u8; $len] = [0; $len];
		let slice = $crate::hex_bytes_unchecked($str);
		if slice.len() == $len {
			bytes.copy_from_slice(&slice);
			};
		bytes
		}};
}

#[macro_export]
macro_rules! array_unchecked {
	($source:expr, $offset:expr, $len:expr) => {{
		unsafe { (*($source[$offset..$offset + $len].as_ptr() as *const [_; $len])) }
		}};
}

/// convert number to bytes base on radix `n`
pub fn base_n_bytes_unchecked(mut x: u64, radix: u64) -> Vec<u8> {
	if x == 0 {
		return vec![b'0'];
	}

	if radix > 36 {
		return vec![];
	}

	let mut buf = vec![];
	while x != 0 {
		let rem = (x % radix) as u8;
		if rem < 10 {
			buf.push(b'0' + rem);
		} else {
			buf.push(b'a' + rem - 10);
		}
		x /= radix;
	}

	buf.reverse();
	buf
}

/// convert bytes to hex string
pub fn hex_string_unchecked<B: AsRef<[u8]>>(b: B, prefix: &str) -> Vec<char> {
	let b = b.as_ref();
	let mut v = Vec::with_capacity(prefix.len() + b.len() * 2);

	for x in prefix.chars() {
		v.push(x);
	}

	for x in b.iter() {
		v.push(core::char::from_digit((x >> 4) as _, 16).unwrap_or_default());
		v.push(core::char::from_digit((x & 0xf) as _, 16).unwrap_or_default());
	}

	v
}

/// convert hex string to bytes
pub fn hex_bytes_unchecked<S: AsRef<str>>(s: S) -> Vec<u8> {
	let s = s.as_ref();
	(if s.starts_with("0x") { 2 } else { 0 }..s.len())
		.step_by(2)
		.map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap_or_default())
		.collect()
}
