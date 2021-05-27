// Copyright 2015-2020 Parity Technologies (UK) Ltd.
// This file is part of OpenEthereum.

// OpenEthereum is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// OpenEthereum is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with OpenEthereum.  If not, see <http://www.gnu.org/licenses/>.

use crate::*;
use bp_bsc::{Address, BSCHeader, ADDRESS_LENGTH, H160, SIGNATURE_LENGTH, VANITY_LENGTH};
use lru_cache::LruCache;
use parking_lot::RwLock;
use sp_core::H256;

/// How many recovered signature to cache in the memory.
pub const CREATOR_CACHE_NUM: usize = 4096;
lazy_static::lazy_static! {
	/// key: header hash
	/// value: creator address
	pub static ref CREATOR_BY_HASH: RwLock<LruCache<H256, Address>> = RwLock::new(LruCache::new(CREATOR_CACHE_NUM));
}

/// Extract authority set from extra_data.
///
/// Layout of extra_data:
/// ----
/// VANITY: 32 bytes
/// Signers: N * 32 bytes as hex encoded (20 characters)
/// Signature: 65 bytes
/// --
pub fn extract_authorities(header: &BSCHeader) -> Result<Vec<Address>, Error> {
	let data = &header.extra_data;

	if data.len() <= VANITY_LENGTH + SIGNATURE_LENGTH {
		return Err(Error::CheckpointNoSigner);
	}

	// extract only the portion of extra_data which includes the signer list
	let signers_raw = &data[(VANITY_LENGTH)..data.len() - (SIGNATURE_LENGTH)];

	if signers_raw.len() % ADDRESS_LENGTH != 0 {
		return Err(Error::CheckpointInvalidSigners(signers_raw.len()));
	}

	let num_signers = signers_raw.len() / 20;

	let signers: Vec<Address> = (0..num_signers)
		.map(|i| {
			let start = i * ADDRESS_LENGTH;
			let end = start + ADDRESS_LENGTH;
			H160::from_slice(&signers_raw[start..end])
		})
		.collect();

	Ok(signers)
}
