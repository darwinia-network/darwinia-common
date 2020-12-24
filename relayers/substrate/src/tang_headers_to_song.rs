// Copyright 2019-2020 Parity Technologies (UK) Ltd.
// This file is part of Parity Bridges Common.

// Parity Bridges Common is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity Bridges Common is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity Bridges Common.  If not, see <http://www.gnu.org/licenses/>.

//! Tang-to-Song headers sync entrypoint.

use crate::{
	headers_pipeline::{SubstrateHeadersSyncPipeline, SubstrateHeadersToSubstrate},
	SongClient, TangClient,
};

use async_trait::async_trait;
use headers_relay::sync_types::QueuedHeader;
use relay_song_client::{BridgeTangCall, SigningParams as SongSigningParams, Song};
use relay_substrate_client::{Error as SubstrateError, TransactionSignScheme};
use relay_tang_client::{HeaderId as TangHeaderId, SyncHeader as TangSyncHeader, Tang};
use sp_core::Pair;
use sp_runtime::Justification;
use tang_node_primitives::{
	BEST_TANG_BLOCKS_METHOD, FINALIZED_TANG_BLOCK_METHOD, INCOMPLETE_TANG_HEADERS_METHOD,
	IS_KNOWN_TANG_BLOCK_METHOD,
};

/// Tang-to-Song headers sync pipeline.
pub(crate) type TangHeadersToSong =
	SubstrateHeadersToSubstrate<Tang, TangSyncHeader, Song, SongSigningParams>;
/// Tang header in-the-queue.
type QueuedTangHeader = QueuedHeader<TangHeadersToSong>;

#[async_trait]
impl SubstrateHeadersSyncPipeline for TangHeadersToSong {
	const BEST_BLOCK_METHOD: &'static str = BEST_TANG_BLOCKS_METHOD;
	const FINALIZED_BLOCK_METHOD: &'static str = FINALIZED_TANG_BLOCK_METHOD;
	const IS_KNOWN_BLOCK_METHOD: &'static str = IS_KNOWN_TANG_BLOCK_METHOD;
	const INCOMPLETE_HEADERS_METHOD: &'static str = INCOMPLETE_TANG_HEADERS_METHOD;

	type SignedTransaction = <Song as TransactionSignScheme>::SignedTransaction;

	async fn make_submit_header_transaction(
		&self,
		header: QueuedTangHeader,
	) -> Result<Self::SignedTransaction, SubstrateError> {
		let account_id = self
			.target_sign
			.signer
			.public()
			.as_array_ref()
			.clone()
			.into();
		let nonce = self.target_client.next_account_index(account_id).await?;
		let call =
			BridgeTangCall::import_signed_header(header.header().clone().into_inner()).into();
		let transaction =
			Song::sign_transaction(&self.target_client, &self.target_sign.signer, nonce, call);
		Ok(transaction)
	}

	async fn make_complete_header_transaction(
		&self,
		id: TangHeaderId,
		completion: Justification,
	) -> Result<Self::SignedTransaction, SubstrateError> {
		let account_id = self
			.target_sign
			.signer
			.public()
			.as_array_ref()
			.clone()
			.into();
		let nonce = self.target_client.next_account_index(account_id).await?;
		let call = BridgeTangCall::finalize_header(id.1, completion).into();
		let transaction =
			Song::sign_transaction(&self.target_client, &self.target_sign.signer, nonce, call);
		Ok(transaction)
	}
}

/// Run Tang-to-Song headers sync.
pub async fn run(
	tang_client: TangClient,
	song_client: SongClient,
	song_sign: SongSigningParams,
	metrics_params: Option<relay_utils::metrics::MetricsParams>,
) {
	crate::headers_pipeline::run(
		TangHeadersToSong::new(song_client.clone(), song_sign),
		tang_client,
		song_client,
		metrics_params,
	)
	.await;
}
