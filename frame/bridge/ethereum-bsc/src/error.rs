// Copyright 2019-2021 Parity Technologies (UK) Ltd.
// This file is part of Parity Bridges Common.

// Parity Bridges Common is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity Bridges Common is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity Bridges Common.  If not, see <http://www.gnu.org/licenses/>.

use sp_runtime::RuntimeDebug;
use std::fmt;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
/// Error indicating an expected value was not found.
pub struct Mismatch<T> {
	/// Value expected.
	pub expect: T,
	/// Value found.
	pub found: T,
}

impl<T: fmt::Display> fmt::Display for Mismatch<T> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.write_fmt(format_args!("Expected {}, found {}", self.expect, self.found))
	}
}

/// Header import error.
#[derive(Clone, Copy, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(PartialEq))]
pub enum Error {
	/// Block number isn't sensible.
	RidiculousNumber,
	/// Block has too much gas used.
	TooMuchGasUsed,
	/// Gas limit header field is invalid.
	InvalidGasLimit,
	/// Difficulty header field is invalid.
	InvalidDifficulty,
	/// Header timestamp is ahead of on-chain timestamp
	HeaderTimestampIsAhead,
	/// extra-data 32 byte vanity prefix missing
	/// MissingVanity is returned if a block's extra-data section is shorter than
	/// 32 bytes, which is required to store the validator(signer) vanity.
	MissingVanity,
	/// extra-data 65 byte signature suffix missing
	/// MissingSignature is returned if a block's extra-data section doesn't seem
	/// to contain a 65 byte secp256k1 signature
	MissingSignature,
	/// non-checkpoint block contains extra validator list
	/// ExtraValidators is returned if non-checkpoint block contain validator data in
	/// their extra-data fields
	ExtraValidators,
	/// Invalid validator list on checkpoint block
	/// errInvalidCheckpointValidators is returned if a checkpoint block contains an
	/// invalid list of validators (i.e. non divisible by 20 bytes).
	InvalidCheckpointValidators,
	/// Non-zero mix digest
	/// InvalidMixDigest is returned if a block's mix digest is non-zero.
	InvalidMixDigest,
	/// Non empty uncle hash
	/// InvalidUncleHash is returned if a block contains an non-empty uncle list.
	InvalidUncleHash,
	/// Non empty nonce
	/// InvalidNonce is returned if a block header nonce is non-empty
	InvalidNonce,
	/// UnknownAncestor is returned when validating a block requires an ancestor that is unknown.
	UnknownAncestor,
	/// Header timestamp too close
	/// HeaderTimestampTooClose is returned when header timestamp is too close with parent's
	HeaderTimestampTooClose,
	/// Missing signers
	CheckpointNoSigner,
	/// EC_RECOVER error
	RecoverPubkeyFail,
	/// List of signers is invalid
	CheckpointInvalidSigners(usize),
}

impl Error {
	pub fn msg(&self) -> &'static str {
		match *self {
			Error::RidiculousNumber => "Header has too large number",
			Error::InvalidGasLimit => "Header has invalid gas limit",
			Error::InvalidDifficulty => "Header has invalid difficulty",
			Error::HeaderTimestampIsAhead => "Header timestamp is ahead of on-chain timestamp",
			Error::MissingVanity => "Extra-data 32 byte vanity prefix missing",
			Error::MissingSignature => "Extra-data 65 byte signature suffix missing",
			Error::ExtraValidators => "Non-checkpoint block contains extra validator list",
			Error::InvalidCheckpointValidators => "Invalid validator list on checkpoint block",
			Error::InvalidMixDigest => "Non-zero mix digest",
			Error::InvalidUncleHash => "Non empty uncle hash",
			Error::InvalidNonce => "Non empty nonce",
			Error::UnknownAncestor => "Unknow ancestor",
			Error::HeaderTimestampTooClose => "Header timestamp too close",
			Error::CheckpointNoSigner => "Missing signers",
			// TODO how to format this and return a static str?
			Error::RecoverPubkeyFail => "Recover pubkey from signature error",
			_ => "Unknown error.",
		}
	}
}
