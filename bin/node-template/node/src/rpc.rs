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
use sc_finality_grandpa::{SharedAuthoritySet, SharedVoterState};
use sp_api::ProvideRuntimeApi;
// --- darwinia ---
use node_template_runtime::{opaque::Block, AccountId, Balance, BlockNumber, Hash, Power};

/// A type representing all RPC extensions.
pub type RpcExtension = jsonrpc_core::IoHandler<sc_rpc::Metadata>;

/// Extra dependencies for GRANDPA
pub struct GrandpaDeps {
	/// Voting round info.
	pub shared_voter_state: SharedVoterState,
	/// Authority set info.
	pub shared_authority_set: SharedAuthoritySet<Hash, BlockNumber>,
}

/// Full client dependencies.
pub struct FullDeps<C> {
	/// The client instance to use.
	pub client: Arc<C>,
	/// GRANDPA specific dependencies.
	pub grandpa: GrandpaDeps,
}

pub fn create<C>(deps: FullDeps<C>) -> RpcExtension
where
	C: ProvideRuntimeApi<Block>,
	C: sp_blockchain::HeaderBackend<Block>,
	C: 'static + Send + Sync,
	C::Api: darwinia_balances_rpc::BalancesRuntimeApi<Block, AccountId, Balance>,
	C::Api: darwinia_staking_rpc::StakingRuntimeApi<Block, AccountId, Power>,
{
	// --- substrate ---
	use sc_finality_grandpa_rpc::GrandpaRpcHandler;
	// --- darwinia ---
	use darwinia_balances_rpc::{Balances, BalancesApi};
	use darwinia_staking_rpc::{Staking, StakingApi};

	let FullDeps { client, grandpa } = deps;

	let mut io = jsonrpc_core::IoHandler::default();
	{
		let GrandpaDeps {
			shared_voter_state,
			shared_authority_set,
		} = grandpa;
		io.extend_with(sc_finality_grandpa_rpc::GrandpaApi::to_delegate(
			GrandpaRpcHandler::new(shared_authority_set, shared_voter_state),
		));
	}
	io.extend_with(BalancesApi::to_delegate(Balances::new(client.clone())));
	io.extend_with(StakingApi::to_delegate(Staking::new(client)));

	io
}
