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

//! Node-specific RPC methods for interaction with staking.

// --- darwinia-network ---
pub use darwinia_staking_rpc_runtime_api::StakingApi as StakingRuntimeApi;

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
// --- darwinia-network ---
use darwinia_staking_rpc_runtime_api::RuntimeDispatchInfo;

const RUNTIME_ERROR: i64 = -1;

#[rpc]
pub trait StakingApi<AccountId, Response> {
	#[rpc(name = "staking_powerOf")]
	fn power_of(&self, who: AccountId) -> Result<Response>;
}

pub struct Staking<Client, Block> {
	client: Arc<Client>,
	_marker: std::marker::PhantomData<Block>,
}

impl<Client, Block> Staking<Client, Block> {
	pub fn new(client: Arc<Client>) -> Self {
		Self { client, _marker: Default::default() }
	}
}

impl<Client, Block, AccountId, Power> StakingApi<AccountId, RuntimeDispatchInfo<Power>>
	for Staking<Client, Block>
where
	Client: 'static + Send + Sync + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	Client::Api: StakingRuntimeApi<Block, AccountId, Power>,
	Block: BlockT,
	AccountId: Codec,
	Power: Debug + Codec + MaybeDisplay + MaybeFromStr,
{
	fn power_of(&self, who: AccountId) -> Result<RuntimeDispatchInfo<Power>> {
		let api = self.client.runtime_api();
		let best = self.client.info().best_hash;
		let at = BlockId::hash(best);

		api.power_of(&at, who).map_err(|e| Error {
			code: ErrorCode::ServerError(RUNTIME_ERROR),
			message: "Unable to query power.".into(),
			data: Some(format!("{:?}", e).into()),
		})
	}
}
