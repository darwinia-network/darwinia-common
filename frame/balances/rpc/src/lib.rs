//! Node-specific RPC methods for interaction with balances.

// --- darwinia ---
pub use darwinia_balances_rpc_runtime_api::BalancesApi as BalancesRuntimeApi;

// --- core ---
use core::fmt::Debug;
// --- std ---
use std::sync::Arc;
// --- crates ---
use codec::Codec;
use jsonrpc_core::{Error, ErrorCode, Result};
use jsonrpc_derive::rpc;
// --- substrate ---
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{
	generic::BlockId,
	traits::{Block as BlockT, MaybeDisplay, MaybeFromStr},
};
// --- darwinia ---
use darwinia_balances_rpc_runtime_api::RuntimeDispatchInfo;

const RUNTIME_ERROR: i64 = -1;

#[rpc]
pub trait BalancesApi<AccountId, Response> {
	#[rpc(name = "balances_usableBalance")]
	fn usable_balance(&self, instance: u8, who: AccountId) -> Result<Response>;
}

pub struct Balances<Client, Block> {
	client: Arc<Client>,
	_marker: std::marker::PhantomData<Block>,
}

impl<Client, Block> Balances<Client, Block> {
	pub fn new(client: Arc<Client>) -> Self {
		Self {
			client,
			_marker: Default::default(),
		}
	}
}

impl<Client, Block, AccountId, Balance> BalancesApi<AccountId, RuntimeDispatchInfo<Balance>>
	for Balances<Client, Block>
where
	Client: 'static + Send + Sync + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	Client::Api: BalancesRuntimeApi<Block, AccountId, Balance>,
	Block: BlockT,
	AccountId: Codec,
	Balance: Debug + Codec + MaybeDisplay + MaybeFromStr,
{
	fn usable_balance(&self, instance: u8, who: AccountId) -> Result<RuntimeDispatchInfo<Balance>> {
		let api = self.client.runtime_api();
		let best = self.client.info().best_hash;
		let at = BlockId::hash(best);

		api.usable_balance(&at, instance, who).map_err(|e| Error {
			code: ErrorCode::ServerError(RUNTIME_ERROR),
			message: "Unable to query usable balance.".into(),
			data: Some(format!("{:?}", e).into()),
		})
	}
}
