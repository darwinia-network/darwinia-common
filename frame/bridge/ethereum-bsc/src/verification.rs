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

use crate::error::Error;
use crate::{BSCConfiguration, ChainTime};
use bp_bsc::{
	BSCHeader, ADDRESS_LENGTH, DIFF_INTURN, DIFF_NOTURN, KECCAK_EMPTY_LIST_RLP, SIGNATURE_LENGTH, VANITY_LENGTH,
};

/// Perform basic checks that only require header itself.
pub fn contextless_checks<CT: ChainTime>(
	config: &BSCConfiguration,
	header: &BSCHeader,
	chain_time: &CT,
) -> Result<(), Error> {
	// he genesis block is the always valid dead-end
	if header.number == 0 {
		return Ok(());
	}
	// Don't waste time checking blocks from the future
	if chain_time.is_timestamp_ahead(header.timestamp) {
		return Err(Error::HeaderTimestampIsAhead);
	}
	// Check that the extra-data contains the vanity, validators and signature.
	if header.extra_data.len() < VANITY_LENGTH {
		return Err(Error::MissingVanity);
	}
	if header.extra_data.len() < VANITY_LENGTH + SIGNATURE_LENGTH {
		return Err(Error::MissingSignature);
	}
	if header.number >= u64::max_value() {
		return Err(Error::RidiculousNumber);
	}
	// Ensure that the extra-data contains a validator list on checkpoint, but none otherwise
	let is_checkpoint = header.number % config.epoch_length == 0;
	let validator_bytes_len = header.extra_data.len() - (VANITY_LENGTH + SIGNATURE_LENGTH);
	if !is_checkpoint && validator_bytes_len != 0 {
		return Err(Error::ExtraValidators);
	}
	// Checkpoint blocks must at least contain one validator
	if is_checkpoint && validator_bytes_len == 0 {
		return Err(Error::InvalidCheckpointValidators);
	}
	// Ensure that the validator bytes length is valid
	if is_checkpoint && validator_bytes_len % ADDRESS_LENGTH != 0 {
		return Err(Error::InvalidCheckpointValidators);
	}
	// Ensure that the mix digest is zero as we don't have fork protection currently
	if !header.mix_digest.is_zero() {
		return Err(Error::InvalidMixDigest);
	}
	// Ensure that the block doesn't contain any uncles which are meaningless in PoA
	if header.uncle_hash != KECCAK_EMPTY_LIST_RLP {
		return Err(Error::InvalidUncleHash);
	}
	// Ensure difficulty is valid
	if header.difficulty != DIFF_INTURN && header.difficulty != DIFF_NOTURN {
		return Err(Error::InvalidDifficulty);
	}
	// Ensure that none is empty
	if !header.nonce.len() != 0 {
		return Err(Error::InvalidNonce);
	}
	// Ensure that the block's difficulty is meaningful (may not be correct at this point)
	if header.number > 0 && header.difficulty.is_zero() {
		return Err(Error::InvalidDifficulty);
	}
	if header.gas_used > header.gas_limit {
		return Err(Error::TooMuchGasUsed);
	}
	if header.gas_limit < config.min_gas_limit {
		return Err(Error::InvalidGasLimit);
	}
	if header.gas_limit > config.max_gas_limit {
		return Err(Error::InvalidGasLimit);
	}

	Ok(())
}

/// Perform checks that require access to parent header.
pub fn contextual_checks(config: &BSCConfiguration, header: &BSCHeader, parent: &BSCHeader) -> Result<(), Error> {
	// parent sanity check
	if parent.compute_hash() != header.parent_hash || parent.number + 1 != header.number {
		return Err(Error::UnknownAncestor);
	}

	// Ensure that the block's timestamp isn't too close to it's parent
	// And header.timestamp is greater than parents'
	if header.timestamp < parent.timestamp.saturating_add(config.period) {
		return Err(Error::HeaderTimestampTooClose);
	}

	Ok(())
}
