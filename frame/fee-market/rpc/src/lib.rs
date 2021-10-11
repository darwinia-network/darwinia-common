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

//! Node-specific RPC methods for interaction with fee-market.

// --- darwinia-network ---
pub use darwinia_fee_market_rpc_runtime_api::FeeMarketApi as FeeMarketRuntimeApi;

// --- core ---
use core::fmt::Debug;
// --- std ---
use std::sync::Arc;
// --- crates.io ---
use codec::Codec;
use jsonrpc_core::{Error, ErrorCode, Result};
use jsonrpc_derive::rpc;
// --- paritytech ---
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{
	generic::BlockId,
	traits::{Block as BlockT, MaybeDisplay, MaybeFromStr},
};

const RUNTIME_ERROR: i64 = -1;

#[rpc]
pub trait FeeMarketApi<Response> {
	#[rpc(name = "feeMarket_marketFee")]
	fn market_fee(&self) -> Result<Response>;
}

pub struct FeeMarket<Client, Block> {
	client: Arc<Client>,
	_marker: std::marker::PhantomData<Block>,
}

impl<Client, Block> FeeMarket<Client, Block> {
	pub fn new(client: Arc<Client>) -> Self {
		Self {
			client,
			_marker: Default::default(),
		}
	}
}

impl<Client, Block, Fee> FeeMarketApi<Option<Fee>> for FeeMarket<Client, Block>
where
	Client: 'static + Send + Sync + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	Client::Api: FeeMarketRuntimeApi<Block, Fee>,
	Block: BlockT,
	Fee: Debug + Codec + MaybeDisplay + MaybeFromStr,
{
	fn market_fee(&self) -> Result<Option<Fee>> {
		let api = self.client.runtime_api();
		let best = self.client.info().best_hash;
		let at = BlockId::hash(best);

		api.market_fee(&at).map_err(|e| Error {
			code: ErrorCode::ServerError(RUNTIME_ERROR),
			message: "Unable to query market fee.".into(),
			data: Some(format!("{:?}", e).into()),
		})
	}
}
