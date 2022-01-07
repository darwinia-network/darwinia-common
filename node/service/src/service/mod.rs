macro_rules! impl_runtime_apis {
	($($extra_apis:path),*) => {
		/// A set of APIs that darwinia-like runtimes must implement.
		pub trait RuntimeApiCollection:
			sp_api::ApiExt<Block>
			+ sp_api::Metadata<Block>
			+ sp_authority_discovery::AuthorityDiscoveryApi<Block>
			+ sp_block_builder::BlockBuilder<Block>
			+ sp_consensus_babe::BabeApi<Block>
			+ sp_finality_grandpa::GrandpaApi<Block>
			+ sp_offchain::OffchainWorkerApi<Block>
			+ sp_session::SessionKeys<Block>
			+ sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
			+ frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce>
			+ pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance>
			$(+ $extra_apis)*
		where
			<Self as sp_api::ApiExt<Block>>::StateBackend: sp_api::StateBackend<BlakeTwo256>,
		{
		}
		impl<Api> RuntimeApiCollection for Api
		where
			Api: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
				+ sp_api::ApiExt<Block>
				+ sp_api::Metadata<Block>
				+ sp_authority_discovery::AuthorityDiscoveryApi<Block>
				+ sp_block_builder::BlockBuilder<Block>
				+ sp_consensus_babe::BabeApi<Block>
				+ sp_finality_grandpa::GrandpaApi<Block>
				+ sp_offchain::OffchainWorkerApi<Block>
				+ sp_session::SessionKeys<Block>
				+ frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce>
				+ pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance>
				$(+ $extra_apis)*,
			<Self as sp_api::ApiExt<Block>>::StateBackend: sp_api::StateBackend<BlakeTwo256>,
		{
		}
	};
}

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
use std::sync::Arc;
// --- crates.io ---
use codec::Codec;
// --- paritytech ---
use sc_consensus::LongestChain;
use sc_executor::NativeElseWasmExecutor;
use sc_finality_grandpa::GrandpaBlockImport;
use sc_keystore::LocalKeystore;
use sc_service::{
	config::PrometheusConfig, ChainSpec, Configuration, Error as ServiceError, TFullBackend,
	TFullClient, TLightBackendWithHash, TLightClientWithBackend,
};
use sp_runtime::traits::BlakeTwo256;
use substrate_prometheus_endpoint::Registry;
// --- darwinia-network ---
use drml_common_primitives::OpaqueBlock as Block;
use drml_rpc::RpcExtension;

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
