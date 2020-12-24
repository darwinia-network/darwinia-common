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

//! Song-to-Tang messages sync entrypoint.

use crate::messages_lane::{SubstrateMessageLane, SubstrateMessageLaneToSubstrate};
use crate::messages_source::SubstrateMessagesSource;
use crate::messages_target::SubstrateMessagesTarget;
use crate::{SongClient, TangClient};

use async_trait::async_trait;
use bp_message_lane::{LaneId, MessageNonce};
use song_node_runtime::tang_message::TANG_BRIDGE_INSTANCE;
use tang_node_runtime::song_message::SONG_BRIDGE_INSTANCE;

use messages_relay::message_lane::MessageLane;
use relay_song_client::{HeaderId as SongHeaderId, SigningParams as SongSigningParams, Song};
use relay_substrate_client::{Chain, Error as SubstrateError, TransactionSignScheme};
use relay_tang_client::{HeaderId as TangHeaderId, SigningParams as TangSigningParams, Tang};
use relay_utils::metrics::MetricsParams;
use sp_core::Pair;
use std::{ops::RangeInclusive, time::Duration};

/// Song-to-Tang message lane.
type SongMessagesToTang =
	SubstrateMessageLaneToSubstrate<Song, SongSigningParams, Tang, TangSigningParams>;

#[async_trait]
impl SubstrateMessageLane for SongMessagesToTang {
	const OUTBOUND_LANE_MESSAGES_DISPATCH_WEIGHT_METHOD: &'static str =
		tang_node_primitives::TO_TANG_MESSAGES_DISPATCH_WEIGHT_METHOD;
	const OUTBOUND_LANE_LATEST_GENERATED_NONCE_METHOD: &'static str =
		tang_node_primitives::TO_TANG_LATEST_GENERATED_NONCE_METHOD;
	const OUTBOUND_LANE_LATEST_RECEIVED_NONCE_METHOD: &'static str =
		tang_node_primitives::TO_TANG_LATEST_RECEIVED_NONCE_METHOD;

	const INBOUND_LANE_LATEST_RECEIVED_NONCE_METHOD: &'static str =
		song_node_primitives::FROM_SONG_LATEST_RECEIVED_NONCE_METHOD;
	const INBOUND_LANE_LATEST_CONFIRMED_NONCE_METHOD: &'static str =
		song_node_primitives::FROM_SONG_LATEST_CONFIRMED_NONCE_METHOD;
	const INBOUND_LANE_UNREWARDED_RELAYERS_STATE: &'static str =
		song_node_primitives::FROM_SONG_UNREWARDED_RELAYERS_STATE;

	const BEST_FINALIZED_SOURCE_HEADER_ID_AT_TARGET: &'static str =
		song_node_primitives::FINALIZED_SONG_BLOCK_METHOD;
	const BEST_FINALIZED_TARGET_HEADER_ID_AT_SOURCE: &'static str =
		tang_node_primitives::FINALIZED_TANG_BLOCK_METHOD;

	type SourceSignedTransaction = <Song as TransactionSignScheme>::SignedTransaction;
	type TargetSignedTransaction = <Tang as TransactionSignScheme>::SignedTransaction;

	async fn make_messages_receiving_proof_transaction(
		&self,
		_generated_at_block: TangHeaderId,
		proof: <Self as MessageLane>::MessagesReceivingProof,
	) -> Result<Self::SourceSignedTransaction, SubstrateError> {
		let account_id = self
			.source_sign
			.signer
			.public()
			.as_array_ref()
			.clone()
			.into();
		let nonce = self.source_client.next_account_index(account_id).await?;
		let call =
			song_node_runtime::MessageLaneCall::receive_messages_delivery_proof(proof).into();
		let transaction =
			Song::sign_transaction(&self.source_client, &self.source_sign.signer, nonce, call);
		Ok(transaction)
	}

	async fn make_messages_delivery_transaction(
		&self,
		_generated_at_header: SongHeaderId,
		_nonces: RangeInclusive<MessageNonce>,
		proof: <Self as MessageLane>::MessagesProof,
	) -> Result<Self::TargetSignedTransaction, SubstrateError> {
		let (dispatch_weight, proof) = proof;
		let account_id = self
			.target_sign
			.signer
			.public()
			.as_array_ref()
			.clone()
			.into();
		let nonce = self.target_client.next_account_index(account_id).await?;
		let call = tang_node_runtime::MessageLaneCall::receive_messages_proof(
			self.relayer_id_at_source.clone(),
			proof,
			dispatch_weight,
		)
		.into();
		let transaction =
			Tang::sign_transaction(&self.target_client, &self.target_sign.signer, nonce, call);
		Ok(transaction)
	}
}

/// Song node as messages source.
type SongSourceClient = SubstrateMessagesSource<Song, SongMessagesToTang>;

/// Tang node as messages target.
type TangTargetClient = SubstrateMessagesTarget<Tang, SongMessagesToTang>;

/// Run Song-to-Tang messages sync.
pub fn run(
	song_client: SongClient,
	song_sign: SongSigningParams,
	tang_client: TangClient,
	tang_sign: TangSigningParams,
	lane_id: LaneId,
	metrics_params: Option<MetricsParams>,
) {
	let reconnect_delay = Duration::from_secs(10);
	let stall_timeout = Duration::from_secs(5 * 60);
	let relayer_id_at_song = song_sign.signer.public().as_array_ref().clone().into();

	let lane = SongMessagesToTang {
		source_client: song_client.clone(),
		source_sign: song_sign,
		target_client: tang_client.clone(),
		target_sign: tang_sign,
		relayer_id_at_source: relayer_id_at_song,
	};

	log::info!(
		target: "bridge",
		"Starting Song -> Tang messages relay. Song relayer account id: {:?}",
		lane.relayer_id_at_source,
	);

	messages_relay::message_lane_loop::run(
		messages_relay::message_lane_loop::Params {
			lane: lane_id,
			source_tick: Song::AVERAGE_BLOCK_INTERVAL,
			target_tick: Tang::AVERAGE_BLOCK_INTERVAL,
			reconnect_delay,
			stall_timeout,
			delivery_params: messages_relay::message_lane_loop::MessageDeliveryParams {
				max_unrewarded_relayer_entries_at_target:
					tang_node_primitives::MAX_UNREWARDED_RELAYER_ENTRIES_AT_INBOUND_LANE,
				max_unconfirmed_nonces_at_target:
					tang_node_primitives::MAX_UNCONFIRMED_MESSAGES_AT_INBOUND_LANE,
				max_messages_in_single_batch:
					tang_node_primitives::MAX_MESSAGES_IN_DELIVERY_TRANSACTION,
				// TODO: subtract base weight of delivery from this when it'll be known
				// https://github.com/paritytech/parity-bridges-common/issues/78
				max_messages_weight_in_single_batch: tang_node_primitives::MAXIMUM_EXTRINSIC_WEIGHT,
				// 2/3 is reserved for proofs and tx overhead
				max_messages_size_in_single_batch: tang_node_primitives::MAXIMUM_EXTRINSIC_SIZE
					as usize / 3,
			},
		},
		SongSourceClient::new(song_client, lane.clone(), lane_id, TANG_BRIDGE_INSTANCE),
		TangTargetClient::new(tang_client, lane, lane_id, SONG_BRIDGE_INSTANCE),
		metrics_params,
		futures::future::pending(),
	);
}
