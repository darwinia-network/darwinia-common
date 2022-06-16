macro_rules! impl_runtime_apis {
	($api:path,$($extra_apis:path),*,) => {
		/// A set of APIs that darwinia-like runtimes must implement.
		pub trait RuntimeApiCollection:
			$api
			$(+ $extra_apis)*
		{
		}
		impl<Api> RuntimeApiCollection for Api
		where
			Api: $api
				$(+ $extra_apis)*
		{
		}
	};
}

// --- darwinia-network ---
use drml_primitives::{OpaqueBlock as Block, *};

impl_runtime_apis![
	sp_api::ApiExt<Block>,
	sp_api::Metadata<Block>,
	sp_block_builder::BlockBuilder<Block>,
	sp_session::SessionKeys<Block>,
	sp_consensus_babe::BabeApi<Block>,
	sp_finality_grandpa::GrandpaApi<Block>,
	// beefy_primitives::BeefyApi<Block>,
	sp_authority_discovery::AuthorityDiscoveryApi<Block>,
	sp_offchain::OffchainWorkerApi<Block>,
	sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>,
	frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce>,
	pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance>,
	darwinia_staking_rpc_runtime_api::StakingApi<Block, AccountId, Power>,
	fp_rpc::EthereumRuntimeRPCApi<Block>,
	fp_rpc::ConvertTransactionRuntimeApi<Block>,
	moonbeam_rpc_primitives_debug::DebugRuntimeApi<Block>,
];
