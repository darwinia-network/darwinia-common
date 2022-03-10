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

//! A collection of node-specific RPC methods.

// --- crates.io ---
use futures::channel::mpsc::Sender;
// --- darwinia-network ---
use crate::*;
use dc_rpc::{Debug, DebugApiServer, Trace, TraceApiServer};
use template_runtime::TransactionConverter;
// --- paritytech ---
use fc_db::Backend as DvmBackend;
use fc_rpc::{
	EthApi, EthApiServer, EthBlockDataCache, EthDevSigner, EthFilterApi, EthFilterApiServer,
	EthPubSubApi, EthPubSubApiServer, EthSigner, HexEncodedIdProvider, NetApi, NetApiServer,
	OverrideHandle, Web3Api, Web3ApiServer,
};
use fc_rpc_core::types::{FeeHistoryCache, FilterPool};
use jsonrpc_pubsub::manager::SubscriptionManager;
use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApi};
use sc_consensus_manual_seal::rpc::{EngineCommand, ManualSeal, ManualSealApi};
use sc_network::NetworkService;
use sc_transaction_pool::{ChainApi, Pool};
use substrate_frame_rpc_system::FullSystem;

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
	pub network: Arc<NetworkService<Block, Hash>>,
	/// EthFilterApi pool.
	pub filter_pool: Option<FilterPool>,
	/// Backend.
	pub backend: Arc<DvmBackend<Block>>,
	/// RPC requester for evm trace
	pub tracing_requesters: EthRpcRequesters,
	/// Ethereum RPC Config
	pub eth_rpc_config: EthRpcConfig,
	/// Fee history cache.
	pub fee_history_cache: FeeHistoryCache,
	/// Manual seal command sink
	pub command_sink: Option<Sender<EngineCommand<Hash>>>,
	/// Ethereum data access overrides.
	pub overrides: Arc<OverrideHandle<Block>>,
	/// Cache for Ethereum block data.
	pub block_data_cache: Arc<EthBlockDataCache<Block>>,
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
	C::Api: fp_rpc::EthereumRuntimeRPCApi<Block>,
	C::Api: fp_rpc::ConvertTransactionRuntimeApi<Block>,
	C::Api: dp_evm_trace_apis::DebugRuntimeApi<Block>,
	P: 'static + sc_transaction_pool_api::TransactionPool<Block = Block>,
	B: 'static + sc_client_api::Backend<Block>,
	B::State: sc_client_api::StateBackend<Hashing>,
	A: ChainApi<Block = Block> + 'static,
{
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
		eth_rpc_config,
		fee_history_cache,
		overrides,
		block_data_cache,
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

	io.extend_with(EthApiServer::to_delegate(EthApi::new(
		client.clone(),
		pool.clone(),
		graph,
		Some(TransactionConverter),
		network.clone(),
		signers,
		overrides.clone(),
		backend.clone(),
		is_authority,
		eth_rpc_config.max_past_logs,
		block_data_cache.clone(),
		eth_rpc_config.fee_history_limit,
		fee_history_cache,
	)));

	if let Some(filter_pool) = filter_pool {
		io.extend_with(EthFilterApiServer::to_delegate(EthFilterApi::new(
			client.clone(),
			backend,
			filter_pool.clone(),
			500 as usize, // max stored filters
			eth_rpc_config.max_past_logs,
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

	let ethapi_cmd = eth_rpc_config.ethapi.clone();
	if ethapi_cmd.contains(&EthApiCmd::Debug) || ethapi_cmd.contains(&EthApiCmd::Trace) {
		if let Some(trace_filter_requester) = tracing_requesters.trace {
			io.extend_with(TraceApiServer::to_delegate(Trace::new(
				client,
				trace_filter_requester,
				eth_rpc_config.ethapi_trace_max_count,
			)));
		}

		if let Some(debug_requester) = tracing_requesters.debug {
			io.extend_with(DebugApiServer::to_delegate(Debug::new(debug_requester)));
		}
	}

	io
}
