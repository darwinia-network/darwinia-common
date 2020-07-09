//! Node-specific RPC methods for interaction with staking.

// --- darwinia ---
pub use darwinia_staking_rpc_runtime_api::StakingApi as StakingRuntimeApi;

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
use darwinia_staking_rpc_runtime_api::RuntimeDispatchInfo;

const RUNTIME_ERROR: i64 = -1;

#[rpc]
pub trait StakingApi<AccountId, Response> {
	#[rpc(name = "staking_powerOf")]
	fn power_of(&self, who: AccountId) -> Result<Response>;
}

pub struct Staking<Client, Block> {
	client: Arc<Client>,
	_marker: std::marker::PhantomData<Block>,
}

impl<Client, Block> Staking<Client, Block> {
	pub fn new(client: Arc<Client>) -> Self {
		Self {
			client,
			_marker: Default::default(),
		}
	}
}

impl<Client, Block, AccountId, Power> StakingApi<AccountId, RuntimeDispatchInfo<Power>>
	for Staking<Client, Block>
where
	Client: 'static + Send + Sync + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	Client::Api: StakingRuntimeApi<Block, AccountId, Power>,
	Block: BlockT,
	AccountId: Codec,
	Power: Codec + MaybeDisplay + MaybeFromStr,
{
	fn power_of(&self, who: AccountId) -> Result<RuntimeDispatchInfo<Power>> {
		let api = self.client.runtime_api();
		let best = self.client.info().best_hash;
		let at = BlockId::hash(best);

		api.power_of(&at, who).map_err(|e| Error {
			code: ErrorCode::ServerError(RUNTIME_ERROR),
			message: "Unable to query power.".into(),
			data: Some(format!("{:?}", e).into()),
		})
	}
}
