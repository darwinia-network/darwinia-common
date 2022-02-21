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

// --- std ---
use std::{sync::Arc, time::Duration};
// --- crates.io ---
use futures::StreamExt;
use tokio::sync::Semaphore;
// --- paritytech ---
use fc_mapping_sync::{MappingSyncWorker, SyncStrategy};
use fc_rpc::{EthTask, OverrideHandle};
use fc_rpc_core::types::{FeeHistoryCache, FilterPool};
use sc_client_api::BlockchainEvents;
use sc_service::TaskManager;
use sp_blockchain::Error as BlockChainError;
use sp_core::H256;
use sp_runtime::traits::{BlakeTwo256, Block as BlockT};
// --- darwinia-network ---
use dc_rpc::{CacheTask, DebugTask};
use drml_rpc::{EthApiCmd, RpcConfig, RpcRequesters};

pub fn spawn<B, C, BE>(params: DvmTasksParams<B, C, BE>) -> RpcRequesters
where
	C: sp_api::ProvideRuntimeApi<B>,
	C: sc_client_api::BlockOf + sc_client_api::backend::StorageProvider<B, BE>,
	C: sp_blockchain::HeaderBackend<B>,
	C: sp_blockchain::HeaderMetadata<B, Error = BlockChainError> + 'static,
	C: BlockchainEvents<B>,
	C::Api: fp_rpc::EthereumRuntimeRPCApi<B>,
	C::Api: dp_evm_trace_apis::DebugRuntimeApi<B>,
	C::Api: sp_block_builder::BlockBuilder<B>,
	B: BlockT<Hash = H256> + Send + Sync + 'static,
	B::Header: sp_api::HeaderT<Number = u32>,
	BE: sc_client_api::backend::Backend<B> + 'static,
	BE::State: sc_client_api::backend::StateBackend<BlakeTwo256>,
{
	let DvmTasksParams {
		task_manager,
		client,
		substrate_backend,
		dvm_backend,
		filter_pool,
		is_archive,
		rpc_config,
		fee_history_cache,
		overrides,
	} = params;

	if is_archive {
		// Spawn schema cache maintenance task.
		task_manager.spawn_essential_handle().spawn(
			"frontier-schema-cache-task",
			EthTask::ethereum_schema_cache_task(Arc::clone(&client), Arc::clone(&dvm_backend)),
		);
		// Spawn Frontier FeeHistory cache maintenance task.
		task_manager.spawn_essential_handle().spawn(
			"frontier-fee-history",
			EthTask::fee_history_task(
				Arc::clone(&client),
				Arc::clone(&overrides),
				fee_history_cache,
				rpc_config.fee_history_limit,
			),
		);
		// Spawn mapping sync worker task.
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
		// Spawn EthFilterApi maintenance task.
		if let Some(filter_pool) = filter_pool {
			// Each filter is allowed to stay in the pool for 100 blocks.
			const FILTER_RETAIN_THRESHOLD: u64 = 100;
			task_manager.spawn_essential_handle().spawn(
				"frontier-filter-pool",
				EthTask::filter_pool_task(
					Arc::clone(&client),
					filter_pool,
					FILTER_RETAIN_THRESHOLD,
				),
			);
		}
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
			let (debug_task, debug_requester) = DebugTask::task(
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
	pub dvm_backend: Arc<fc_db::Backend<B>>,
	pub filter_pool: Option<FilterPool>,
	pub is_archive: bool,
	pub rpc_config: RpcConfig,
	pub fee_history_cache: FeeHistoryCache,
	pub overrides: Arc<OverrideHandle<B>>,
}
