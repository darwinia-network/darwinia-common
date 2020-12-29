//! A collection of node-specific RPC methods.
//!
//! Since `substrate` core functionality makes no assumptions
//! about the modules used inside the runtime, so do
//! RPC methods defined in `sc-rpc` crate.
//! It means that `client/rpc` can't have any methods that
//! need some strong assumptions about the particular runtime.
//!
//! The RPCs available in this crate however can make some assumptions
//! about how the runtime is constructed and what FRAME pallets
//! are part of it. Therefore all node-runtime-specific RPCs can
//! be placed here or imported from corresponding FRAME RPC definitions.

#![warn(missing_docs)]

// --- substrate ---
pub use sc_rpc::{DenyUnsafe, SubscriptionTaskExecutor};

// --- std ---
use std::sync::Arc;
// --- darwinia ---
use tang_node_primitives::{
	AccountId, Balance, BlockNumber, Hash, Nonce, OpaqueBlock as Block, Power,
};

/// A type representing all RPC extensions.
pub type RpcExtension = jsonrpc_core::IoHandler<sc_rpc::Metadata>;

/// Extra dependencies for BABE.
pub struct BabeDeps {
	/// BABE protocol config.
	pub babe_config: sc_consensus_babe::Config,
	/// BABE pending epoch changes.
	pub shared_epoch_changes:
		sc_consensus_epochs::SharedEpochChanges<Block, sc_consensus_babe::Epoch>,
	/// The keystore that manages the keys of the node.
	pub keystore: sc_keystore::KeyStorePtr,
}

/// Extra dependencies for GRANDPA
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

/// Full client dependencies.
pub struct FullDeps<C, P, SC, B> {
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
	/// The SelectChain Strategy
	pub select_chain: SC,
	/// Whether to deny unsafe calls
	pub deny_unsafe: sc_rpc::DenyUnsafe,
	/// The Node authority flag
	pub is_authority: bool,
	/// Network service
	pub network: Arc<sc_network::NetworkService<Block, Hash>>,
	/// BABE specific dependencies.
	pub babe: BabeDeps,
	/// GRANDPA specific dependencies.
	pub grandpa: GrandpaDeps<B>,
	/// Full client backend
	pub backend: Arc<B>,
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

/// Instantiate all RPC extensions.
pub fn create_full<C, P, SC, B>(
	deps: FullDeps<C, P, SC, B>,
	subscription_task_executor: sc_rpc::SubscriptionTaskExecutor,
) -> RpcExtension
where
	C: 'static
		+ Send
		+ Sync
		+ sp_api::ProvideRuntimeApi<Block>
		+ sc_client_api::AuxStore
		+ sc_client_api::BlockchainEvents<Block>
		+ sc_client_api::StorageProvider<Block, B>
		+ sp_blockchain::HeaderBackend<Block>
		+ sp_blockchain::HeaderMetadata<Block, Error = sp_blockchain::Error>,
	C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>,
	C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
	C::Api: sc_consensus_babe::BabeApi<Block>,
	C::Api: sp_block_builder::BlockBuilder<Block>,
	C::Api: darwinia_balances_rpc::BalancesRuntimeApi<Block, AccountId, Balance>,
	C::Api: darwinia_header_mmr_rpc::HeaderMMRRuntimeApi<Block, Hash>,
	C::Api: darwinia_staking_rpc::StakingRuntimeApi<Block, AccountId, Power>,
	C::Api: dvm_rpc_runtime_api::EthereumRuntimeRPCApi<Block>,
	<C::Api as sp_api::ApiErrorExt>::Error: std::fmt::Debug,
	P: 'static + Sync + Send + sp_transaction_pool::TransactionPool<Block = Block>,
	SC: 'static + sp_consensus::SelectChain<Block>,
	B: 'static + Send + Sync + sc_client_api::Backend<Block>,
	B::State: sc_client_api::StateBackend<sp_runtime::traits::HashFor<Block>>,
{
	// --- crates ---
	use jsonrpc_pubsub::manager::SubscriptionManager;
	// --- substrate ---
	use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApi};
	use sc_consensus_babe_rpc::{BabeApi, BabeRpcHandler};
	use sc_finality_grandpa_rpc::GrandpaRpcHandler;
	use substrate_frame_rpc_system::{FullSystem, SystemApi};
	// --- darwinia ---
	use darwinia_balances_rpc::{Balances, BalancesApi};
	use darwinia_header_mmr_rpc::{HeaderMMR, HeaderMMRApi};
	use darwinia_staking_rpc::{Staking, StakingApi};
	use dvm_rpc::{
		EthApi, EthApiServer, EthPubSubApi, EthPubSubApiServer, HexEncodedIdProvider, NetApi,
		NetApiServer, Web3Api, Web3ApiServer,
	};
	use tang_node_runtime::TransactionConverter;

	let FullDeps {
		client,
		pool,
		select_chain,
		deny_unsafe,
		is_authority,
		network,
		babe,
		grandpa,
		backend,
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
	{
		let BabeDeps {
			keystore,
			babe_config,
			shared_epoch_changes,
		} = babe;
		io.extend_with(BabeApi::to_delegate(BabeRpcHandler::new(
			client.clone(),
			shared_epoch_changes,
			keystore,
			babe_config,
			select_chain,
			deny_unsafe,
		)));
	};
	{
		let GrandpaDeps {
			shared_voter_state,
			shared_authority_set,
			justification_stream,
			subscription_executor,
			finality_provider,
		} = grandpa;
		io.extend_with(sc_finality_grandpa_rpc::GrandpaApi::to_delegate(
			GrandpaRpcHandler::new(
				shared_authority_set,
				shared_voter_state,
				justification_stream,
				subscription_executor,
				finality_provider,
			),
		));
	}
	io.extend_with(BalancesApi::to_delegate(Balances::new(client.clone())));
	io.extend_with(HeaderMMRApi::to_delegate(HeaderMMR::new(client.clone())));
	io.extend_with(StakingApi::to_delegate(Staking::new(client.clone())));
	io.extend_with(EthApiServer::to_delegate(EthApi::new(
		client.clone(),
		pool.clone(),
		TransactionConverter,
		network.clone(),
		is_authority,
	)));
	io.extend_with(EthPubSubApiServer::to_delegate(EthPubSubApi::new(
		pool,
		client.clone(),
		network.clone(),
		SubscriptionManager::<HexEncodedIdProvider>::with_id_provider(
			HexEncodedIdProvider::default(),
			Arc::new(subscription_task_executor),
		),
	)));
	io.extend_with(NetApiServer::to_delegate(NetApi::new(
		client.clone(),
		network,
	)));
	io.extend_with(Web3ApiServer::to_delegate(Web3Api::new(client)));

	use bp_message_lane::{LaneId, MessageNonce};
	use bp_runtime::InstanceId;
	use pallet_message_lane_rpc::{MessageLaneApi, MessageLaneRpcHandler};
	use sp_core::storage::StorageKey;
	use tang_node_runtime::song_message::SONG_BRIDGE_INSTANCE;

	/// Tang runtime from message-lane RPC point of view.
	struct TangMessageLaneKeys;
	impl pallet_message_lane_rpc::Runtime for TangMessageLaneKeys {
		fn message_key(
			&self,
			instance: &InstanceId,
			lane: &LaneId,
			nonce: MessageNonce,
		) -> Option<StorageKey> {
			match *instance {
				SONG_BRIDGE_INSTANCE => {
					Some(tang_node_runtime::song_message::message_key(lane, nonce))
				}
				_ => None,
			}
		}

		fn outbound_lane_data_key(
			&self,
			instance: &InstanceId,
			lane: &LaneId,
		) -> Option<StorageKey> {
			match *instance {
				SONG_BRIDGE_INSTANCE => Some(
					tang_node_runtime::song_message::outbound_lane_data_key(lane),
				),
				_ => None,
			}
		}

		fn inbound_lane_data_key(
			&self,
			instance: &InstanceId,
			lane: &LaneId,
		) -> Option<StorageKey> {
			match *instance {
				SONG_BRIDGE_INSTANCE => {
					Some(tang_node_runtime::song_message::inbound_lane_data_key(lane))
				}
				_ => None,
			}
		}
	}

	io.extend_with(MessageLaneApi::to_delegate(MessageLaneRpcHandler::new(
		// no backend here, how to do with it
		backend.clone(),
		Arc::new(TangMessageLaneKeys),
	)));

	io
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
	P: 'static + sp_transaction_pool::TransactionPool,
	F: 'static + sc_client_api::Fetcher<Block>,
{
	// --- substrate ---
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
