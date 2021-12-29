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

//! A collection of node-specific RPC methods.

// --- std ---
use std::collections::BTreeMap;
// --- darwinia-network ---
use crate::*;
use dc_rpc::EthBlockDataCache;
// --- paritytech ---
use sc_transaction_pool::{ChainApi, Pool};

/// Full client dependencies.
pub struct FullDeps<C, P, A: ChainApi> {
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
	/// Graph pool instance.
	pub graph: Arc<Pool<A>>,
	/// Whether to deny unsafe calls
	pub deny_unsafe: DenyUnsafe,
	/// The Node authority flag
	pub is_authority: bool,
	/// Whether to enable dev signer
	pub enable_dev_signer: bool,
	/// Network service
	pub network: Arc<sc_network::NetworkService<Block, Hash>>,
	/// EthFilterApi pool.
	pub filter_pool: Option<dp_rpc::FilterPool>,
	/// Backend.
	pub backend: Arc<dc_db::Backend<Block>>,
	/// Rpc requester for evm trace
	pub tracing_requesters: RpcRequesters,
	/// Rpc Config
	pub rpc_config: RpcConfig,
	/// Manual seal command sink
	pub command_sink:
		Option<futures::channel::mpsc::Sender<sc_consensus_manual_seal::rpc::EngineCommand<Hash>>>,
}

/// Instantiate all Full RPC extensions.
pub fn create_full<C, P, B, A>(
	deps: FullDeps<C, P, A>,
	subscription_task_executor: SubscriptionTaskExecutor,
) -> RpcExtension
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
	C::Api: sp_block_builder::BlockBuilder<Block>,
	C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
	C::Api: dp_evm_trace_apis::DebugRuntimeApi<Block>,
	C::Api: dvm_rpc_runtime_api::EthereumRuntimeRPCApi<Block>,
	P: 'static + sc_transaction_pool_api::TransactionPool<Block = Block>,
	B: 'static + sc_client_api::Backend<Block>,
	B::State: sc_client_api::StateBackend<Hashing>,
	A: ChainApi<Block = Block> + 'static,
{
	// --- crates.io ---
	use jsonrpc_pubsub::manager::SubscriptionManager;
	// --- paritytech ---
	use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApi};
	use sc_consensus_manual_seal::rpc::{ManualSeal, ManualSealApi};
	use substrate_frame_rpc_system::{FullSystem, SystemApi};
	// --- darwinia-network ---
	use dc_rpc::{
		Debug, DebugApiServer, EthApi, EthApiServer, EthDevSigner, EthFilterApi,
		EthFilterApiServer, EthPubSubApi, EthPubSubApiServer, EthSigner, HexEncodedIdProvider,
		NetApi, NetApiServer, OverrideHandle, RuntimeApiStorageOverride, SchemaV1Override,
		StorageOverride, Trace, TraceApiServer, Web3Api, Web3ApiServer,
	};
	use dvm_ethereum::EthereumStorageSchema;
	use template_runtime::TransactionConverter;

	let mut io = jsonrpc_core::IoHandler::default();
	let FullDeps {
		client,
		pool,
		graph,
		deny_unsafe,
		is_authority,
		enable_dev_signer,
		network,
		filter_pool,
		command_sink,
		backend,
		tracing_requesters,
		rpc_config,
	} = deps;

	io.extend_with(SystemApi::to_delegate(FullSystem::new(
		client.clone(),
		pool.clone(),
		deny_unsafe,
	)));
	io.extend_with(TransactionPaymentApi::to_delegate(TransactionPayment::new(
		client.clone(),
	)));

	let mut signers = Vec::new();
	if enable_dev_signer {
		signers.push(Box::new(EthDevSigner::new()) as Box<dyn EthSigner>);
	}
	let mut overrides_map = BTreeMap::new();
	overrides_map.insert(
		EthereumStorageSchema::V1,
		Box::new(SchemaV1Override::new(client.clone()))
			as Box<dyn StorageOverride<_> + Send + Sync>,
	);

	let overrides = Arc::new(OverrideHandle {
		schemas: overrides_map,
		fallback: Box::new(RuntimeApiStorageOverride::new(client.clone())),
	});

	let block_data_cache = Arc::new(EthBlockDataCache::new(50, 50));
	io.extend_with(EthApiServer::to_delegate(EthApi::new(
		client.clone(),
		pool.clone(),
		graph,
		TransactionConverter,
		network.clone(),
		overrides.clone(),
		backend.clone(),
		is_authority,
		signers,
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

	io.extend_with(NetApiServer::to_delegate(NetApi::new(
		client.clone(),
		network.clone(),
		// Whether to format the `peer_count` response as Hex (default) or not.
		true,
	)));

	io.extend_with(Web3ApiServer::to_delegate(Web3Api::new(client.clone())));

	io.extend_with(EthPubSubApiServer::to_delegate(EthPubSubApi::new(
		pool.clone(),
		client.clone(),
		network.clone(),
		SubscriptionManager::<HexEncodedIdProvider>::with_id_provider(
			HexEncodedIdProvider::default(),
			Arc::new(subscription_task_executor),
		),
		overrides,
	)));

	match command_sink {
		Some(command_sink) => {
			io.extend_with(
				// We provide the rpc handler with the sending end of the channel to allow the rpc
				// send EngineCommands to the background block authorship task.
				ManualSealApi::to_delegate(ManualSeal::new(command_sink)),
			);
		}
		_ => {}
	}

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

	io
}
