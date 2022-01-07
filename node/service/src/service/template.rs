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
use std::{
	cell::RefCell,
	collections::BTreeMap,
	path::PathBuf,
	sync::{Arc, Mutex},
};
// --- crates.io ---
use async_trait::async_trait;
// --- paritytech ---
use dc_db::{Backend, DatabaseSettings, DatabaseSettingsSrc};
use fc_rpc_core::types::FilterPool;
use sc_consensus_manual_seal as manual_seal;
use sc_executor::{NativeElseWasmExecutor, NativeExecutionDispatch};
use sc_keystore::LocalKeystore;
use sc_service::{error::Error as ServiceError, BasePath, Configuration, TaskManager};
use sc_telemetry::{Telemetry, TelemetryWorker};
use sp_inherents::{InherentData, InherentDataProvider, InherentIdentifier};
// --- darwinia-network ---
use crate::service::{
	dvm_tasks::{self, DvmTasksParams},
	FullBackend, FullClient, FullSelectChain,
};
use drml_common_primitives::{OpaqueBlock as Block, SLOT_DURATION};
use drml_rpc::{template::FullDeps, RpcConfig, SubscriptionTaskExecutor};
use template_runtime::RuntimeApi;

thread_local!(static TIMESTAMP: RefCell<u64> = RefCell::new(0));

pub struct Executor;
impl NativeExecutionDispatch for Executor {
	type ExtendHostFunctions = ();

	fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
		template_runtime::api::dispatch(method, data)
	}

	fn native_version() -> sc_executor::NativeVersion {
		template_runtime::native_version()
	}
}

pub type ConsensusResult = (Arc<FullClient<RuntimeApi, Executor>>, bool);

pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"timstap0";

/// Provide a mock duration starting at 0 in millisecond for timestamp inherent.
/// Each call will increment timestamp by slot_duration making Aura think time has passed.
pub struct MockTimestampInherentDataProvider;
#[async_trait]
impl InherentDataProvider for MockTimestampInherentDataProvider {
	fn provide_inherent_data(
		&self,
		inherent_data: &mut InherentData,
	) -> Result<(), sp_inherents::Error> {
		TIMESTAMP.with(|x| {
			*x.borrow_mut() += SLOT_DURATION;
			inherent_data.put_data(INHERENT_IDENTIFIER, &*x.borrow())
		})
	}

	async fn try_handle_error(
		&self,
		_identifier: &InherentIdentifier,
		_error: &[u8],
	) -> Option<Result<(), sp_inherents::Error>> {
		// The pallet never reports error.
		None
	}
}

pub fn frontier_database_dir(config: &Configuration) -> PathBuf {
	let config_dir = config
		.base_path
		.as_ref()
		.map(|base_path| base_path.config_dir(config.chain_spec.id()))
		.unwrap_or_else(|| {
			BasePath::from_project("", "", "template").config_dir(config.chain_spec.id())
		});
	config_dir.join("frontier").join("db")
}

pub fn open_frontier_backend(config: &Configuration) -> Result<Arc<Backend<Block>>, String> {
	Ok(Arc::new(Backend::<Block>::new(&DatabaseSettings {
		source: DatabaseSettingsSrc::RocksDb {
			path: frontier_database_dir(&config),
			cache_size: 0,
		},
	})?))
}

pub fn new_partial(
	config: &Configuration,
	is_manual_sealing: bool,
) -> Result<
	sc_service::PartialComponents<
		FullClient<RuntimeApi, Executor>,
		FullBackend,
		FullSelectChain,
		sc_consensus::DefaultImportQueue<Block, FullClient<RuntimeApi, Executor>>,
		sc_transaction_pool::FullPool<Block, FullClient<RuntimeApi, Executor>>,
		(
			ConsensusResult,
			Option<FilterPool>,
			Arc<dc_db::Backend<Block>>,
			Option<Telemetry>,
		),
	>,
	ServiceError,
> {
	if config.keystore_remote.is_some() {
		return Err(ServiceError::Other(format!(
			"Remote Keystores are not supported."
		)));
	}

	let telemetry = config
		.telemetry_endpoints
		.clone()
		.filter(|x| !x.is_empty())
		.map(|endpoints| -> Result<_, sc_telemetry::Error> {
			let worker = TelemetryWorker::new(16)?;
			let telemetry = worker.handle().new_telemetry(endpoints);
			Ok((worker, telemetry))
		})
		.transpose()?;
	let executor = <NativeElseWasmExecutor<Executor>>::new(
		config.wasm_method,
		config.default_heap_pages,
		config.max_runtime_instances,
	);
	let (client, backend, keystore_container, task_manager) =
		sc_service::new_full_parts::<Block, RuntimeApi, _>(
			config,
			telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
			executor,
		)?;
	let client = Arc::new(client);
	let telemetry = telemetry.map(|(worker, telemetry)| {
		task_manager.spawn_handle().spawn("telemetry", worker.run());
		telemetry
	});
	let select_chain = sc_consensus::LongestChain::new(backend.clone());
	let transaction_pool = sc_transaction_pool::BasicPool::new_full(
		config.transaction_pool.clone(),
		config.role.is_authority().into(),
		config.prometheus_registry(),
		task_manager.spawn_essential_handle(),
		client.clone(),
	);
	let filter_pool: Option<FilterPool> = Some(Arc::new(Mutex::new(BTreeMap::new())));
	let frontier_backend = open_frontier_backend(config)?;
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
		other: (
			(client, is_manual_sealing),
			filter_pool,
			frontier_backend,
			telemetry,
		),
	})
}

fn remote_keystore(_url: &String) -> Result<Arc<LocalKeystore>, &'static str> {
	// FIXME: here would the concrete keystore be built,
	//        must return a concrete type (NOT `LocalKeystore`) that
	//        implements `CryptoStore` and `SyncCryptoStore`
	Err("Remote Keystore not supported.")
}

/// Builds a new service for a full client.
pub fn new_full(
	config: Configuration,
	is_manual_sealing: bool,
	enable_dev_signer: bool,
	rpc_config: RpcConfig,
) -> Result<TaskManager, ServiceError> {
	let sc_service::PartialComponents {
		client,
		backend,
		mut task_manager,
		import_queue,
		mut keystore_container,
		select_chain,
		transaction_pool,
		other: (consensus_result, filter_pool, frontier_backend, mut telemetry),
	} = new_partial(&config, is_manual_sealing)?;

	if let Some(url) = &config.keystore_remote {
		match remote_keystore(url) {
			Ok(k) => keystore_container.set_remote_keystore(k),
			Err(e) => {
				return Err(ServiceError::Other(format!(
					"Error hooking up remote keystore for {}: {}",
					url, e
				)))
			}
		};
	}

	let (network, system_rpc_tx, network_starter) =
		sc_service::build_network(sc_service::BuildNetworkParams {
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
	let tracing_requesters = dvm_tasks::spawn(DvmTasksParams {
		task_manager: &task_manager,
		client: client.clone(),
		substrate_backend: backend.clone(),
		dvm_backend: frontier_backend.clone(),
		filter_pool: filter_pool.clone(),
		rpc_config: rpc_config.clone(),
		is_archive,
	});
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
				graph: pool.pool().clone(),
				deny_unsafe,
				is_authority,
				enable_dev_signer,
				network: network.clone(),
				filter_pool: filter_pool.clone(),
				backend: frontier_backend.clone(),
				tracing_requesters: tracing_requesters.clone(),
				rpc_config: rpc_config.clone(),
				command_sink: Some(command_sink.clone()),
			};

			Ok(drml_rpc::template::create_full(
				deps,
				subscription_task_executor.clone(),
			))
		})
	};
	let _ = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
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
		telemetry: telemetry.as_mut(),
	})?;
	let (block_import, is_manual_sealing) = consensus_result;

	if role.is_authority() {
		let env = sc_basic_authorship::ProposerFactory::new(
			task_manager.spawn_handle(),
			client.clone(),
			transaction_pool.clone(),
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|x| x.handle()),
		);

		// Background authorship future
		if is_manual_sealing {
			let authorship_future = manual_seal::run_manual_seal(manual_seal::ManualSealParams {
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
			task_manager
				.spawn_essential_handle()
				.spawn_blocking("manual-seal", authorship_future);
		} else {
			let authorship_future = manual_seal::run_instant_seal(manual_seal::InstantSealParams {
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
			task_manager
				.spawn_essential_handle()
				.spawn_blocking("instant-seal", authorship_future);
		}

		log::info!("Manual Seal Ready");
	}

	network_starter.start_network();

	Ok(task_manager)
}
