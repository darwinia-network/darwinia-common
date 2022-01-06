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

pub mod unexpected {
	/// Error indicating value found is outside of a valid range.
	#[derive(Clone, Copy, Debug, PartialEq, Eq)]
	pub struct OutOfBounds<T> {
		/// Minimum allowed value.
		pub min: Option<T>,
		/// Maximum allowed value.
		pub max: Option<T>,
		/// Value found.
		pub found: T,
	}

	/// Error indicating an expected value was not found.
	#[derive(Clone, Copy, Debug, PartialEq, Eq)]
	pub struct Mismatch<T> {
		/// Value expected.
		pub expected: T,
		/// Value found.
		pub found: T,
	}
}
pub use unexpected::*;

// --- core ---
use core::fmt;
// --- crates.io ---
#[cfg(any(feature = "full-rlp", test))]
use merkle_patricia_trie::TrieError;
#[cfg(any(feature = "full-rlp", test))]
use rlp::DecoderError;
// --- darwinia-network ---
use crate::{H128, U256};

/// Errors concerning block processing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
	Custom(&'static str),
	Block(BlockError),
	Proof(ProofError),
	#[cfg(any(feature = "full-rlp", test))]
	Rlp(RlpError),
	#[cfg(any(feature = "full-rlp", test))]
	Trie(TrieError),
}
impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		fmt::Debug::fmt(&self, f)
	}
}
impl From<&'static str> for Error {
	fn from(s: &'static str) -> Self {
		Self::Custom(s)
	}
}
impl From<BlockError> for Error {
	fn from(e: BlockError) -> Self {
		Self::Block(e)
	}
}
impl From<ProofError> for Error {
	fn from(e: ProofError) -> Self {
		Self::Proof(e)
	}
}
#[cfg(any(feature = "full-rlp", test))]
impl From<RlpError> for Error {
	fn from(e: RlpError) -> Self {
		Self::Rlp(e)
	}
}
#[cfg(any(feature = "full-rlp", test))]
impl From<TrieError> for Error {
	fn from(e: TrieError) -> Self {
		Self::Trie(e)
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockError {
	InvalidSealArity(Mismatch<usize>),
	DifficultyOutOfBounds(OutOfBounds<U256>),
	InvalidProofOfWork(OutOfBounds<U256>),
	InvalidSeal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProofError {
	TrieKeyNotExist,
	MerkleRootMismatch(Mismatch<H128>),
	MerkleProofOutOfRange(OutOfBounds<usize>),
}

#[cfg(any(feature = "full-rlp", test))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RlpError {
	Decoder(DecoderError),
}
#[cfg(any(feature = "full-rlp", test))]
impl From<DecoderError> for RlpError {
	fn from(e: DecoderError) -> Self {
		Self::Decoder(e)
	}
}
