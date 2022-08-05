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

// --- std ---
use std::sync::Arc;
// --- darwinia-network ---
use crate::EthDeps;
use drml_primitives::{OpaqueBlock as Block, *};
use template_runtime::TransactionConverter;

/// Full client dependencies.
pub struct FullDeps<C, P, A>
where
	A: sc_transaction_pool::ChainApi,
{
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
	/// Whether to deny unsafe calls
	pub deny_unsafe: sc_rpc::DenyUnsafe,
	/// DVM related rpc helper.
	pub eth: EthDeps<A>,
	/// Whether to enable dev signer
	pub enable_dev_signer: bool,
	/// Manual seal command sink
	pub command_sink:
		Option<futures::channel::mpsc::Sender<sc_consensus_manual_seal::EngineCommand<Hash>>>,
}

/// Instantiate all Full RPC extensions.
pub fn create_full<C, P, B, A>(
	deps: FullDeps<C, P, A>,
	subscription_task_executor: sc_rpc::SubscriptionTaskExecutor,
) -> crate::RpcExtension
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
	C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>
		+ sp_block_builder::BlockBuilder<Block>
		+ pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>
		+ fp_rpc::EthereumRuntimeRPCApi<Block>
		+ fp_rpc::ConvertTransactionRuntimeApi<Block>
		+ moonbeam_rpc_primitives_debug::DebugRuntimeApi<Block>,
	P: 'static + sc_transaction_pool_api::TransactionPool<Block = Block>,
	B: 'static + sc_client_api::Backend<Block>,
	B::State: sc_client_api::StateBackend<Hashing>,
	A: 'static + sc_transaction_pool::ChainApi<Block = Block>,
{
	// --- crates.io ---
	use jsonrpc_pubsub::manager::SubscriptionManager;
	// --- paritytech ---
	use fc_rpc::*;
	use pallet_transaction_payment_rpc::*;
	use sc_consensus_manual_seal::rpc::*;
	use substrate_frame_rpc_system::*;
	// --- darwinia-network ---
	use crate::EthRpcConfig;
	use moonbeam_rpc_debug::*;
	use moonbeam_rpc_trace::*;

	let FullDeps { client, pool, deny_unsafe, eth, enable_dev_signer, command_sink } = deps;
	let EthDeps {
		config:
			EthRpcConfig {
				ethapi_debug_targets,
				ethapi_trace_max_count,
				max_past_logs,
				fee_history_limit,
				..
			},
		graph,
		is_authority,
		network,
		filter_pool,
		backend,
		fee_history_cache,
		overrides,
		block_data_cache,
		rpc_requesters,
	} = eth;
	let mut io = jsonrpc_core::IoHandler::default();

	io.extend_with(SystemApi::to_delegate(FullSystem::new(
		client.clone(),
		pool.clone(),
		deny_unsafe,
	)));
	io.extend_with(TransactionPaymentApi::to_delegate(TransactionPayment::new(client.clone())));

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
		// max_past_logs,
		block_data_cache.clone(),
		fee_history_limit,
		fee_history_cache,
	)));

	if let Some(filter_pool) = filter_pool {
		io.extend_with(EthFilterApiServer::to_delegate(EthFilterApi::new(
			client.clone(),
			filter_pool.clone(),
			500 as usize, // max stored filters
			max_past_logs,
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

	if let Some(command_sink) = command_sink {
		io.extend_with(
			// We provide the rpc handler with the sending end of the channel to allow the rpc
			// send EngineCommands to the background block authorship task.
			ManualSealApi::to_delegate(ManualSeal::new(command_sink)),
		);
	}

	if ethapi_debug_targets.iter().any(|cmd| matches!(cmd.as_str(), "debug" | "trace")) {
		if let Some(trace_filter_requester) = rpc_requesters.trace {
			io.extend_with(TraceServer::to_delegate(Trace::new(
				client,
				trace_filter_requester,
				ethapi_trace_max_count,
			)));
		}

		if let Some(debug_requester) = rpc_requesters.debug {
			io.extend_with(DebugServer::to_delegate(Debug::new(debug_requester)));
		}
	}

	io
}
