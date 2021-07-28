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

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{format, string::String};

use core::fmt;
use rlp::DecoderError;
#[cfg(not(feature = "std"))]
use sp_std::borrow::ToOwned;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrieError {
	DB(String),
	Decoder(DecoderError),
	InvalidData,
	InvalidStateRoot,
	InvalidProof,
}
impl fmt::Display for TrieError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		fmt::Debug::fmt(&self, f)
	}
}
impl From<DecoderError> for TrieError {
	fn from(e: DecoderError) -> Self {
		TrieError::Decoder(e)
	}
}
