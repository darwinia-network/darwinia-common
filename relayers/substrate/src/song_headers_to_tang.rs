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

//! Song-to-Tang headers sync entrypoint.

use crate::{
	headers_pipeline::{SubstrateHeadersSyncPipeline, SubstrateHeadersToSubstrate},
	SongClient, TangClient,
};

use async_trait::async_trait;
use headers_relay::sync_types::QueuedHeader;
use relay_song_client::{HeaderId as SongHeaderId, Song, SyncHeader as SongSyncHeader};
use relay_substrate_client::{Error as SubstrateError, TransactionSignScheme};
use relay_tang_client::{BridgeSongCall, SigningParams as TangSigningParams, Tang};
use song_node_primitives::{
	BEST_SONG_BLOCKS_METHOD, FINALIZED_SONG_BLOCK_METHOD, INCOMPLETE_SONG_HEADERS_METHOD,
	IS_KNOWN_SONG_BLOCK_METHOD,
};
use sp_core::Pair;
use sp_runtime::Justification;

/// Song-to-Tang headers sync pipeline.
type SongHeadersToTang = SubstrateHeadersToSubstrate<Song, SongSyncHeader, Tang, TangSigningParams>;
/// Song header in-the-queue.
type QueuedSongHeader = QueuedHeader<SongHeadersToTang>;

#[async_trait]
impl SubstrateHeadersSyncPipeline for SongHeadersToTang {
	const BEST_BLOCK_METHOD: &'static str = BEST_SONG_BLOCKS_METHOD;
	const FINALIZED_BLOCK_METHOD: &'static str = FINALIZED_SONG_BLOCK_METHOD;
	const IS_KNOWN_BLOCK_METHOD: &'static str = IS_KNOWN_SONG_BLOCK_METHOD;
	const INCOMPLETE_HEADERS_METHOD: &'static str = INCOMPLETE_SONG_HEADERS_METHOD;

	type SignedTransaction = <Tang as TransactionSignScheme>::SignedTransaction;

	async fn make_submit_header_transaction(
		&self,
		header: QueuedSongHeader,
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
			BridgeSongCall::import_signed_header(header.header().clone().into_inner()).into();
		let transaction =
			Tang::sign_transaction(&self.target_client, &self.target_sign.signer, nonce, call);
		Ok(transaction)
	}

	async fn make_complete_header_transaction(
		&self,
		id: SongHeaderId,
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
		let call = BridgeSongCall::finalize_header(id.1, completion).into();
		let transaction =
			Tang::sign_transaction(&self.target_client, &self.target_sign.signer, nonce, call);
		Ok(transaction)
	}
}

/// Run Song-to-Tang headers sync.
pub async fn run(
	song_client: SongClient,
	tang_client: TangClient,
	tang_sign: TangSigningParams,
	metrics_params: Option<relay_utils::metrics::MetricsParams>,
) {
	crate::headers_pipeline::run(
		SongHeadersToTang::new(tang_client.clone(), tang_sign),
		song_client,
		tang_client,
		metrics_params,
	)
	.await;
}
