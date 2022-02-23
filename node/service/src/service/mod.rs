macro_rules! impl_runtime_apis {
	($api:path,$($extra_apis:path),*;$($bound:tt)*) => {
		/// A set of APIs that darwinia-like runtimes must implement.
		pub trait RuntimeApiCollection:
			$api
			$(+ $extra_apis)*
		where
			$($bound)*
		{
		}
		impl<Api> RuntimeApiCollection for Api
		where
			Api: $api
				$(+ $extra_apis)*,
			$($bound)*
		{
		}
	};
}

impl_runtime_apis![
	sp_api::ApiExt<Block>,
	sp_api::Metadata<Block>,
	sp_block_builder::BlockBuilder<Block>,
	sp_session::SessionKeys<Block>,
	sp_consensus_babe::BabeApi<Block>,
	sp_finality_grandpa::GrandpaApi<Block>,
	beefy_primitives::BeefyApi<Block>,
	sp_authority_discovery::AuthorityDiscoveryApi<Block>,
	sp_offchain::OffchainWorkerApi<Block>,
	sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>,
	frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce>,
	pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance>,
	darwinia_balances_rpc_runtime_api::BalancesApi<Block, AccountId, Balance>,
	darwinia_staking_rpc_runtime_api::StakingApi<Block, AccountId, Power>,
	darwinia_fee_market_rpc_runtime_api::FeeMarketApi<Block, Balance>,
	dvm_rpc_runtime_api::EthereumRuntimeRPCApi<Block>,
	dp_evm_trace_apis::DebugRuntimeApi<Block>;
	<Self as sp_api::ApiExt<Block>>::StateBackend: sp_api::StateBackend<BlakeTwo256>,
];

pub mod dvm_tasks;

pub mod pangolin;
pub use pangolin::Executor as PangolinExecutor;

pub mod pangoro;
pub use pangoro::Executor as PangoroExecutor;

#[cfg(feature = "template")]
pub mod template;
#[cfg(feature = "template")]
pub use template::Executor as TemplateExecutor;

// --- std ---
use std::{
	collections::BTreeMap,
	path::PathBuf,
	sync::{Arc, Mutex},
	time::Duration,
};
// --- crates.io ---
use codec::Codec;
use futures::stream::StreamExt;
// --- paritytech ---
use beefy_gadget::{notification::BeefySignedCommitmentStream, BeefyParams};
use fc_rpc_core::types::FilterPool;
use sc_authority_discovery::WorkerConfig;
use sc_basic_authorship::ProposerFactory;
use sc_client_api::{ExecutorProvider, RemoteBackend, StateBackendFor};
use sc_consensus::{BasicQueue, DefaultImportQueue, LongestChain};
use sc_consensus_babe::{
	BabeBlockImport, BabeLink, BabeParams, Config as BabeConfig, SlotProportion,
};
use sc_executor::{NativeElseWasmExecutor, NativeExecutionDispatch};
use sc_finality_grandpa::{
	warp_proof::NetworkProvider, Config as GrandpaConfig,
	FinalityProofProvider as GrandpaFinalityProofProvider, GrandpaBlockImport, GrandpaParams,
	LinkHalf, SharedVoterState as GrandpaSharedVoterState,
	VotingRulesBuilder as GrandpaVotingRulesBuilder,
};
use sc_keystore::LocalKeystore;
use sc_network::Event;
use sc_service::{
	config::{KeystoreConfig, PrometheusConfig},
	BuildNetworkParams, ChainSpec, Configuration, Error as ServiceError, NoopRpcExtensionBuilder,
	PartialComponents, RpcHandlers, SpawnTasksParams, TFullBackend, TFullClient,
	TLightBackendWithHash, TLightClientWithBackend, TaskManager,
};
use sc_telemetry::{Telemetry, TelemetryWorker};
use sc_transaction_pool::{BasicPool, FullPool};
use sp_api::ConstructRuntimeApi;
use sp_consensus::{CanAuthorWithNativeVersion, NeverCanAuthor};
use sp_runtime::traits::{BlakeTwo256, Block as BlockT};
use sp_trie::PrefixedMemoryDB;
use substrate_prometheus_endpoint::Registry;
// --- darwinia-network ---
use crate::service::dvm_tasks::DvmTasksParams;
use dc_db::{Backend, DatabaseSettings, DatabaseSettingsSrc};
use drml_common_primitives::{AccountId, Balance, Nonce, OpaqueBlock as Block, Power};
use drml_rpc::{
	BabeDeps, BeefyDeps, FullDeps, GrandpaDeps, LightDeps, RpcConfig, RpcExtension,
	SubscriptionTaskExecutor,
};

type FullBackend = TFullBackend<Block>;
type FullSelectChain = LongestChain<FullBackend, Block>;
type FullClient<RuntimeApi, Executor> =
	TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>;
type FullGrandpaBlockImport<RuntimeApi, Executor> =
	GrandpaBlockImport<FullBackend, Block, FullClient<RuntimeApi, Executor>, FullSelectChain>;
type LightBackend = TLightBackendWithHash<Block, BlakeTwo256>;
type LightClient<RuntimeApi, Executor> =
	TLightClientWithBackend<Block, RuntimeApi, NativeElseWasmExecutor<Executor>, LightBackend>;
type RpcResult = Result<RpcExtension, ServiceError>;

pub trait RuntimeExtrinsic: 'static + Send + Sync + Codec {}
impl<E> RuntimeExtrinsic for E where E: 'static + Send + Sync + Codec {}

/// Can be called for a `Configuration` to check the network type.
pub trait IdentifyVariant {
	/// Returns if this is a configuration for the `Pangolin` network.
	fn is_pangolin(&self) -> bool;

	/// Returns if this is a configuration for the `Pangoro` network.
	fn is_pangoro(&self) -> bool;

	/// Returns if this is a configuration for the `Template` network.
	#[cfg(feature = "template")]
	fn is_template(&self) -> bool;

	/// Returns true if this configuration is for a development network.
	fn is_dev(&self) -> bool;
}
impl IdentifyVariant for Box<dyn ChainSpec> {
	fn is_pangolin(&self) -> bool {
		self.id().starts_with("pangolin")
	}

	fn is_pangoro(&self) -> bool {
		self.id().starts_with("pangoro")
	}

	#[cfg(feature = "template")]
	fn is_template(&self) -> bool {
		self.id().starts_with("template")
	}

	fn is_dev(&self) -> bool {
		self.id().ends_with("dev")
	}
}

/// Builds a new object suitable for chain operations.
#[cfg(feature = "full-node")]
pub fn new_chain_ops<Runtime, Dispatch>(
	config: &mut Configuration,
) -> Result<
	(
		Arc<FullClient<Runtime, Dispatch>>,
		Arc<FullBackend>,
		BasicQueue<Block, PrefixedMemoryDB<BlakeTwo256>>,
		TaskManager,
	),
	ServiceError,
>
where
	Dispatch: 'static + NativeExecutionDispatch,
	Runtime: 'static + Send + Sync + ConstructRuntimeApi<Block, FullClient<Runtime, Dispatch>>,
	Runtime::RuntimeApi: RuntimeApiCollection<StateBackend = StateBackendFor<FullBackend, Block>>,
{
	config.keystore = KeystoreConfig::InMemory;

	let PartialComponents {
		client,
		backend,
		import_queue,
		task_manager,
		..
	} = new_partial::<Runtime, Dispatch>(config)?;

	Ok((client, backend, import_queue, task_manager))
}

#[cfg(feature = "full-node")]
fn new_partial<RuntimeApi, Executor>(
	config: &mut Configuration,
) -> Result<
	PartialComponents<
		FullClient<RuntimeApi, Executor>,
		FullBackend,
		FullSelectChain,
		DefaultImportQueue<Block, FullClient<RuntimeApi, Executor>>,
		FullPool<Block, FullClient<RuntimeApi, Executor>>,
		(
			(
				BabeBlockImport<
					Block,
					FullClient<RuntimeApi, Executor>,
					FullGrandpaBlockImport<RuntimeApi, Executor>,
				>,
				LinkHalf<Block, FullClient<RuntimeApi, Executor>, FullSelectChain>,
				BabeLink<Block>,
			),
			Option<Telemetry>,
		),
	>,
	ServiceError,
>
where
	Executor: 'static + NativeExecutionDispatch,
	RuntimeApi:
		'static + Send + Sync + ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>>,
	RuntimeApi::RuntimeApi:
		RuntimeApiCollection<StateBackend = StateBackendFor<FullBackend, Block>>,
{
	if config.keystore_remote.is_some() {
		return Err(ServiceError::Other(format!(
			"Remote Keystores are not supported."
		)));
	}

	set_prometheus_registry(config)?;

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
	let telemetry = telemetry.map(|(worker, telemetry)| {
		task_manager.spawn_handle().spawn("telemetry", worker.run());
		telemetry
	});
	let client = Arc::new(client);
	let select_chain = LongestChain::new(backend.clone());
	let transaction_pool = BasicPool::new_full(
		config.transaction_pool.clone(),
		config.role.is_authority().into(),
		config.prometheus_registry(),
		task_manager.spawn_essential_handle(),
		client.clone(),
	);
	let grandpa_hard_forks = vec![];
	let (grandpa_block_import, grandpa_link) =
		sc_finality_grandpa::block_import_with_authority_set_hard_forks(
			client.clone(),
			&(client.clone() as Arc<_>),
			select_chain.clone(),
			grandpa_hard_forks,
			telemetry.as_ref().map(|x| x.handle()),
		)?;
	let justification_import = grandpa_block_import.clone();
	let (babe_import, babe_link) = sc_consensus_babe::block_import(
		BabeConfig::get_or_compute(&*client)?,
		grandpa_block_import,
		client.clone(),
	)?;
	let slot_duration = babe_link.config().slot_duration();
	let import_queue = sc_consensus_babe::import_queue(
		babe_link.clone(),
		babe_import.clone(),
		Some(Box::new(justification_import)),
		client.clone(),
		select_chain.clone(),
		move |_, ()| async move {
			let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
			let slot =
				sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_duration(
					*timestamp,
					slot_duration,
				);
			let uncles =
				sp_authorship::InherentDataProvider::<<Block as BlockT>::Header>::check_inherents();

			Ok((timestamp, slot, uncles))
		},
		&task_manager.spawn_essential_handle(),
		config.prometheus_registry(),
		CanAuthorWithNativeVersion::new(client.executor().clone()),
		telemetry.as_ref().map(|x| x.handle()),
	)?;
	let import_setup = (babe_import.clone(), grandpa_link, babe_link.clone());

	Ok(PartialComponents {
		client,
		backend,
		task_manager,
		keystore_container,
		select_chain,
		import_queue,
		transaction_pool,
		other: (import_setup, telemetry),
	})
}

#[cfg(feature = "full-node")]
fn new_full<RuntimeApi, Executor, CT>(
	mut config: Configuration,
	authority_discovery_disabled: bool,
	rpc_config: RpcConfig,
	eth_transaction_convertor: CT,
) -> Result<
	(
		TaskManager,
		Arc<FullClient<RuntimeApi, Executor>>,
		RpcHandlers,
	),
	ServiceError,
>
where
	Executor: 'static + NativeExecutionDispatch,
	RuntimeApi:
		'static + Send + Sync + ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>>,
	RuntimeApi::RuntimeApi:
		RuntimeApiCollection<StateBackend = StateBackendFor<FullBackend, Block>>,
	CT: 'static
		+ Clone
		+ Send
		+ Sync
		+ dvm_rpc_runtime_api::ConvertTransaction<sp_runtime::OpaqueExtrinsic>,
{
	let role = config.role.clone();
	let is_authority = role.is_authority();
	let is_archive = config.state_pruning.is_archive();
	let force_authoring = config.force_authoring;
	let disable_grandpa = config.disable_grandpa;
	let name = config.network.node_name.clone();
	let prometheus_registry = config.prometheus_registry().cloned();
	let auth_disc_publish_non_global_ips = config.network.allow_non_globals_in_dht;

	config
		.network
		.extra_sets
		.push(sc_finality_grandpa::grandpa_peers_set_config());
	config
		.network
		.extra_sets
		.push(beefy_gadget::beefy_peers_set_config());

	let backoff_authoring_blocks =
		Some(sc_consensus_slots::BackoffAuthoringOnFinalizedHeadLagging::default());
	let PartialComponents {
		client,
		backend,
		mut task_manager,
		mut keystore_container,
		select_chain,
		import_queue,
		transaction_pool,
		other: ((babe_import, grandpa_link, babe_link), mut telemetry),
	} = new_partial::<RuntimeApi, Executor>(&mut config)?;

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

	let warp_sync = Arc::new(NetworkProvider::new(
		backend.clone(),
		grandpa_link.shared_authority_set().clone(),
	));
	let (network, system_rpc_tx, network_starter) =
		sc_service::build_network(BuildNetworkParams {
			config: &config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			on_demand: None,
			block_announce_validator_builder: None,
			warp_sync: Some(warp_sync),
		})?;

	if config.offchain_worker.enabled {
		sc_service::build_offchain_workers(
			&config,
			task_manager.spawn_handle(),
			client.clone(),
			network.clone(),
		);
	}

	let dvm_backend = open_dvm_backend(&config)?;
	let filter_pool: Option<FilterPool> = Some(Arc::new(Mutex::new(BTreeMap::new())));
	let tracing_requesters = dvm_tasks::spawn(DvmTasksParams {
		task_manager: &task_manager,
		client: client.clone(),
		substrate_backend: backend.clone(),
		dvm_backend: dvm_backend.clone(),
		filter_pool: filter_pool.clone(),
		is_archive,
		rpc_config: rpc_config.clone(),
	});
	let subscription_task_executor = SubscriptionTaskExecutor::new(task_manager.spawn_handle());
	let shared_voter_state = GrandpaSharedVoterState::empty();
	let babe_config = babe_link.config().clone();
	let shared_epoch_changes = babe_link.epoch_changes().clone();
	let justification_stream = grandpa_link.justification_stream();
	let (beefy_link, beefy_commitment_stream) = BeefySignedCommitmentStream::channel();
	let shared_authority_set = grandpa_link.shared_authority_set().clone();
	let finality_proof_provider = GrandpaFinalityProofProvider::new_for_service(
		backend.clone(),
		Some(shared_authority_set.clone()),
	);
	let rpc_extensions_builder = {
		let client = client.clone();
		let keystore = keystore_container.sync_keystore();
		let transaction_pool = transaction_pool.clone();
		let select_chain = select_chain.clone();
		let chain_spec = config.chain_spec.cloned_box();
		let shared_voter_state = shared_voter_state.clone();
		let network = network.clone();

		Box::new(
			move |deny_unsafe, subscription_executor: SubscriptionTaskExecutor| -> RpcResult {
				drml_rpc::create_full(
					FullDeps {
						client: client.clone(),
						pool: transaction_pool.clone(),
						graph: transaction_pool.pool().clone(),
						select_chain: select_chain.clone(),
						chain_spec: chain_spec.cloned_box(),
						deny_unsafe,
						is_authority,
						network: network.clone(),
						babe: BabeDeps {
							babe_config: babe_config.clone(),
							shared_epoch_changes: shared_epoch_changes.clone(),
							keystore: keystore.clone(),
						},
						grandpa: GrandpaDeps {
							shared_voter_state: shared_voter_state.clone(),
							shared_authority_set: shared_authority_set.clone(),
							justification_stream: justification_stream.clone(),
							subscription_executor: subscription_executor.clone(),
							finality_proof_provider: finality_proof_provider.clone(),
						},
						beefy: BeefyDeps {
							beefy_commitment_stream: beefy_commitment_stream.clone(),
							subscription_executor,
						},
						backend: dvm_backend.clone(),
						filter_pool: filter_pool.clone(),
						tracing_requesters: tracing_requesters.clone(),
						rpc_config: rpc_config.clone(),
					},
					subscription_task_executor.clone(),
					eth_transaction_convertor.clone(),
				)
				.map_err(Into::into)
			},
		)
	};
	let rpc_handlers = sc_service::spawn_tasks(SpawnTasksParams {
		config,
		backend: backend.clone(),
		client: client.clone(),
		keystore: keystore_container.sync_keystore(),
		network: network.clone(),
		rpc_extensions_builder,
		transaction_pool: transaction_pool.clone(),
		task_manager: &mut task_manager,
		on_demand: None,
		remote_blockchain: None,
		system_rpc_tx,
		telemetry: telemetry.as_mut(),
	})?;

	if is_authority {
		let can_author_with = CanAuthorWithNativeVersion::new(client.executor().clone());
		let proposer = ProposerFactory::new(
			task_manager.spawn_handle(),
			client.clone(),
			transaction_pool,
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|x| x.handle()),
		);
		let client_clone = client.clone();
		let slot_duration = babe_link.config().slot_duration();
		let babe_config = BabeParams {
			keystore: keystore_container.sync_keystore(),
			client: client.clone(),
			select_chain,
			block_import: babe_import,
			env: proposer,
			sync_oracle: network.clone(),
			justification_sync_link: network.clone(),
			create_inherent_data_providers: move |parent, ()| {
				let client_clone = client_clone.clone();
				async move {
					let uncles = sc_consensus_uncles::create_uncles_inherent_data_provider(
						&*client_clone,
						parent,
					)?;
					let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
					let slot =
						sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_duration(
							*timestamp,
							slot_duration,
						);

					Ok((timestamp, slot, uncles))
				}
			},
			force_authoring,
			backoff_authoring_blocks,
			babe_link,
			can_author_with,
			block_proposal_slot_portion: SlotProportion::new(2f32 / 3f32),
			max_block_proposal_slot_portion: None,
			telemetry: telemetry.as_ref().map(|x| x.handle()),
		};
		let babe = sc_consensus_babe::start_babe(babe_config)?;

		task_manager
			.spawn_essential_handle()
			.spawn_blocking("babe", babe);
	}

	if is_authority && !authority_discovery_disabled {
		let authority_discovery_role =
			sc_authority_discovery::Role::PublishAndDiscover(keystore_container.keystore());
		let dht_event_stream =
			network
				.event_stream("authority-discovery")
				.filter_map(|e| async move {
					match e {
						Event::Dht(e) => Some(e),
						_ => None,
					}
				});
		let (authority_discovery_worker, _service) =
			sc_authority_discovery::new_worker_and_service_with_config(
				WorkerConfig {
					publish_non_global_ips: auth_disc_publish_non_global_ips,
					..Default::default()
				},
				client.clone(),
				network.clone(),
				Box::pin(dht_event_stream),
				authority_discovery_role,
				prometheus_registry.clone(),
			);

		task_manager.spawn_handle().spawn(
			"authority-discovery-worker",
			authority_discovery_worker.run(),
		);
	}

	let keystore = if is_authority {
		Some(keystore_container.sync_keystore())
	} else {
		None
	};

	task_manager.spawn_essential_handle().spawn_blocking(
		"beefy-gadget",
		beefy_gadget::start_beefy_gadget::<_, _, _, _>(BeefyParams {
			client: client.clone(),
			backend: backend.clone(),
			key_store: keystore.clone(),
			network: network.clone(),
			signed_commitment_sender: beefy_link,
			min_block_delta: 4,
			prometheus_registry: prometheus_registry.clone(),
		}),
	);

	if !disable_grandpa {
		let grandpa_config = GrandpaParams {
			config: GrandpaConfig {
				// FIXME substrate#1578 make this available through chainspec
				gossip_duration: Duration::from_millis(1000),
				justification_period: 512,
				name: Some(name),
				observer_enabled: false,
				keystore,
				local_role: role,
				telemetry: telemetry.as_ref().map(|x| x.handle()),
			},
			link: grandpa_link,
			network,
			telemetry: telemetry.as_ref().map(|x| x.handle()),
			voting_rule: GrandpaVotingRulesBuilder::default().build(),
			prometheus_registry,
			shared_voter_state,
		};

		task_manager.spawn_essential_handle().spawn_blocking(
			"grandpa-voter",
			sc_finality_grandpa::run_grandpa_voter(grandpa_config)?,
		);
	}

	network_starter.start_network();

	Ok((task_manager, client, rpc_handlers))
}

fn new_light<RuntimeApi, Executor>(
	mut config: Configuration,
) -> Result<(TaskManager, RpcHandlers), ServiceError>
where
	Executor: 'static + NativeExecutionDispatch,
	RuntimeApi:
		'static + Send + Sync + ConstructRuntimeApi<Block, LightClient<RuntimeApi, Executor>>,
	<RuntimeApi as ConstructRuntimeApi<Block, LightClient<RuntimeApi, Executor>>>::RuntimeApi:
		RuntimeApiCollection<StateBackend = StateBackendFor<LightBackend, Block>>,
{
	set_prometheus_registry(&mut config)?;

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
	let (client, backend, keystore_container, mut task_manager, on_demand) =
		sc_service::new_light_parts::<Block, RuntimeApi, _>(
			&config,
			telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
			executor,
		)?;
	let mut telemetry = telemetry.map(|(worker, telemetry)| {
		task_manager.spawn_handle().spawn("telemetry", worker.run());
		telemetry
	});

	config
		.network
		.extra_sets
		.push(sc_finality_grandpa::grandpa_peers_set_config());
	config
		.network
		.extra_sets
		.push(beefy_gadget::beefy_peers_set_config());

	let select_chain = LongestChain::new(backend.clone());
	let transaction_pool = Arc::new(BasicPool::new_light(
		config.transaction_pool.clone(),
		config.prometheus_registry(),
		task_manager.spawn_essential_handle(),
		client.clone(),
		on_demand.clone(),
	));
	let (grandpa_block_import, grandpa_link) = sc_finality_grandpa::block_import(
		client.clone(),
		&(client.clone() as Arc<_>),
		select_chain.clone(),
		telemetry.as_ref().map(|x| x.handle()),
	)?;
	let justification_import = grandpa_block_import.clone();
	let (babe_block_import, babe_link) = sc_consensus_babe::block_import(
		BabeConfig::get_or_compute(&*client)?,
		grandpa_block_import,
		client.clone(),
	)?;
	// FIXME: pruning task isn't started since light client doesn't do `AuthoritySetup`.
	let slot_duration = babe_link.config().slot_duration();
	let import_queue = sc_consensus_babe::import_queue(
		babe_link,
		babe_block_import,
		Some(Box::new(justification_import)),
		client.clone(),
		select_chain,
		move |_, ()| async move {
			let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
			let slot =
				sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_duration(
					*timestamp,
					slot_duration,
				);
			let uncles =
				sp_authorship::InherentDataProvider::<<Block as BlockT>::Header>::check_inherents();

			Ok((timestamp, slot, uncles))
		},
		&task_manager.spawn_essential_handle(),
		config.prometheus_registry(),
		NeverCanAuthor,
		telemetry.as_ref().map(|x| x.handle()),
	)?;
	let warp_sync = Arc::new(NetworkProvider::new(
		backend.clone(),
		grandpa_link.shared_authority_set().clone(),
	));
	let (network, system_rpc_tx, network_starter) =
		sc_service::build_network(BuildNetworkParams {
			config: &config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			on_demand: Some(on_demand.clone()),
			block_announce_validator_builder: None,
			warp_sync: Some(warp_sync),
		})?;
	let enable_grandpa = !config.disable_grandpa;

	if enable_grandpa {
		let name = config.network.node_name.clone();

		let config = sc_finality_grandpa::Config {
			gossip_duration: Duration::from_millis(1000),
			justification_period: 512,
			name: Some(name),
			observer_enabled: false,
			keystore: None,
			local_role: config.role.clone(),
			telemetry: telemetry.as_ref().map(|x| x.handle()),
		};

		task_manager.spawn_handle().spawn_blocking(
			"grandpa-observer",
			sc_finality_grandpa::run_grandpa_observer(config, grandpa_link, network.clone())?,
		);
	}

	if config.offchain_worker.enabled {
		sc_service::build_offchain_workers(
			&config,
			task_manager.spawn_handle(),
			client.clone(),
			network.clone(),
		);
	}

	let light_deps = LightDeps {
		remote_blockchain: backend.remote_blockchain(),
		fetcher: on_demand.clone(),
		client: client.clone(),
		pool: transaction_pool.clone(),
	};
	let rpc_extension = drml_rpc::create_light(light_deps);
	let rpc_handlers = sc_service::spawn_tasks(SpawnTasksParams {
		on_demand: Some(on_demand),
		remote_blockchain: Some(backend.remote_blockchain()),
		rpc_extensions_builder: Box::new(NoopRpcExtensionBuilder(rpc_extension)),
		task_manager: &mut task_manager,
		config,
		keystore: keystore_container.sync_keystore(),
		backend,
		transaction_pool,
		client,
		network,
		system_rpc_tx,
		telemetry: telemetry.as_mut(),
	})?;

	network_starter.start_network();

	Ok((task_manager, rpc_handlers))
}

// If we're using prometheus, use a registry with a prefix of `drml`.
fn set_prometheus_registry(config: &mut Configuration) -> Result<(), ServiceError> {
	if let Some(PrometheusConfig { registry, .. }) = config.prometheus_config.as_mut() {
		*registry = Registry::new_custom(Some("drml".into()), None)?;
	}

	Ok(())
}

fn remote_keystore(_url: &String) -> Result<Arc<LocalKeystore>, &'static str> {
	// FIXME: here would the concrete keystore be built,
	//        must return a concrete type (NOT `LocalKeystore`) that
	//        implements `CryptoStore` and `SyncCryptoStore`
	Err("Remote Keystore not supported.")
}

pub fn dvm_database_dir(config: &Configuration) -> PathBuf {
	let chain_id = config.chain_spec.id();
	let config_dir = config
		.base_path
		.as_ref()
		.map(|base_path| base_path.config_dir(chain_id))
		.expect("Config dir must be set.");

	config_dir.join("dvm").join("db")
}

fn open_dvm_backend(config: &Configuration) -> Result<Arc<Backend<Block>>, String> {
	Ok(Arc::new(Backend::<Block>::new(&DatabaseSettings {
		source: DatabaseSettingsSrc::RocksDb {
			path: dvm_database_dir(&config),
			cache_size: 0,
		},
	})?))
}
