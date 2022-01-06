// This file is part of Darwinia.
//
// Copyright (C) 2018-2022 Darwinia Network
// SPDX-License-Identifier: GPL-3.0
//
// Darwinia is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Darwinia is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

#![cfg_attr(not(feature = "std"), no_std)]

// --- core ---
use core::marker::PhantomData;
// --- paritytech ---
use bp_messages::{
	source_chain::{LaneMessageVerifier, Sender},
	LaneId, MessageDetails, MessageNonce, OutboundLaneData, UnrewardedRelayersState,
};
use bp_runtime::{Chain, ChainId};
use bridge_runtime_common::messages::{
	source::{
		FromThisChainMessagePayload, BAD_ORIGIN, OUTBOUND_LANE_DISABLED, TOO_LOW_FEE,
		TOO_MANY_PENDING_MESSAGES,
	},
	AccountIdOf, BalanceOf, MessageBridge, ThisChain, ThisChainWithMessages,
};
use frame_support::{weights::Weight, Parameter};
use sp_core::H256;
use sp_runtime::{traits::Convert, RuntimeDebug};
use sp_std::prelude::*;
// --- darwinia-network ---
use darwinia_fee_market::RingBalance;
use drml_common_primitives::*;

/// Maximal size (in bytes) of encoded (using `Encode::encode()`) account id.
pub const MAXIMAL_ENCODED_ACCOUNT_ID_SIZE: u32 = 32;

/// Number of extra bytes (excluding size of storage value itself) of storage proof, built at
/// Pangolin chain. This mostly depends on number of entries (and their density) in the storage trie.
/// Some reserve is reserved to account future chain growth.
pub const EXTRA_STORAGE_PROOF_SIZE: u32 = 1024;

/// Number of bytes, included in the signed Pangolin transaction apart from the encoded call itself.
///
/// Can be computed by subtracting encoded call size from raw transaction size.
pub const TX_EXTRA_BYTES: u32 = 103;
/// Increase of delivery transaction weight on Pangolin chain with every additional message byte.
///
/// This value is a result of `pallet_bridge_messages::WeightInfoExt::storage_proof_size_overhead(1)` call. The
/// result then must be rounded up to account possible future runtime upgrades.
pub const ADDITIONAL_MESSAGE_BYTE_DELIVERY_WEIGHT: Weight = 25_000;
/// Weight of single regular message delivery transaction on Pangolin chain.
///
/// This value is a result of `pallet_bridge_messages::Pallet::receive_messages_proof_weight()` call
/// for the case when single message of `pallet_bridge_messages::EXPECTED_DEFAULT_MESSAGE_LENGTH` bytes is delivered.
/// The message must have dispatch weight set to zero. The result then must be rounded up to account
/// possible future runtime upgrades.
pub const DEFAULT_MESSAGE_DELIVERY_TX_WEIGHT: Weight = 1_000_000_000;
/// Maximal weight of single message delivery confirmation transaction on Pangolin chain.
///
/// This value is a result of `pallet_bridge_messages::Pallet::receive_messages_delivery_proof` weight formula computation
/// for the case when single message is confirmed. The result then must be rounded up to account possible future
/// runtime upgrades.
pub const MAX_SINGLE_MESSAGE_DELIVERY_CONFIRMATION_TX_WEIGHT: Weight = 2_000_000_000;
pub const PAY_INBOUND_DISPATCH_FEE_WEIGHT: Weight = 600_000_000;

/// Maximal number of unrewarded relayer entries at inbound lane.
pub const MAX_UNREWARDED_RELAYER_ENTRIES_AT_INBOUND_LANE: MessageNonce = 128;
/// Maximal number of unconfirmed messages at inbound lane.
pub const MAX_UNCONFIRMED_MESSAGES_AT_INBOUND_LANE: MessageNonce = 128;

// 726f6c69
pub const PANGORO_PANGOLIN_LANE: [u8; 4] = *b"roli";

// === Pangolin const define
/// Bridge-with-Pangolin instance id.
pub const PANGOLIN_CHAIN_ID: ChainId = *b"pagl";

/// Name of the With-Pangoro messages pallet instance in the Pangolin runtime.
pub const WITH_PANGORO_MESSAGES_PALLET_NAME: &str = "BridgePangoroMessages";

/// Name of the `FromPangolinInboundLaneApi::latest_received_nonce` runtime method.
pub const FROM_PANGOLIN_LATEST_RECEIVED_NONCE_METHOD: &str =
	"FromPangolinInboundLaneApi_latest_received_nonce";
/// Name of the `FromPangolinInboundLaneApi::latest_confirmed_nonce` runtime method.
pub const FROM_PANGOLIN_LATEST_CONFIRMED_NONCE_METHOD: &str =
	"FromPangolinInboundLaneApi_latest_confirmed_nonce";
/// Name of the `FromPangolinInboundLaneApi::unrewarded_relayers_state` runtime method.
pub const FROM_PANGOLIN_UNREWARDED_RELAYERS_STATE: &str =
	"FromPangolinInboundLaneApi_unrewarded_relayers_state";

// /// Name of the `ToPangolinOutboundLaneApi::estimate_message_delivery_and_dispatch_fee` runtime method.
// pub const TO_PANGOLIN_ESTIMATE_MESSAGE_FEE_METHOD: &str =
// 	"ToPangolinOutboundLaneApi_estimate_message_delivery_and_dispatch_fee";
/// Name of the `ToPangolinOutboundLaneApi::message_details` runtime method.
pub const TO_PANGOLIN_MESSAGE_DETAILS_METHOD: &str = "ToPangolinOutboundLaneApi_message_details";
/// Name of the `ToPangolinOutboundLaneApi::latest_generated_nonce` runtime method.
pub const TO_PANGOLIN_LATEST_GENERATED_NONCE_METHOD: &str =
	"ToPangolinOutboundLaneApi_latest_generated_nonce";
/// Name of the `ToPangolinOutboundLaneApi::latest_received_nonce` runtime method.
pub const TO_PANGOLIN_LATEST_RECEIVED_NONCE_METHOD: &str =
	"ToPangolinOutboundLaneApi_latest_received_nonce";

/// Name of the `PangolinFinalityApi::best_finalized` runtime method.
pub const BEST_FINALIZED_PANGOLIN_HEADER_METHOD: &str = "PangolinFinalityApi_best_finalized";
// === end

// === Pangoro const define
/// Bridge-with-Pangoro instance id.
pub const PANGORO_CHAIN_ID: ChainId = *b"pagr";

/// Name of the With-Pangolin messages pallet instance in the Pangoro runtime.
pub const WITH_PANGOLIN_MESSAGES_PALLET_NAME: &str = "BridgePangolinMessages";

/// Name of the `FromPangoroInboundLaneApi::latest_received_nonce` runtime method.
pub const FROM_PANGORO_LATEST_RECEIVED_NONCE_METHOD: &str =
	"FromPangoroInboundLaneApi_latest_received_nonce";
/// Name of the `FromPangoroInboundLaneApi::latest_confirmed_nonce` runtime method.
pub const FROM_PANGORO_LATEST_CONFIRMED_NONCE_METHOD: &str =
	"FromPangoroInboundLaneApi_latest_confirmed_nonce";
/// Name of the `FromPangoroInboundLaneApi::unrewarded_relayers_state` runtime method.
pub const FROM_PANGORO_UNREWARDED_RELAYERS_STATE: &str =
	"FromPangoroInboundLaneApi_unrewarded_relayers_state";

// /// Name of the `ToPangoroOutboundLaneApi::estimate_message_delivery_and_dispatch_fee` runtime method.
// pub const TO_PANGORO_ESTIMATE_MESSAGE_FEE_METHOD: &str =
// 	"ToPangoroOutboundLaneApi_estimate_message_delivery_and_dispatch_fee";
/// Name of the `ToPangolinOutboundLaneApi::message_details` runtime method.
pub const TO_PANGORO_MESSAGE_DETAILS_METHOD: &str = "ToPangoroOutboundLaneApi_message_details";
/// Name of the `ToPangoroOutboundLaneApi::latest_generated_nonce` runtime method.
pub const TO_PANGORO_LATEST_GENERATED_NONCE_METHOD: &str =
	"ToPangoroOutboundLaneApi_latest_generated_nonce";
/// Name of the `ToPangoroOutboundLaneApi::latest_received_nonce` runtime method.
pub const TO_PANGORO_LATEST_RECEIVED_NONCE_METHOD: &str =
	"ToPangoroOutboundLaneApi_latest_received_nonce";

/// Name of the `PangoroFinalityApi::best_finalized` runtime method.
pub const BEST_FINALIZED_PANGORO_HEADER_METHOD: &str = "PangoroFinalityApi_best_finalized";
// === end

/// Convert a 256-bit hash into an AccountId.
pub struct AccountIdConverter;
impl Convert<H256, AccountId> for AccountIdConverter {
	fn convert(hash: H256) -> AccountId {
		hash.to_fixed_bytes().into()
	}
}
/// Pangoro chain.
#[derive(RuntimeDebug)]
pub struct Pangoro;
impl Chain for Pangoro {
	type BlockNumber = BlockNumber;
	type Hash = Hash;
	type Hasher = Hashing;
	type Header = Header;
	type AccountId = AccountId;
	type Balance = Balance;
	type Index = Nonce;
	type Signature = Signature;

	// TODO: S2S
	fn max_extrinsic_size() -> u32 {
		todo!()
	}

	// TODO: S2S
	fn max_extrinsic_weight() -> Weight {
		todo!()
	}
}

/// Pangolin chain.
#[derive(RuntimeDebug)]
pub struct Pangolin;
impl Chain for Pangolin {
	type BlockNumber = BlockNumber;
	type Hash = Hash;
	type Hasher = Hashing;
	type Header = Header;
	type AccountId = AccountId;
	type Balance = Balance;
	type Index = Nonce;
	type Signature = Signature;

	// TODO: S2S
	fn max_extrinsic_size() -> u32 {
		todo!()
	}

	// TODO: S2S
	fn max_extrinsic_weight() -> Weight {
		todo!()
	}
}

/// Message verifier that is doing all basic checks.
///
/// This verifier assumes following:
///
/// - all message lanes are equivalent, so all checks are the same;
/// - messages are being dispatched using `pallet-bridge-dispatch` pallet on the target chain.
///
/// Following checks are made:
///
/// - message is rejected if its lane is currently blocked;
/// - message is rejected if there are too many pending (undelivered) messages at the outbound
///   lane;
/// - check that the sender has rights to dispatch the call on target chain using provided
///   dispatch origin;
/// - check that the sender has paid enough funds for both message delivery and dispatch.
#[derive(RuntimeDebug)]
pub struct FromThisChainMessageVerifier<B, R>(PhantomData<(B, R)>);
impl<B, R>
	LaneMessageVerifier<
		AccountIdOf<ThisChain<B>>,
		FromThisChainMessagePayload<B>,
		BalanceOf<ThisChain<B>>,
	> for FromThisChainMessageVerifier<B, R>
where
	B: MessageBridge,
	R: darwinia_fee_market::Config,
	AccountIdOf<ThisChain<B>>: PartialEq + Clone,
	RingBalance<R>: From<BalanceOf<ThisChain<B>>>,
{
	type Error = &'static str;

	fn verify_message(
		submitter: &Sender<AccountIdOf<ThisChain<B>>>,
		delivery_and_dispatch_fee: &BalanceOf<ThisChain<B>>,
		lane: &LaneId,
		lane_outbound_data: &OutboundLaneData,
		payload: &FromThisChainMessagePayload<B>,
	) -> Result<(), Self::Error> {
		// reject message if lane is blocked
		if !ThisChain::<B>::is_outbound_lane_enabled(lane) {
			return Err(OUTBOUND_LANE_DISABLED);
		}

		// reject message if there are too many pending messages at this lane
		let max_pending_messages = ThisChain::<B>::maximal_pending_messages_at_outbound_lane();
		let pending_messages = lane_outbound_data
			.latest_generated_nonce
			.saturating_sub(lane_outbound_data.latest_received_nonce);
		if pending_messages > max_pending_messages {
			return Err(TOO_MANY_PENDING_MESSAGES);
		}

		// Do the dispatch-specific check. We assume that the target chain uses
		// `Dispatch`, so we verify the message accordingly.
		pallet_bridge_dispatch::verify_message_origin(submitter, payload)
			.map_err(|_| BAD_ORIGIN)?;

		// Do the delivery_and_dispatch_fee. We assume that the delivery and dispatch fee always
		// greater than the fee market provided fee.
		let message_fee: RingBalance<R> = (*delivery_and_dispatch_fee).into();
		if let Some(market_fee) = darwinia_fee_market::Pallet::<R>::market_fee() {
			// compare with actual fee paid
			if message_fee < market_fee {
				return Err(TOO_LOW_FEE);
			}
		} else {
			const NO_MARKET_FEE: &str = "The fee market are not ready for accepting messages.";

			return Err(NO_MARKET_FEE);
		}

		Ok(())
	}
}

sp_api::decl_runtime_apis! {
	/// API for querying information about the finalized Pangolin headers.
	///
	/// This API is implemented by runtimes that are bridging with the Pangolin chain, not the
	/// Pangoro runtime itself.
	pub trait PangolinFinalityApi {
		/// Returns number and hash of the best finalized header known to the bridge module.
		fn best_finalized() -> (BlockNumber, Hash);
		/// Returns true if the header is known to the runtime.
		fn is_known_header(hash: Hash) -> bool;
	}
	/// Outbound message lane API for messages that are sent to Pangolin chain.
	///
	/// This API is implemented by runtimes that are sending messages to Pangolin chain, not the
	/// Pangolin runtime itself.
	pub trait ToPangolinOutboundLaneApi<OutboundMessageFee: Parameter, OutboundPayload: Parameter> {
		// /// Estimate message delivery and dispatch fee that needs to be paid by the sender on
		// /// this chain.
		// ///
		// /// Returns `None` if message is too expensive to be sent to Pangolin from this chain.
		// ///
		// /// Please keep in mind that this method returns lowest message fee required for message
		// /// to be accepted to the lane. It may be good idea to pay a bit over this price to account
		// /// future exchange rate changes and guarantee that relayer would deliver your message
		// /// to the target chain.
		// fn estimate_message_delivery_and_dispatch_fee(
		// 	lane_id: LaneId,
		// 	payload: OutboundPayload,
		// ) -> Option<OutboundMessageFee>;
		/// Returns dispatch weight, encoded payload size and delivery+dispatch fee of all
		/// messages in given inclusive range.
		///
		/// If some (or all) messages are missing from the storage, they'll also will
		/// be missing from the resulting vector. The vector is ordered by the nonce.
		fn message_details(
			lane: LaneId,
			begin: MessageNonce,
			end: MessageNonce,
		) -> Vec<MessageDetails<OutboundMessageFee>>;
		/// Returns nonce of the latest message, received by bridged chain.
		fn latest_received_nonce(lane: LaneId) -> MessageNonce;
		/// Returns nonce of the latest message, generated by given lane.
		fn latest_generated_nonce(lane: LaneId) -> MessageNonce;
	}
	/// Inbound message lane API for messages sent by Pangolin chain.
	///
	/// This API is implemented by runtimes that are receiving messages from Pangolin chain, not the
	/// Pangolin runtime itself.
	pub trait FromPangolinInboundLaneApi {
		/// Returns nonce of the latest message, received by given lane.
		fn latest_received_nonce(lane: LaneId) -> MessageNonce;
		/// Nonce of latest message that has been confirmed to the bridged chain.
		fn latest_confirmed_nonce(lane: LaneId) -> MessageNonce;
		/// State of the unrewarded relayers set at given lane.
		fn unrewarded_relayers_state(lane: LaneId) -> UnrewardedRelayersState;
	}

	/// API for querying information about the finalized Pangoro headers.
	///
	/// This API is implemented by runtimes that are bridging with the Pangoro chain, not the
	/// Pangoro runtime itself.
	pub trait PangoroFinalityApi {
		/// Returns number and hash of the best finalized header known to the bridge module.
		fn best_finalized() -> (BlockNumber, Hash);
		/// Returns true if the header is known to the runtime.
		fn is_known_header(hash: Hash) -> bool;
	}
	/// Outbound message lane API for messages that are sent to Pangoro chain.
	///
	/// This API is implemented by runtimes that are sending messages to Pangoro chain, not the
	/// Pangoro runtime itself.
	pub trait ToPangoroOutboundLaneApi<OutboundMessageFee: Parameter, OutboundPayload: Parameter> {
		// /// Estimate message delivery and dispatch fee that needs to be paid by the sender on
		// /// this chain.
		// ///
		// /// Returns `None` if message is too expensive to be sent to Pangoro from this chain.
		// ///
		// /// Please keep in mind that this method returns lowest message fee required for message
		// /// to be accepted to the lane. It may be good idea to pay a bit over this price to account
		// /// future exchange rate changes and guarantee that relayer would deliver your message
		// /// to the target chain.
		// fn estimate_message_delivery_and_dispatch_fee(
		// 	lane_id: LaneId,
		// 	payload: OutboundPayload,
		// ) -> Option<OutboundMessageFee>;
		/// Returns dispatch weight, encoded payload size and delivery+dispatch fee of all
		/// messages in given inclusive range.
		///
		/// If some (or all) messages are missing from the storage, they'll also will
		/// be missing from the resulting vector. The vector is ordered by the nonce.
		fn message_details(
			lane: LaneId,
			begin: MessageNonce,
			end: MessageNonce,
		) -> Vec<MessageDetails<OutboundMessageFee>>;
		/// Returns nonce of the latest message, received by bridged chain.
		fn latest_received_nonce(lane: LaneId) -> MessageNonce;
		/// Returns nonce of the latest message, generated by given lane.
		fn latest_generated_nonce(lane: LaneId) -> MessageNonce;
	}
	/// Inbound message lane API for messages sent by Pangoro chain.
	///
	/// This API is implemented by runtimes that are receiving messages from Pangoro chain, not the
	/// Pangoro runtime itself.
	pub trait FromPangoroInboundLaneApi {
		/// Returns nonce of the latest message, received by given lane.
		fn latest_received_nonce(lane: LaneId) -> MessageNonce;
		/// Nonce of latest message that has been confirmed to the bridged chain.
		fn latest_confirmed_nonce(lane: LaneId) -> MessageNonce;
		/// State of the unrewarded relayers set at given lane.
		fn unrewarded_relayers_state(lane: LaneId) -> UnrewardedRelayersState;
	}
}
