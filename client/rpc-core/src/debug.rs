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

// --- crates.io ---
use ethereum_types::H256;
use futures::future::BoxFuture;
use jsonrpc_core::Result as RpcResult;
use jsonrpc_derive::rpc;
use serde::Deserialize;
// --- darwinia-network ---
use darwinia_client_evm_tracer::types::single;
use dp_evm_trace_rpc::RequestBlockId;

pub use rpc_impl_DebugApi::gen_server::DebugApi as DebugApiServer;

#[derive(Clone, Eq, PartialEq, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceParams {
	pub disable_storage: Option<bool>,
	pub disable_memory: Option<bool>,
	pub disable_stack: Option<bool>,
	/// Javascript tracer (we just check if it's Blockscout tracer string)
	pub tracer: Option<String>,
	pub timeout: Option<String>,
}

#[rpc(server)]
pub trait DebugApi {
	#[rpc(name = "debug_traceTransaction")]
	fn trace_transaction(
		&self,
		transaction_hash: H256,
		params: Option<TraceParams>,
	) -> BoxFuture<'static, RpcResult<single::TransactionTrace>>;
	#[rpc(name = "debug_traceBlockByNumber", alias("debug_traceBlockByHash"))]
	fn trace_block(
		&self,
		id: RequestBlockId,
		params: Option<TraceParams>,
	) -> BoxFuture<'static, RpcResult<Vec<single::TransactionTrace>>>;
}