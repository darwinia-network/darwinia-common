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

use bp_bsc::Address;
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
