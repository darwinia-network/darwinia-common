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

// --- crates.io ---
#[cfg(feature = "codec")]
use codec::{Decode, Encode};
// --- darwinia-network ---
use crate::*;

#[cfg_attr(feature = "codec", derive(Encode, Decode))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Error indicating value found is outside of a valid range.
pub struct OutOfBounds<T> {
	/// Minimum allowed value.
	pub min: Option<T>,
	/// Maximum allowed value.
	pub max: Option<T>,
	/// Value found.
	pub found: T,
}

#[cfg_attr(feature = "codec", derive(Encode, Decode))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Error indicating an expected value was not found.
pub struct Mismatch<T> {
	/// Value expected.
	pub expected: T,
	/// Value found.
	pub found: T,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EthereumError {
	InvalidProofOfWork(OutOfBounds<U256>),
	DifficultyOutOfBounds(OutOfBounds<U256>),
	InvalidSealArity(Mismatch<usize>),
	SealInvalid,
	MerkleProofMismatch(&'static str),
	Rlp(&'static str),
	InvalidReceiptProof,
}

impl From<EthereumError> for &str {
	fn from(e: EthereumError) -> Self {
		use EthereumError::*;

		match e {
			InvalidProofOfWork(_) => "Proof Of Work - INVALID",
			DifficultyOutOfBounds(_) => "Difficulty - OUT OF BOUNDS",
			InvalidSealArity(_) => "Seal Arity - INVALID",
			SealInvalid => "Seal - INVALID",
			MerkleProofMismatch(msg) => msg,
			Rlp(msg) => msg,
			InvalidReceiptProof => "EthereumReceipt Proof - INVALID",
		}
	}
}
