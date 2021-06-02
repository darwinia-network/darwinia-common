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

//! Utilities for testing and benchmarking the Ethereum Bridge Pallet.
//!
//! Although the name implies that it is used by tests, it shouldn't be be used _directly_ by tests.
//! Instead these utilities should be used by the Mock runtime, which in turn is used by tests.
//!
//! On the other hand, they may be used directly by the bechmarking module.

// Since this is test code it's fine that not everything is used
#![allow(dead_code)]

use crate::{Config, Storage};

use bp_bsc::{
	rlp_encode,
	signatures::{secret_to_address, sign, SignHeader},
	Address, BSCHeader, Bloom, H256, U256,
	DIFF_INTURN,
};
use secp256k1::SecretKey;
use sp_std::prelude::*;



/// Test header builder.
pub struct HeaderBuilder {
	header: BSCHeader,
	parent_header: BSCHeader,
}

impl HeaderBuilder {
	/// Creates default genesis header.
	pub fn genesis() -> Self {
		Self {
			header: BSCHeader {
				gas_limit: GAS_LIMIT.into(),
				..Default::default()
			},
			parent_header: Default::default(),
		}
	}

	/// Creates default header on top of given parent.
	pub fn with_parent(parent_header: &BSCHeader) -> Self {
		Self {
			header: BSCHeader {
				parent_hash: parent_header.compute_hash(),
				number: parent_header.number + 1,
				gas_limit: GAS_LIMIT.into(),
				difficulty: DIFF_INTURN,
				..Default::default()
			},
			parent_header: parent_header.clone(),
		}
	}


	/// Update difficulty field of this header.
	pub fn difficulty(mut self, difficulty: U256) -> Self {
		self.header.difficulty = difficulty;
		self
	}

	/// Update extra data field of this header.
	pub fn extra_data(mut self, extra_data: Vec<u8>) -> Self {
		self.header.extra_data = extra_data;
		self
	}

	/// Update gas limit field of this header.
	pub fn gas_limit(mut self, gas_limit: U256) -> Self {
		self.header.gas_limit = gas_limit;
		self
	}

	/// Update gas used field of this header.
	pub fn gas_used(mut self, gas_used: U256) -> Self {
		self.header.gas_used = gas_used;
		self
	}

	/// Update log bloom field of this header.
	pub fn log_bloom(mut self, log_bloom: Bloom) -> Self {
		self.header.log_bloom = log_bloom;
		self
	}

	/// Update receipts root field of this header.
	pub fn receipts_root(mut self, receipts_root: H256) -> Self {
		self.header.receipts_root = receipts_root;
		self
	}

	/// Update timestamp field of this header.
	pub fn timestamp(mut self, timestamp: u64) -> Self {
		self.header.timestamp = timestamp;
		self
	}

	/// Update transactions root field of this header.
	pub fn transactions_root(mut self, transactions_root: H256) -> Self {
		self.header.transactions_root = transactions_root;
		self
	}

	/// Signs header by given author.
	pub fn sign_by(self, author: &SecretKey) -> BSCHeader {
		self.header.sign_by(author)
	}
}

/// Helper function for getting a genesis header which has been signed by an authority.
pub fn build_genesis_header(author: &SecretKey) -> BSCHeader {
	let genesis = HeaderBuilder::genesis();
	genesis.header.sign_by(&author)
}

/// Helper function for building a custom child header which has been signed by an authority.
pub fn build_custom_header<F>(author: &SecretKey, previous: &BSCHeader, customize_header: F) -> BSCHeader
where
	F: FnOnce(BSCHeader) -> BSCHeader,
{
	let new_header = HeaderBuilder::with_parent(&previous);
	let custom_header = customize_header(new_header.header);
	custom_header.sign_by(author)
}
