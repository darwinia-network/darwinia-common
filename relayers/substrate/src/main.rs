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

//! Substrate-to-substrate relay entrypoint.

#![warn(missing_docs)]

use codec::Encode;
use frame_support::weights::GetDispatchInfo;
use pallet_bridge_call_dispatch::{CallOrigin, MessagePayload};
use relay_song_client::{SigningParams as SongSigningParams, Song};
use relay_substrate_client::{ConnectionParams, TransactionSignScheme};
use relay_tang_client::{SigningParams as TangSigningParams, Tang};
use relay_utils::initialize::initialize_relay;
use sp_core::{Bytes, Pair};
use sp_runtime::traits::IdentifyAccount;

/// Tang node client.
pub type TangClient = relay_substrate_client::Client<Tang>;
/// Song node client.
pub type SongClient = relay_substrate_client::Client<Song>;

mod cli;
mod headers_initialize;
mod headers_maintain;
mod headers_pipeline;
mod headers_target;
mod messages_lane;
mod messages_source;
mod messages_target;
mod song_headers_to_tang;
mod song_messages_to_tang;
mod tang_headers_to_song;
mod tang_messages_to_song;

fn main() {
	initialize_relay();

	let result = async_std::task::block_on(run_command(cli::parse_args()));
	if let Err(error) = result {
		log::error!(target: "bridge", "Failed to start relay: {}", error);
	}
}

async fn run_command(command: cli::Command) -> Result<(), String> {
	match command {
		cli::Command::InitializeTangHeadersBridgeInSong {
			tang,
			song,
			song_sign,
			tang_bridge_params,
		} => {
			let tang_client = TangClient::new(ConnectionParams {
				host: tang.tang_host,
				port: tang.tang_port,
			})
			.await?;
			let song_client = SongClient::new(ConnectionParams {
				host: song.song_host,
				port: song.song_port,
			})
			.await?;
			let song_sign = SongSigningParams::from_suri(
				&song_sign.song_signer,
				song_sign.song_signer_password.as_deref(),
			)
			.map_err(|e| format!("Failed to parse song-signer: {:?}", e))?;
			let song_signer_next_index = song_client
				.next_account_index(song_sign.signer.public().into())
				.await?;

			headers_initialize::initialize(
				tang_client,
				song_client.clone(),
				tang_bridge_params.tang_initial_header,
				tang_bridge_params.tang_initial_authorities,
				tang_bridge_params.tang_initial_authorities_set_id,
				move |initialization_data| {
					Ok(Bytes(
						Song::sign_transaction(
							&song_client,
							&song_sign.signer,
							song_signer_next_index,
							tang_node_runtime::SudoCall::sudo(Box::new(
								song_node_runtime::BridgeTangCall::initialize(initialization_data)
									.into(),
							))
							.into(),
						)
						.encode(),
					))
				},
			)
			.await;
		}
		cli::Command::TangHeadersToSong {
			tang,
			song,
			song_sign,
			prometheus_params,
		} => {
			let tang_client = TangClient::new(ConnectionParams {
				host: tang.tang_host,
				port: tang.tang_port,
			})
			.await?;
			let song_client = SongClient::new(ConnectionParams {
				host: song.song_host,
				port: song.song_port,
			})
			.await?;
			let song_sign = SongSigningParams::from_suri(
				&song_sign.song_signer,
				song_sign.song_signer_password.as_deref(),
			)
			.map_err(|e| format!("Failed to parse song-signer: {:?}", e))?;
			tang_headers_to_song::run(
				tang_client,
				song_client,
				song_sign,
				prometheus_params.into(),
			)
			.await;
		}
		cli::Command::InitializeSongHeadersBridgeInTang {
			song,
			tang,
			tang_sign,
			song_bridge_params,
		} => {
			let song_client = SongClient::new(ConnectionParams {
				host: song.song_host,
				port: song.song_port,
			})
			.await?;
			let tang_client = TangClient::new(ConnectionParams {
				host: tang.tang_host,
				port: tang.tang_port,
			})
			.await?;
			let tang_sign = TangSigningParams::from_suri(
				&tang_sign.tang_signer,
				tang_sign.tang_signer_password.as_deref(),
			)
			.map_err(|e| format!("Failed to parse tang-signer: {:?}", e))?;
			let tang_signer_next_index = tang_client
				.next_account_index(tang_sign.signer.public().into())
				.await?;

			headers_initialize::initialize(
				song_client,
				tang_client.clone(),
				song_bridge_params.song_initial_header,
				song_bridge_params.song_initial_authorities,
				song_bridge_params.song_initial_authorities_set_id,
				move |initialization_data| {
					Ok(Bytes(
						Tang::sign_transaction(
							&tang_client,
							&tang_sign.signer,
							tang_signer_next_index,
							tang_node_runtime::SudoCall::sudo(Box::new(
								tang_node_runtime::BridgeSongCall::initialize(initialization_data)
									.into(),
							))
							.into(),
						)
						.encode(),
					))
				},
			)
			.await;
		}
		cli::Command::SongHeadersToTang {
			song,
			tang,
			tang_sign,
			prometheus_params,
		} => {
			let song_client = SongClient::new(ConnectionParams {
				host: song.song_host,
				port: song.song_port,
			})
			.await?;
			let tang_client = TangClient::new(ConnectionParams {
				host: tang.tang_host,
				port: tang.tang_port,
			})
			.await?;
			let tang_sign = TangSigningParams::from_suri(
				&tang_sign.tang_signer,
				tang_sign.tang_signer_password.as_deref(),
			)
			.map_err(|e| format!("Failed to parse tang-signer: {:?}", e))?;

			song_headers_to_tang::run(
				song_client,
				tang_client,
				tang_sign,
				prometheus_params.into(),
			)
			.await;
		}
		cli::Command::TangMessagesToSong {
			tang,
			tang_sign,
			song,
			song_sign,
			prometheus_params,
			lane,
		} => {
			let tang_client = TangClient::new(ConnectionParams {
				host: tang.tang_host,
				port: tang.tang_port,
			})
			.await?;
			let tang_sign = TangSigningParams::from_suri(
				&tang_sign.tang_signer,
				tang_sign.tang_signer_password.as_deref(),
			)
			.map_err(|e| format!("Failed to parse tang-signer: {:?}", e))?;
			let song_client = SongClient::new(ConnectionParams {
				host: song.song_host,
				port: song.song_port,
			})
			.await?;
			let song_sign = SongSigningParams::from_suri(
				&song_sign.song_signer,
				song_sign.song_signer_password.as_deref(),
			)
			.map_err(|e| format!("Failed to parse song-signer: {:?}", e))?;

			tang_messages_to_song::run(
				tang_client,
				tang_sign,
				song_client,
				song_sign,
				lane.into(),
				prometheus_params.into(),
			);
		}
		cli::Command::SubmitTangToSongMessage {
			tang,
			tang_sign,
			song_sign,
			lane,
			message,
			fee,
			origin,
			..
		} => {
			let tang_client = TangClient::new(ConnectionParams {
				host: tang.tang_host,
				port: tang.tang_port,
			})
			.await?;
			let tang_sign = TangSigningParams::from_suri(
				&tang_sign.tang_signer,
				tang_sign.tang_signer_password.as_deref(),
			)
			.map_err(|e| format!("Failed to parse tang-signer: {:?}", e))?;
			let song_sign = SongSigningParams::from_suri(
				&song_sign.song_signer,
				song_sign.song_signer_password.as_deref(),
			)
			.map_err(|e| format!("Failed to parse song-signer: {:?}", e))?;

			let song_call = match message {
				cli::ToSongMessage::Remark => {
					song_node_runtime::Call::System(song_node_runtime::SystemCall::remark(
						format!(
							"Unix time: {}",
							std::time::SystemTime::now()
								.duration_since(std::time::SystemTime::UNIX_EPOCH)
								.unwrap_or_default()
								.as_secs(),
						)
						.as_bytes()
						.to_vec(),
					))
				}
				cli::ToSongMessage::Transfer { recipient, amount } => {
					song_node_runtime::Call::Balances(song_node_runtime::BalancesCall::transfer(
						recipient, amount,
					))
				}
			};

			let song_call_weight = song_call.get_dispatch_info().weight;
			let tang_sender_public: tang_node_primitives::AccountSigner =
				tang_sign.signer.public().clone().into();
			let tang_account_id: tang_node_primitives::AccountId =
				tang_sender_public.into_account();
			let song_origin_public = song_sign.signer.public();

			let payload = match origin {
				cli::Origins::Root => MessagePayload {
					spec_version: song_node_runtime::VERSION.spec_version,
					weight: song_call_weight,
					origin: CallOrigin::SourceRoot,
					call: song_call.encode(),
				},
				cli::Origins::Source => MessagePayload {
					spec_version: song_node_runtime::VERSION.spec_version,
					weight: song_call_weight,
					origin: CallOrigin::SourceAccount(tang_account_id),
					call: song_call.encode(),
				},
				cli::Origins::Target => {
					let mut song_origin_signature_message = Vec::new();
					song_call.encode_to(&mut song_origin_signature_message);
					tang_account_id.encode_to(&mut song_origin_signature_message);
					let song_origin_signature =
						song_sign.signer.sign(&song_origin_signature_message);

					MessagePayload {
						spec_version: song_node_runtime::VERSION.spec_version,
						weight: song_call_weight,
						origin: CallOrigin::TargetAccount(
							tang_account_id.clone(),
							song_origin_public.into(),
							song_origin_signature.into(),
						),
						call: song_call.encode(),
					}
				}
			};

			let tang_call = tang_node_runtime::Call::BridgeSongMessageLane(
				tang_node_runtime::MessageLaneCall::send_message(lane.into(), payload, fee),
			);

			let signed_tang_call = Tang::sign_transaction(
				&tang_client,
				&tang_sign.signer,
				tang_client
					.next_account_index(tang_sign.signer.public().clone().into())
					.await?,
				tang_call,
			);

			tang_client
				.submit_extrinsic(Bytes(signed_tang_call.encode()))
				.await?;
		}
		cli::Command::SongMessagesToTang {
			song,
			song_sign,
			tang,
			tang_sign,
			prometheus_params,
			lane,
		} => {
			let song_client = SongClient::new(ConnectionParams {
				host: song.song_host,
				port: song.song_port,
			})
			.await?;
			let song_sign = SongSigningParams::from_suri(
				&song_sign.song_signer,
				song_sign.song_signer_password.as_deref(),
			)
			.map_err(|e| format!("Failed to parse song-signer: {:?}", e))?;
			let tang_client = TangClient::new(ConnectionParams {
				host: tang.tang_host,
				port: tang.tang_port,
			})
			.await?;
			let tang_sign = TangSigningParams::from_suri(
				&tang_sign.tang_signer,
				tang_sign.tang_signer_password.as_deref(),
			)
			.map_err(|e| format!("Failed to parse tang-signer: {:?}", e))?;

			song_messages_to_tang::run(
				song_client,
				song_sign,
				tang_client,
				tang_sign,
				lane.into(),
				prometheus_params.into(),
			);
		}
		cli::Command::SubmitSongToTangMessage {
			song,
			song_sign,
			tang_sign,
			lane,
			message,
			fee,
		} => {
			let song_client = SongClient::new(ConnectionParams {
				host: song.song_host,
				port: song.song_port,
			})
			.await?;
			let song_sign = SongSigningParams::from_suri(
				&song_sign.song_signer,
				song_sign.song_signer_password.as_deref(),
			)
			.map_err(|e| format!("Failed to parse song-signer: {:?}", e))?;
			let tang_sign = TangSigningParams::from_suri(
				&tang_sign.tang_signer,
				tang_sign.tang_signer_password.as_deref(),
			)
			.map_err(|e| format!("Failed to parse tang-signer: {:?}", e))?;

			let tang_call = match message {
				cli::ToTangMessage::Remark => {
					tang_node_runtime::Call::System(tang_node_runtime::SystemCall::remark(
						format!(
							"Unix time: {}",
							std::time::SystemTime::now()
								.duration_since(std::time::SystemTime::UNIX_EPOCH)
								.unwrap_or_default()
								.as_secs(),
						)
						.as_bytes()
						.to_vec(),
					))
				}
			};
			let tang_call_weight = tang_call.get_dispatch_info().weight;

			let song_sender_public: song_node_primitives::AccountSigner =
				song_sign.signer.public().clone().into();
			let song_account_id: song_node_primitives::AccountId =
				song_sender_public.into_account();
			let tang_origin_public = tang_sign.signer.public();

			let mut tang_origin_signature_message = Vec::new();
			tang_call.encode_to(&mut tang_origin_signature_message);
			song_account_id.encode_to(&mut tang_origin_signature_message);
			let tang_origin_signature = tang_sign.signer.sign(&tang_origin_signature_message);

			let song_call = song_node_runtime::Call::BridgeTangMessageLane(
				song_node_runtime::MessageLaneCall::send_message(
					lane.into(),
					MessagePayload {
						spec_version: tang_node_runtime::VERSION.spec_version,
						weight: tang_call_weight,
						origin: CallOrigin::TargetAccount(
							song_account_id,
							tang_origin_public.into(),
							tang_origin_signature.into(),
						),
						call: tang_call.encode(),
					},
					fee,
				),
			);

			let signed_song_call = Song::sign_transaction(
				&song_client,
				&song_sign.signer,
				song_client
					.next_account_index(song_sign.signer.public().clone().into())
					.await?,
				song_call,
			);

			song_client
				.submit_extrinsic(Bytes(signed_song_call.encode()))
				.await?;
		}
	}

	Ok(())
}
