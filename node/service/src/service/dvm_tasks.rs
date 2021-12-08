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
// --- paritytech ---
use sc_client_api::{backend::Backend as BlockChainBackend, BlockOf, BlockchainEvents};
use sc_service::TaskManager;
use sp_api::{HeaderT, ProvideRuntimeApi};
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use sp_core::H256;
use sp_runtime::traits::Block as BlockT;
// --- darwinia-network ---
use dc_mapping_sync::{MappingSyncWorker, SyncStrategy};
use dc_rpc::EthTask;
use dp_rpc::{FilterPool, PendingTransactions};
use dvm_rpc_runtime_api::EthereumRuntimeRPCApi;

pub struct DvmTasksParams<'a, B: BlockT, C, BE> {
	pub task_manager: &'a TaskManager,
	pub client: Arc<C>,
	pub substrate_backend: Arc<BE>,
	pub dvm_backend: Arc<dc_db::Backend<B>>,
	pub filter_pool: Option<FilterPool>,
	pub pending_transactions: PendingTransactions,
	pub is_archive: bool,
}

pub fn spawn<B, C, BE>(params: DvmTasksParams<B, C, BE>)
where
	C: ProvideRuntimeApi<B> + BlockOf,
	C: HeaderBackend<B> + HeaderMetadata<B, Error = BlockChainError> + 'static,
	C: BlockchainEvents<B>,
	C::Api: EthereumRuntimeRPCApi<B>,
	B: BlockT<Hash = H256> + Send + Sync + 'static,
	B::Header: HeaderT<Number = u32>,
	BE: BlockChainBackend<B> + 'static,
{
	let DvmTasksParams {
		task_manager,
		client,
		substrate_backend,
		dvm_backend,
		filter_pool,
		pending_transactions,
		is_archive,
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
}
