//! Node-specific RPC methods for interaction with header-mmr.

// --- darwinia ---
pub use darwinia_header_mmr_rpc_runtime_api::HeaderMMRApi as HeaderMMRRuntimeApi;

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
use darwinia_header_mmr_rpc_runtime_api::RuntimeDispatchInfo;

const RUNTIME_ERROR: i64 = -1;

#[rpc]
pub trait HeaderMMRApi<BlockNumber, Hash, Response> {
	#[rpc(name = "headerMMR_genProof")]
	fn gen_proof(
		&self,
		block_number: BlockNumber,
		mmr_block_number: BlockNumber,
	) -> Result<Response>;
}

pub struct HeaderMMR<Client, Block> {
	client: Arc<Client>,
	_marker: std::marker::PhantomData<Block>,
}

impl<Client, Block> HeaderMMR<Client, Block> {
	pub fn new(client: Arc<Client>) -> Self {
		Self {
			client,
			_marker: Default::default(),
		}
	}
}

impl<Client, Block, BlockNumber, Hash> HeaderMMRApi<BlockNumber, Hash, RuntimeDispatchInfo<Hash>>
	for HeaderMMR<Client, Block>
where
	Client: 'static + Send + Sync + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	Client::Api: HeaderMMRRuntimeApi<Block, BlockNumber, Hash>,
	Block: BlockT,
	BlockNumber: Codec,
	Hash: Codec + MaybeDisplay + MaybeFromStr,
{
	fn gen_proof(
		&self,
		block_number: BlockNumber,
		mmr_block_number: BlockNumber,
	) -> Result<RuntimeDispatchInfo<Hash>> {
		let api = self.client.runtime_api();
		let best = self.client.info().best_hash;
		let at = BlockId::hash(best);

		api.gen_proof(&at, block_number, mmr_block_number)
			.map_err(|e| Error {
				code: ErrorCode::ServerError(RUNTIME_ERROR),
				message: "Unable to query power.".into(),
				data: Some(format!("{:?}", e).into()),
			})
	}
}
