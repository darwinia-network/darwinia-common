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

// --- std ---
use std::{sync::Arc, time::Duration};
// --- crates.io ---
use futures::StreamExt;
use tokio::sync::Semaphore;
// --- paritytech ---
use sc_client_api::{
	backend::{AuxStore, Backend, StateBackend, StorageProvider},
	BlockOf, BlockchainEvents,
};
use sc_service::TaskManager;
use sp_api::{HeaderT, ProvideRuntimeApi};
use sp_block_builder::BlockBuilder;
use sp_blockchain::{
	Backend as BlockchainBackend, Error as BlockChainError, HeaderBackend, HeaderMetadata,
};
use sp_core::H256;
use sp_runtime::traits::Block as BlockT;
// --- darwinia-network ---
use super::*;
use crate::pangolin_service::RuntimeApiCollection;
use dc_mapping_sync::{MappingSyncWorker, SyncStrategy};
use dc_rpc::EthTask;
use dc_tracing_debug_handler::{Debug, DebugHandler, DebugRequester, DebugServer};
use dc_tracing_trace_handler::{
	CacheRequester as TraceFilterCacheRequester, CacheTask, Trace, TraceServer,
};
use dp_evm_trace_apis::DebugRuntimeApi;
use dp_rpc::{FilterPool, PendingTransactions};
use dvm_rpc_runtime_api::EthereumRuntimeRPCApi;

pub fn extend_with_tracing<C, BE>(
	client: Arc<C>,
	requesters: RpcRequesters,
	trace_filter_max_count: u32,
	io: &mut jsonrpc_core::IoHandler<sc_rpc::Metadata>,
) where
	BE: Backend<Block> + 'static,
	BE::State: StateBackend<BlakeTwo256>,
	BE::Blockchain: BlockchainBackend<Block>,
	C: ProvideRuntimeApi<Block> + StorageProvider<Block, BE> + AuxStore,
	C: BlockchainEvents<Block>,
	C: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError> + 'static,
	C: Send + Sync + 'static,
	C::Api: RuntimeApiCollection<StateBackend = BE::State>,
{
	if let Some(trace_filter_requester) = requesters.trace {
		io.extend_with(TraceServer::to_delegate(Trace::new(
			client,
			trace_filter_requester,
			trace_filter_max_count,
		)));
	}

	if let Some(debug_requester) = requesters.debug {
		io.extend_with(DebugServer::to_delegate(Debug::new(debug_requester)));
	}
}

pub fn spawn<B, C, BE>(params: DvmTasksParams<B, C, BE>) -> RpcRequesters
where
	C: ProvideRuntimeApi<B> + BlockOf,
	C: HeaderBackend<B> + HeaderMetadata<B, Error = BlockChainError> + 'static,
	C: BlockchainEvents<B>,
	C::Api: EthereumRuntimeRPCApi<B> + DebugRuntimeApi<B>,
	C::Api: BlockBuilder<B>,
	B: BlockT<Hash = H256> + Send + Sync + 'static,
	B::Header: HeaderT<Number = u32>,
	BE: Backend<B> + 'static,
{
	let DvmTasksParams {
		task_manager,
		client,
		substrate_backend,
		dvm_backend,
		filter_pool,
		pending_transactions,
		is_archive,
		rpc_config,
	} = params;
	// Spawn pending transactions maintenance task (as essential, otherwise we leak).
	if let Some(pending_transactions) = pending_transactions {
		const TRANSACTION_RETAIN_THRESHOLD: u64 = 5;
		task_manager.spawn_essential_handle().spawn(
			"frontier-pending-transactions",
			EthTask::pending_transaction_task(
				Arc::clone(&client),
				pending_transactions,
				TRANSACTION_RETAIN_THRESHOLD,
			),
		);
	}

	// Spawn schema cache maintenance task.
	task_manager.spawn_essential_handle().spawn(
		"frontier-schema-cache-task",
		EthTask::ethereum_schema_cache_task(Arc::clone(&client), Arc::clone(&dvm_backend)),
	);

	// Spawn mapping sync worker task.
	if is_archive {
		task_manager.spawn_essential_handle().spawn(
			"frontier-mapping-sync-worker",
			MappingSyncWorker::new(
				client.import_notification_stream(),
				Duration::new(6, 0),
				client.clone(),
				substrate_backend.clone(),
				dvm_backend.clone(),
				SyncStrategy::Normal,
			)
			.for_each(|()| futures::future::ready(())),
		);
	}

	// Spawn EthFilterApi maintenance task.
	if let Some(filter_pool) = filter_pool {
		// Each filter is allowed to stay in the pool for 100 blocks.
		const FILTER_RETAIN_THRESHOLD: u64 = 100;
		task_manager.spawn_essential_handle().spawn(
			"frontier-filter-pool",
			EthTask::filter_pool_task(Arc::clone(&client), filter_pool, FILTER_RETAIN_THRESHOLD),
		);
	}

	let cmd = rpc_config.ethapi.clone();
	if cmd.contains(&EthApiCmd::Debug) || cmd.contains(&EthApiCmd::Trace) {
		let permit_pool = Arc::new(Semaphore::new(rpc_config.ethapi_max_permits as usize));
		let (trace_filter_task, trace_filter_requester) =
			if rpc_config.ethapi.contains(&EthApiCmd::Trace) {
				let (trace_filter_task, trace_filter_requester) = CacheTask::create(
					Arc::clone(&client),
					Arc::clone(&substrate_backend),
					Duration::from_secs(rpc_config.ethapi_trace_cache_duration),
					Arc::clone(&permit_pool),
				);
				(Some(trace_filter_task), Some(trace_filter_requester))
			} else {
				(None, None)
			};

		let (debug_task, debug_requester) = if rpc_config.ethapi.contains(&EthApiCmd::Debug) {
			let (debug_task, debug_requester) = DebugHandler::task(
				Arc::clone(&client),
				Arc::clone(&substrate_backend),
				Arc::clone(&dvm_backend),
				Arc::clone(&permit_pool),
			);
			(Some(debug_task), Some(debug_requester))
		} else {
			(None, None)
		};

		// `trace_filter` cache task. Essential.
		// Proxies rpc requests to it's handler.
		if let Some(trace_filter_task) = trace_filter_task {
			params
				.task_manager
				.spawn_essential_handle()
				.spawn("trace-filter-cache", trace_filter_task);
		}

		// `debug` task if enabled. Essential.
		// Proxies rpc requests to it's handler.
		if let Some(debug_task) = debug_task {
			params
				.task_manager
				.spawn_essential_handle()
				.spawn("ethapi-debug", debug_task);
		}

		return RpcRequesters {
			debug: debug_requester,
			trace: trace_filter_requester,
		};
	}
	RpcRequesters {
		debug: None,
		trace: None,
	}
}

pub struct DvmTasksParams<'a, B: BlockT, C, BE> {
	pub task_manager: &'a TaskManager,
	pub client: Arc<C>,
	pub substrate_backend: Arc<BE>,
	pub dvm_backend: Arc<dc_db::Backend<B>>,
	pub filter_pool: Option<FilterPool>,
	pub pending_transactions: PendingTransactions,
	pub is_archive: bool,
	pub rpc_config: RpcConfig,
}

#[derive(Debug, PartialEq, Clone)]
pub struct RpcConfig {
	pub ethapi: Vec<EthApiCmd>,
	pub ethapi_max_permits: u32,
	pub ethapi_trace_max_count: u32,
	pub ethapi_trace_cache_duration: u64,
	pub eth_log_block_cache: usize,
	pub max_past_logs: u32,
}

#[derive(Debug, PartialEq, Clone)]
pub enum EthApiCmd {
	Txpool,
	Debug,
	Trace,
}

use std::str::FromStr;
impl FromStr for EthApiCmd {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(match s {
			"txpool" => Self::Txpool,
			"debug" => Self::Debug,
			"trace" => Self::Trace,
			_ => {
				return Err(format!(
					"`{}` is not recognized as a supported Ethereum Api",
					s
				))
			}
		})
	}
}
#[derive(Clone)]
pub struct RpcRequesters {
	pub debug: Option<DebugRequester>,
	pub trace: Option<TraceFilterCacheRequester>,
}
