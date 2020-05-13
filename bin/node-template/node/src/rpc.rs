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

// --- std ---
use std::sync::Arc;
// --- substrate ---
use sp_api::ProvideRuntimeApi;
// --- darwinia ---
use node_template_runtime::{opaque::Block, AccountId, Balance, Power};

/// A type representing all RPC extensions.
pub type RpcExtension = jsonrpc_core::IoHandler<sc_rpc::Metadata>;

pub fn create<C>(client: Arc<C>) -> RpcExtension
where
	C: ProvideRuntimeApi<Block>,
	C: sc_client::blockchain::HeaderBackend<Block>,
	C: 'static + Send + Sync,
	C::Api: darwinia_balances_rpc::BalancesRuntimeApi<Block, AccountId, Balance>,
	C::Api: darwinia_staking_rpc::StakingRuntimeApi<Block, AccountId, Power>,
{
	// --- darwinia ---
	use darwinia_balances_rpc::{Balances, BalancesApi};
	use darwinia_staking_rpc::{Staking, StakingApi};

	let mut io = jsonrpc_core::IoHandler::default();
	io.extend_with(BalancesApi::to_delegate(Balances::new(client.clone())));
	io.extend_with(StakingApi::to_delegate(Staking::new(client)));

	io
}
