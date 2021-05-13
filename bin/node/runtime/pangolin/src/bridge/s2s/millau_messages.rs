// This file is part of Darwinia.
//
// Copyright (C) 2018-2021 Darwinia Network
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

//! Everything required to serve Millau <-> Pangolin messages.

use crate::Runtime;

use bp_messages::{
	source_chain::TargetHeaderChain,
	target_chain::{ProvedMessages, SourceHeaderChain},
	InboundLaneData, LaneId, Message, MessageNonce, Parameter as MessagesParameter,
};
use bp_runtime::ChainId;
use bridge_runtime_common::messages::{self, ChainWithMessages, MessageBridge, MessageTransaction};
use codec::{Decode, Encode};
use frame_support::{
	parameter_types,
	weights::{DispatchClass, Weight},
	RuntimeDebug,
};
use pangolin_runtime_params::s2s as s2s_params;
use sp_core::storage::StorageKey;
use sp_runtime::traits::Zero;
use sp_runtime::{FixedPointNumber, FixedU128};
use sp_std::{convert::TryFrom, ops::RangeInclusive};

/// Initial value of `MillauToPangolinConversionRate` parameter.
pub const INITIAL_MILLAU_TO_PANGOLIN_CONVERSION_RATE: FixedU128 =
	FixedU128::from_inner(FixedU128::DIV);

parameter_types! {
	/// Millau to Rialto conversion rate. Initially we treat both tokens as equal.
	pub storage MillauToPangolinConversionRate: FixedU128 = INITIAL_MILLAU_TO_PANGOLIN_CONVERSION_RATE;
}

// fixme: reminder #1: now the darwinia-common use darwinia-network/substrate@darwinia-v0.10.0#3655f9b but the darwinia-network/parity-bridges-common use darwinia-network/substrate@s2s#97f1b63 , so have different substrate version, there can't compile now, the darwinia-common need upgrade substrate version
/// Storage key of the Rialto -> Millau message in the runtime storage.
pub fn message_key(lane: &LaneId, nonce: MessageNonce) -> StorageKey {
	pallet_bridge_messages::storage_keys::message_key::<
		Runtime,
		<PangolinChainWithMessage as ChainWithMessages>::MessagesInstance,
	>(lane, nonce)
}

/// Storage key of the Rialto -> Millau message lane state in the runtime storage.
pub fn outbound_lane_data_key(lane: &LaneId) -> StorageKey {
	pallet_bridge_messages::storage_keys::outbound_lane_data_key::<
		<PangolinChainWithMessage as ChainWithMessages>::MessagesInstance,
	>(lane)
}

/// Storage key of the Millau -> Rialto message lane state in the runtime storage.
pub fn inbound_lane_data_key(lane: &LaneId) -> StorageKey {
	pallet_bridge_messages::storage_keys::inbound_lane_data_key::<
		Runtime,
		<PangolinChainWithMessage as ChainWithMessages>::MessagesInstance,
	>(lane)
}

/// Message payload for Rialto -> Millau messages.
pub type ToMillauMessagePayload =
	messages::source::FromThisChainMessagePayload<WithMillauMessageBridge>;

/// Message verifier for Rialto -> Millau messages.
pub type ToMillauMessageVerifier =
	messages::source::FromThisChainMessageVerifier<WithMillauMessageBridge>;

/// Message payload for Millau -> Rialto messages.
pub type FromMillauMessagePayload =
	messages::target::FromBridgedChainMessagePayload<WithMillauMessageBridge>;

/// Encoded Rialto Call as it comes from Millau.
pub type FromMillauEncodedCall =
	messages::target::FromBridgedChainEncodedMessageCall<WithMillauMessageBridge>;

/// Call-dispatch based message dispatch for Millau -> Rialto messages.
pub type FromMillauMessageDispatch = messages::target::FromBridgedChainMessageDispatch<
	WithMillauMessageBridge,
	crate::Runtime,
	crate::WithMillauDispatchInstance,
>;

/// Messages proof for Millau -> Rialto messages.
pub type FromMillauMessagesProof = messages::target::FromBridgedChainMessagesProof<bp_millau::Hash>;

/// Messages delivery proof for Rialto -> Millau messages.
pub type ToMillauMessagesDeliveryProof =
	messages::source::FromBridgedChainMessagesDeliveryProof<bp_millau::Hash>;

/// Millau <-> Rialto message bridge.
#[derive(RuntimeDebug, Clone, Copy)]
pub struct WithMillauMessageBridge;

impl MessageBridge for WithMillauMessageBridge {
	const RELAYER_FEE_PERCENT: u32 = 10;

	type ThisChain = PangolinChainWithMessage;
	type BridgedChain = Millau;

	fn bridged_balance_to_this_balance(
		bridged_balance: bp_millau::Balance,
	) -> drml_primitives::Balance {
		drml_primitives::Balance::try_from(
			MillauToPangolinConversionRate::get().saturating_mul_int(bridged_balance),
		)
		.unwrap_or(drml_primitives::Balance::MAX)
	}
}

/// Rialto chain from message lane point of view.
#[derive(RuntimeDebug, Clone, Copy)]
pub struct PangolinChainWithMessage;

impl messages::ChainWithMessages for PangolinChainWithMessage {
	const ID: ChainId = bp_runtime::PANGOLIN_CHAIN_ID;

	type Hash = drml_primitives::Hash;
	type AccountId = drml_primitives::AccountId;
	type Signer = drml_primitives::AccountSigner;
	type Signature = drml_primitives::Signature;
	type Weight = Weight;
	type Balance = drml_primitives::Balance;

	type MessagesInstance = crate::WithMillauMessagesInstance;
}

impl messages::ThisChainWithMessages for PangolinChainWithMessage {
	type Call = crate::Call;

	fn is_outbound_lane_enabled(lane: &LaneId) -> bool {
		*lane == LaneId::default()
	}

	fn maximal_pending_messages_at_outbound_lane() -> MessageNonce {
		MessageNonce::MAX
	}

	fn estimate_delivery_confirmation_transaction() -> MessageTransaction<Weight> {
		let inbound_data_size = InboundLaneData::<crate::AccountId>::encoded_size_hint(
			s2s_params::MAXIMAL_ENCODED_ACCOUNT_ID_SIZE,
			1,
		)
		.unwrap_or(u32::MAX);

		MessageTransaction {
			dispatch_weight: s2s_params::MAX_SINGLE_MESSAGE_DELIVERY_CONFIRMATION_TX_WEIGHT,
			size: inbound_data_size
				.saturating_add(bp_millau::EXTRA_STORAGE_PROOF_SIZE)
				.saturating_add(s2s_params::TX_EXTRA_BYTES),
		}
	}

	fn transaction_payment(transaction: MessageTransaction<Weight>) -> drml_primitives::Balance {
		// in our testnets, both per-byte fee and weight-to-fee are 1:1
		messages::transaction_payment(
			pangolin_runtime_params::system::RuntimeBlockWeights::get()
				.get(DispatchClass::Normal)
				.base_extrinsic,
			1,
			FixedU128::zero(),
			|weight| weight as _,
			transaction,
		)
	}
}

/// Millau chain from message lane point of view.
#[derive(RuntimeDebug, Clone, Copy)]
pub struct Millau;

impl messages::ChainWithMessages for Millau {
	const ID: ChainId = bp_runtime::MILLAU_CHAIN_ID;

	type Hash = bp_millau::Hash;
	type AccountId = bp_millau::AccountId;
	type Signer = bp_millau::AccountSigner;
	type Signature = bp_millau::Signature;
	type Weight = Weight;
	type Balance = bp_millau::Balance;

	type MessagesInstance = crate::WithMillauMessagesInstance;
}

impl messages::BridgedChainWithMessages for Millau {
	fn maximal_extrinsic_size() -> u32 {
		bp_millau::max_extrinsic_size()
	}

	fn message_weight_limits(_message_payload: &[u8]) -> RangeInclusive<Weight> {
		// we don't want to relay too large messages + keep reserve for future upgrades
		let upper_limit = messages::target::maximal_incoming_message_dispatch_weight(
			bp_millau::max_extrinsic_weight(),
		);

		// we're charging for payload bytes in `WithMillauMessageBridge::transaction_payment` function
		//
		// this bridge may be used to deliver all kind of messages, so we're not making any assumptions about
		// minimal dispatch weight here

		0..=upper_limit
	}

	fn estimate_delivery_transaction(
		message_payload: &[u8],
		message_dispatch_weight: Weight,
	) -> MessageTransaction<Weight> {
		let message_payload_len = u32::try_from(message_payload.len()).unwrap_or(u32::MAX);
		let extra_bytes_in_payload = Weight::from(message_payload_len)
			.saturating_sub(pallet_bridge_messages::EXPECTED_DEFAULT_MESSAGE_LENGTH.into());

		MessageTransaction {
			dispatch_weight: extra_bytes_in_payload
				.saturating_mul(bp_millau::ADDITIONAL_MESSAGE_BYTE_DELIVERY_WEIGHT)
				.saturating_add(bp_millau::DEFAULT_MESSAGE_DELIVERY_TX_WEIGHT)
				.saturating_add(message_dispatch_weight),
			size: message_payload_len
				.saturating_add(s2s_params::EXTRA_STORAGE_PROOF_SIZE)
				.saturating_add(bp_millau::TX_EXTRA_BYTES),
		}
	}

	fn transaction_payment(transaction: MessageTransaction<Weight>) -> bp_millau::Balance {
		// fixme: same with reminder #1
		// in our testnets, both per-byte fee and weight-to-fee are 1:1
		messages::transaction_payment(
			bp_millau::BlockWeights::get()
				.get(DispatchClass::Normal)
				.base_extrinsic,
			1,
			FixedU128::zero(),
			|weight| weight as _,
			transaction,
		)
	}
}

impl TargetHeaderChain<ToMillauMessagePayload, bp_millau::AccountId> for Millau {
	type Error = &'static str;
	// The proof is:
	// - hash of the header this proof has been created with;
	// - the storage proof of one or several keys;
	// - id of the lane we prove state of.
	type MessagesDeliveryProof = ToMillauMessagesDeliveryProof;

	fn verify_message(payload: &ToMillauMessagePayload) -> Result<(), Self::Error> {
		messages::source::verify_chain_message::<WithMillauMessageBridge>(payload)
	}

	fn verify_messages_delivery_proof(
		proof: Self::MessagesDeliveryProof,
	) -> Result<(LaneId, InboundLaneData<drml_primitives::AccountId>), Self::Error> {
		messages::source::verify_messages_delivery_proof::<
			WithMillauMessageBridge,
			Runtime,
			crate::WithMillauGrandpaInstance,
		>(proof)
	}
}

impl SourceHeaderChain<bp_millau::Balance> for Millau {
	type Error = &'static str;
	// The proof is:
	// - hash of the header this proof has been created with;
	// - the storage proof of one or several keys;
	// - id of the lane we prove messages for;
	// - inclusive range of messages nonces that are proved.
	type MessagesProof = FromMillauMessagesProof;

	fn verify_messages_proof(
		proof: Self::MessagesProof,
		messages_count: u32,
	) -> Result<ProvedMessages<Message<bp_millau::Balance>>, Self::Error> {
		messages::target::verify_messages_proof::<
			WithMillauMessageBridge,
			Runtime,
			crate::WithMillauGrandpaInstance,
		>(proof, messages_count)
	}
}

/// Rialto -> Millau message lane pallet parameters.
#[derive(RuntimeDebug, Clone, Encode, Decode, PartialEq, Eq)]
pub enum RialtoToMillauMessagesParameter {
	/// The conversion formula we use is: `RialtoTokens = MillauTokens * conversion_rate`.
	MillauToPangolinConversionRate(FixedU128),
}

impl MessagesParameter for RialtoToMillauMessagesParameter {
	fn save(&self) {
		match *self {
			RialtoToMillauMessagesParameter::MillauToPangolinConversionRate(
				ref conversion_rate,
			) => MillauToPangolinConversionRate::set(conversion_rate),
		}
	}
}
