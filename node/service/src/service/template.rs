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

//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.

// --- std ---
use std::{cell::RefCell, sync::Arc};
// --- darwinia-network ---
use crate::service::{
	dvm::{self, DvmTaskParams},
	*,
};
use drml_primitives::{OpaqueBlock as Block, *};
use template_runtime::RuntimeApi;

thread_local!(static TIMESTAMP: RefCell<u64> = RefCell::new(0));

pub type ConsensusResult = (Arc<FullClient<RuntimeApi, Executor>>, bool);

pub const INHERENT_IDENTIFIER: sp_inherents::InherentIdentifier = *b"timstap0";

pub struct Executor;
impl sc_executor::NativeExecutionDispatch for Executor {
	type ExtendHostFunctions = ();

	fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
		template_runtime::api::dispatch(method, data)
	}

	fn native_version() -> sc_executor::NativeVersion {
		template_runtime::native_version()
	}
}

/// Provide a mock duration starting at 0 in millisecond for timestamp inherent.
/// Each call will increment timestamp by slot_duration making Aura think time has passed.
pub struct MockTimestampInherentDataProvider;
#[async_trait::async_trait]
impl sp_inherents::InherentDataProvider for MockTimestampInherentDataProvider {
	fn provide_inherent_data(
		&self,
		inherent_data: &mut sp_inherents::InherentData,
	) -> Result<(), sp_inherents::Error> {
		TIMESTAMP.with(|x| {
			*x.borrow_mut() += SLOT_DURATION;
			inherent_data.put_data(INHERENT_IDENTIFIER, &*x.borrow())
		})
	}

	async fn try_handle_error(
		&self,
		_identifier: &sp_inherents::InherentIdentifier,
		_error: &[u8],
	) -> Option<Result<(), sp_inherents::Error>> {
		// The pallet never reports error.
		None
	}
}

pub fn new_partial(
	config: &sc_service::Configuration,
	is_manual_sealing: bool,
) -> ServiceResult<
	sc_service::PartialComponents<
		FullClient<RuntimeApi, Executor>,
		FullBackend,
		FullSelectChain,
		sc_consensus::DefaultImportQueue<Block, FullClient<RuntimeApi, Executor>>,
		sc_transaction_pool::FullPool<Block, FullClient<RuntimeApi, Executor>>,
		(ConsensusResult, Option<fc_rpc_core::types::FilterPool>, Arc<fc_db::Backend<Block>>),
	>,
> {
	// --- std ---
	use std::{collections::BTreeMap, sync::Mutex};
	// --- paritytech ---
	use fc_rpc_core::types::FilterPool;
	use sc_executor::NativeElseWasmExecutor;
	use sc_service::error::Error as ServiceError;

	if config.keystore_remote.is_some() {
		return Err(ServiceError::Other(format!("Remote Keystores are not supported.")));
	}

	let executor = <NativeElseWasmExecutor<Executor>>::new(
		config.wasm_method,
		config.default_heap_pages,
		config.max_runtime_instances,
	);
	let (client, backend, keystore_container, task_manager) =
		sc_service::new_full_parts::<Block, RuntimeApi, _>(config, None, executor)?;
	let client = Arc::new(client);
	let select_chain = sc_consensus::LongestChain::new(backend.clone());
	let transaction_pool = sc_transaction_pool::BasicPool::new_full(
		config.transaction_pool.clone(),
		config.role.is_authority().into(),
		config.prometheus_registry(),
		task_manager.spawn_essential_handle(),
		client.clone(),
	);
	let filter_pool: Option<FilterPool> = Some(Arc::new(Mutex::new(BTreeMap::new())));
	let frontier_backend = dvm::open_backend(config)?;
	let import_queue = sc_consensus_manual_seal::import_queue(
		Box::new(client.clone()),
		&task_manager.spawn_essential_handle(),
		config.prometheus_registry(),
	);

	Ok(sc_service::PartialComponents {
		client: client.clone(),
		backend,
		task_manager,
		import_queue,
		keystore_container,
		select_chain,
		transaction_pool,
		other: ((client, is_manual_sealing), filter_pool, frontier_backend),
	})
}

/// Builds a new service for a full client.
pub fn new_full(
	config: sc_service::Configuration,
	is_manual_sealing: bool,
	enable_dev_signer: bool,
	eth_rpc_config: drml_rpc::EthRpcConfig,
) -> ServiceResult<sc_service::TaskManager> {
	// --- std ---
	use std::{collections::BTreeMap, sync::Mutex};
	// --- paritytech ---
	use fc_rpc::EthBlockDataCache;
	use fc_rpc_core::types::FeeHistoryCache;
	use manual_seal::{InstantSealParams, ManualSealParams};
	use sc_consensus_manual_seal as manual_seal;
	use sc_service::{BuildNetworkParams, SpawnTasksParams};
	// --- darwinia-network ---
	use drml_rpc::{template::FullDeps, *};

	let sc_service::PartialComponents {
		client,
		backend,
		mut task_manager,
		import_queue,
		keystore_container,
		select_chain,
		transaction_pool,
		other: (consensus_result, filter_pool, frontier_backend),
	} = new_partial(&config, is_manual_sealing)?;
	let (network, system_rpc_tx, network_starter) =
		sc_service::build_network(BuildNetworkParams {
			config: &config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			on_demand: None,
			block_announce_validator_builder: None,
			warp_sync: None,
		})?;
	// Channel for the rpc handler to communicate with the authorship task.
	let (command_sink, commands_stream) = futures::channel::mpsc::channel(1000);

	if config.offchain_worker.enabled {
		sc_service::build_offchain_workers(
			&config,
			task_manager.spawn_handle(),
			client.clone(),
			network.clone(),
		);
	}

	let is_archive = config.state_pruning.is_archive();
	let overrides = drml_rpc::overrides_handle(client.clone());
	let block_data_cache =
		Arc::new(EthBlockDataCache::new(task_manager.spawn_handle(), overrides.clone(), 50, 50));
	let fee_history_cache: FeeHistoryCache = Arc::new(Mutex::new(BTreeMap::new()));
	let eth_rpc_requesters = DvmTaskParams {
		task_manager: &task_manager,
		client: client.clone(),
		substrate_backend: backend.clone(),
		dvm_backend: frontier_backend.clone(),
		filter_pool: filter_pool.clone(),
		is_archive,
		rpc_config: eth_rpc_config.clone(),
		fee_history_cache: fee_history_cache.clone(),
		overrides: overrides.clone(),
	}
	.spawn_task("Template");
	let role = config.role.clone();
	let prometheus_registry = config.prometheus_registry().cloned();
	let is_authority = config.role.is_authority();
	let subscription_task_executor = SubscriptionTaskExecutor::new(task_manager.spawn_handle());
	let rpc_extensions_builder = {
		let client = client.clone();
		let pool = transaction_pool.clone();
		let network = network.clone();
		let filter_pool = filter_pool.clone();
		let frontier_backend = frontier_backend.clone();

		Box::new(move |deny_unsafe, _| {
			let deps = FullDeps {
				client: client.clone(),
				pool: pool.clone(),
				deny_unsafe,
				eth: EthDeps {
					config: eth_rpc_config.clone(),
					graph: pool.pool().clone(),
					is_authority,
					network: network.clone(),
					filter_pool: filter_pool.clone(),
					backend: frontier_backend.clone(),
					fee_history_cache: fee_history_cache.clone(),
					overrides: overrides.clone(),
					block_data_cache: block_data_cache.clone(),
					rpc_requesters: eth_rpc_requesters.clone(),
				},
				enable_dev_signer,
				command_sink: Some(command_sink.clone()),
			};

			Ok(drml_rpc::template::create_full(deps, subscription_task_executor.clone()))
		})
	};
	let _ = sc_service::spawn_tasks(SpawnTasksParams {
		network: network.clone(),
		client: client.clone(),
		keystore: keystore_container.sync_keystore(),
		task_manager: &mut task_manager,
		transaction_pool: transaction_pool.clone(),
		rpc_extensions_builder,
		on_demand: None,
		remote_blockchain: None,
		backend: backend.clone(),
		system_rpc_tx,
		config,
		telemetry: None,
	})?;
	let (block_import, is_manual_sealing) = consensus_result;

	if role.is_authority() {
		let env = sc_basic_authorship::ProposerFactory::new(
			task_manager.spawn_handle(),
			client.clone(),
			transaction_pool.clone(),
			prometheus_registry.as_ref(),
			None,
		);

		// Background authorship future
		if is_manual_sealing {
			let authorship_future = manual_seal::run_manual_seal(ManualSealParams {
				block_import,
				env,
				client,
				pool: transaction_pool.clone(),
				commands_stream,
				select_chain,
				consensus_data_provider: None,
				create_inherent_data_providers: move |_, ()| async move {
					let mock_timestamp = MockTimestampInherentDataProvider;

					Ok(mock_timestamp)
				},
			});
			// we spawn the future on a background thread managed by service.
			task_manager.spawn_essential_handle().spawn_blocking("manual-seal", authorship_future);
		} else {
			let authorship_future = manual_seal::run_instant_seal(InstantSealParams {
				block_import,
				env,
				client: client.clone(),
				pool: transaction_pool.clone(),
				select_chain,
				consensus_data_provider: None,
				create_inherent_data_providers: move |_, ()| async move {
					let mock_timestamp = MockTimestampInherentDataProvider;

					Ok(mock_timestamp)
				},
			});
			// we spawn the future on a background thread managed by service.
			task_manager.spawn_essential_handle().spawn_blocking("instant-seal", authorship_future);
		}

		log::info!("Manual Seal Ready");
	}

	network_starter.start_network();

	Ok(task_manager)
}
