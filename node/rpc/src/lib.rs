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

#[cfg(feature = "template")]
pub mod template;

pub use sc_rpc::{DenyUnsafe, SubscriptionTaskExecutor};

// --- std ---
use std::{collections::BTreeMap, error::Error, str::FromStr, sync::Arc};
// --- crates.io ---
use jsonrpc_core::IoHandler;
// --- paritytech ---
use sc_rpc::Metadata;
// --- darwinia-network ---
use dc_rpc::{CacheRequester as TraceFilterCacheRequester, DebugRequester, EthBlockDataCache};
use drml_common_primitives::{
	AccountId, Balance, BlockNumber, Hash, Hashing, Nonce, OpaqueBlock as Block, Power,
};
use dvm_ethereum::EthereumStorageSchema;

/// A type representing all RPC extensions.
pub type RpcExtension = IoHandler<Metadata>;
/// RPC result.
pub type RpcResult = Result<RpcExtension, Box<dyn Error + Send + Sync>>;

/// Full client dependencies.
pub struct FullDeps<C, P, SC, B, A>
where
	A: sc_transaction_pool::ChainApi,
{
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
	/// Graph pool instance.
	pub graph: Arc<sc_transaction_pool::Pool<A>>,
	/// The SelectChain Strategy
	pub select_chain: SC,
	/// A copy of the chain spec.
	pub chain_spec: Box<dyn sc_chain_spec::ChainSpec>,
	/// Whether to deny unsafe calls
	pub deny_unsafe: DenyUnsafe,
	/// BABE specific dependencies.
	pub babe: BabeDeps,
	/// GRANDPA specific dependencies.
	pub grandpa: GrandpaDeps<B>,
	/// The Node authority flag
	pub is_authority: bool,
	/// Network service
	pub network: Arc<sc_network::NetworkService<Block, Hash>>,
	/// EthFilterApi pool.
	pub filter_pool: Option<fc_rpc_core::types::FilterPool>,
	/// Backend.
	pub backend: Arc<dc_db::Backend<Block>>,
	/// Rpc requester for evm trace
	pub tracing_requesters: RpcRequesters,
	/// Rpc Config
	pub rpc_config: RpcConfig,
}

/// Light client extra dependencies.
pub struct LightDeps<C, F, P> {
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
	/// Remote access to the blockchain (async).
	pub remote_blockchain: Arc<dyn sc_client_api::RemoteBlockchain<Block>>,
	/// Fetcher instance.
	pub fetcher: Arc<F>,
}

/// Extra dependencies for BABE.
pub struct BabeDeps {
	/// BABE protocol config.
	pub babe_config: sc_consensus_babe::Config,
	/// BABE pending epoch changes.
	pub shared_epoch_changes:
		sc_consensus_epochs::SharedEpochChanges<Block, sc_consensus_babe::Epoch>,
	/// The keystore that manages the keys of the node.
	pub keystore: sp_keystore::SyncCryptoStorePtr,
}

/// Dependencies for GRANDPA
pub struct GrandpaDeps<B> {
	/// Voting round info.
	pub shared_voter_state: sc_finality_grandpa::SharedVoterState,
	/// Authority set info.
	pub shared_authority_set: sc_finality_grandpa::SharedAuthoritySet<Hash, BlockNumber>,
	/// Receives notifications about justification events from Grandpa.
	pub justification_stream: sc_finality_grandpa::GrandpaJustificationStream<Block>,
	/// Executor to drive the subscription manager in the Grandpa RPC handler.
	pub subscription_executor: sc_rpc::SubscriptionTaskExecutor,
	/// Finality proof provider.
	pub finality_provider: Arc<sc_finality_grandpa::FinalityProofProvider<B, Block>>,
}

#[derive(Clone)]
pub struct RpcRequesters {
	pub debug: Option<DebugRequester>,
	pub trace: Option<TraceFilterCacheRequester>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RpcConfig {
	pub ethapi: Vec<EthApiCmd>,
	pub ethapi_max_permits: u32,
	pub ethapi_trace_max_count: u32,
	pub ethapi_trace_cache_duration: u64,
	pub eth_log_block_cache: usize,
	pub max_past_logs: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EthApiCmd {
	Debug,
	Trace,
}
impl FromStr for EthApiCmd {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(match s {
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

/// Instantiate all RPC extensions.
pub fn create_full<C, P, SC, B, A, CT>(
	deps: FullDeps<C, P, SC, B, A>,
	subscription_task_executor: SubscriptionTaskExecutor,
	eth_transaction_convertor: CT,
) -> RpcResult
where
	C: 'static
		+ Send
		+ Sync
		+ sc_client_api::AuxStore
		+ sc_client_api::BlockchainEvents<Block>
		+ sc_client_api::StorageProvider<Block, B>
		+ sp_api::ProvideRuntimeApi<Block>
		+ sp_blockchain::HeaderBackend<Block>
		+ sp_blockchain::HeaderMetadata<Block, Error = sp_blockchain::Error>,
	C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>,
	C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
	C::Api: sc_consensus_babe::BabeApi<Block>,
	C::Api: sp_block_builder::BlockBuilder<Block>,
	C::Api: darwinia_balances_rpc::BalancesRuntimeApi<Block, AccountId, Balance>,
	C::Api: darwinia_staking_rpc::StakingRuntimeApi<Block, AccountId, Power>,
	C::Api: darwinia_fee_market_rpc::FeeMarketRuntimeApi<Block, Balance>,
	C::Api: dp_evm_trace_apis::DebugRuntimeApi<Block>,
	C::Api: dvm_rpc_runtime_api::EthereumRuntimeRPCApi<Block>,
	P: 'static + Sync + Send + sc_transaction_pool_api::TransactionPool<Block = Block>,
	SC: 'static + sp_consensus::SelectChain<Block>,
	B: 'static + Send + Sync + sc_client_api::Backend<Block>,
	B::State: sc_client_api::StateBackend<Hashing>,
	A: sc_transaction_pool::ChainApi<Block = Block> + 'static,
	CT: 'static
		+ Send
		+ Sync
		+ dvm_rpc_runtime_api::ConvertTransaction<sp_runtime::OpaqueExtrinsic>,
{
	// --- crates.io ---
	use jsonrpc_pubsub::manager::SubscriptionManager;
	// --- paritytech ---
	use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApi};
	use sc_consensus_babe_rpc::{BabeApi, BabeRpcHandler};
	use sc_finality_grandpa_rpc::{GrandpaApi, GrandpaRpcHandler};
	use sc_sync_state_rpc::{SyncStateRpcApi, SyncStateRpcHandler};
	use substrate_frame_rpc_system::{FullSystem, SystemApi};
	// --- darwinia-network ---
	use darwinia_balances_rpc::{Balances, BalancesApi};
	use darwinia_fee_market_rpc::{FeeMarket, FeeMarketApi};
	use darwinia_staking_rpc::{Staking, StakingApi};
	use dc_rpc::{
		Debug, DebugApiServer, EthApi, EthApiServer, EthFilterApi, EthFilterApiServer,
		EthPubSubApi, EthPubSubApiServer, HexEncodedIdProvider, NetApi, NetApiServer,
		OverrideHandle, RuntimeApiStorageOverride, SchemaV1Override, StorageOverride, Trace,
		TraceApiServer, Web3Api, Web3ApiServer,
	};

	let FullDeps {
		client,
		pool,
		graph,
		select_chain,
		chain_spec,
		deny_unsafe,
		babe,
		grandpa,
		is_authority,
		network,
		filter_pool,
		backend,
		tracing_requesters,
		rpc_config,
	} = deps;
	let mut io = jsonrpc_core::IoHandler::default();

	io.extend_with(SystemApi::to_delegate(FullSystem::new(
		client.clone(),
		pool.clone(),
		deny_unsafe,
	)));
	io.extend_with(TransactionPaymentApi::to_delegate(TransactionPayment::new(
		client.clone(),
	)));
	let BabeDeps {
		keystore,
		babe_config,
		shared_epoch_changes,
	} = babe;
	io.extend_with(BabeApi::to_delegate(BabeRpcHandler::new(
		client.clone(),
		shared_epoch_changes.clone(),
		keystore,
		babe_config,
		select_chain,
		deny_unsafe,
	)));
	let GrandpaDeps {
		shared_voter_state,
		shared_authority_set,
		justification_stream,
		subscription_executor,
		finality_provider,
	} = grandpa;
	io.extend_with(GrandpaApi::to_delegate(GrandpaRpcHandler::new(
		shared_authority_set.clone(),
		shared_voter_state,
		justification_stream,
		subscription_executor,
		finality_provider,
	)));
	io.extend_with(SyncStateRpcApi::to_delegate(SyncStateRpcHandler::new(
		chain_spec,
		client.clone(),
		shared_authority_set,
		shared_epoch_changes,
		deny_unsafe,
	)?));
	io.extend_with(BalancesApi::to_delegate(Balances::new(client.clone())));
	io.extend_with(StakingApi::to_delegate(Staking::new(client.clone())));
	io.extend_with(FeeMarketApi::to_delegate(FeeMarket::new(client.clone())));

	let overrides_map = BTreeMap::from([(
		EthereumStorageSchema::V1,
		Box::new(SchemaV1Override::new(client.clone()))
			as Box<dyn StorageOverride<_> + Send + Sync>,
	)]);
	let overrides = Arc::new(OverrideHandle {
		schemas: overrides_map,
		fallback: Box::new(RuntimeApiStorageOverride::new(client.clone())),
	});
	let block_data_cache = Arc::new(EthBlockDataCache::new(50, 50));

	io.extend_with(EthApiServer::to_delegate(EthApi::new(
		client.clone(),
		pool.clone(),
		graph,
		eth_transaction_convertor,
		network.clone(),
		overrides.clone(),
		backend.clone(),
		is_authority,
		vec![],
		rpc_config.max_past_logs,
		block_data_cache.clone(),
	)));
	if let Some(filter_pool) = filter_pool {
		io.extend_with(EthFilterApiServer::to_delegate(EthFilterApi::new(
			client.clone(),
			backend,
			filter_pool.clone(),
			500 as usize, // max stored filters
			overrides.clone(),
			rpc_config.max_past_logs,
			block_data_cache.clone(),
		)));
	}
	io.extend_with(EthPubSubApiServer::to_delegate(EthPubSubApi::new(
		pool,
		client.clone(),
		network.clone(),
		SubscriptionManager::<HexEncodedIdProvider>::with_id_provider(
			HexEncodedIdProvider::default(),
			Arc::new(subscription_task_executor),
		),
		overrides,
	)));
	io.extend_with(NetApiServer::to_delegate(NetApi::new(
		client.clone(),
		network,
		// Whether to format the `peer_count` response as Hex (default) or not.
		true,
	)));
	io.extend_with(Web3ApiServer::to_delegate(Web3Api::new(client.clone())));

	let ethapi_cmd = rpc_config.ethapi.clone();

	if ethapi_cmd.contains(&EthApiCmd::Debug) || ethapi_cmd.contains(&EthApiCmd::Trace) {
		if let Some(trace_filter_requester) = tracing_requesters.trace {
			io.extend_with(TraceApiServer::to_delegate(Trace::new(
				client,
				trace_filter_requester,
				rpc_config.ethapi_trace_max_count,
			)));
		}

		if let Some(debug_requester) = tracing_requesters.debug {
			io.extend_with(DebugApiServer::to_delegate(Debug::new(debug_requester)));
		}
	}

	Ok(io)
}

/// Instantiate all RPC extensions for light node.
pub fn create_light<C, P, F>(deps: LightDeps<C, F, P>) -> RpcExtension
where
	C: 'static
		+ Send
		+ Sync
		+ sp_api::ProvideRuntimeApi<Block>
		+ sp_blockchain::HeaderBackend<Block>,
	C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>,
	C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
	P: 'static + sc_transaction_pool_api::TransactionPool,
	F: 'static + sc_client_api::Fetcher<Block>,
{
	// --- paritytech ---
	use substrate_frame_rpc_system::{LightSystem, SystemApi};

	let LightDeps {
		client,
		pool,
		remote_blockchain,
		fetcher,
	} = deps;
	let mut io = jsonrpc_core::IoHandler::default();

	io.extend_with(SystemApi::<Hash, AccountId, Nonce>::to_delegate(
		LightSystem::new(client, remote_blockchain, fetcher, pool),
	));

	io
}
