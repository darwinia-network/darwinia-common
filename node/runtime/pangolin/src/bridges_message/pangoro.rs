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

// --- crates.io ---
use codec::{Decode, Encode};
use scale_info::TypeInfo;
// --- paritytech ---
use frame_support::{
	weights::{DispatchClass, Weight},
	RuntimeDebug,
};
use sp_runtime::{traits::Zero, FixedPointNumber, FixedU128};
use sp_std::{convert::TryFrom, ops::RangeInclusive};
// --- darwinia-network ---
use crate::*;
use bp_message_dispatch::CallOrigin;
use bp_messages::{source_chain::*, target_chain::*, *};
use bp_runtime::{messages::DispatchFeePayment, ChainId, *};
use bridge_runtime_common::messages::{
	self,
	source::{self, *},
	target::{self, *},
	*,
};
use dp_s2s::{CallParams, CreatePayload};
use drml_common_runtime::impls::FromThisChainMessageVerifier;
use pallet_bridge_messages::EXPECTED_DEFAULT_MESSAGE_LENGTH;

/// Messages delivery proof for Pangolin -> Pangoro messages.
type ToPangoroMessagesDeliveryProof = FromBridgedChainMessagesDeliveryProof<bp_pangoro::Hash>;
/// Messages proof for Pangoro -> Pangolin messages.
type FromPangoroMessagesProof = FromBridgedChainMessagesProof<bp_pangoro::Hash>;

/// Message payload for Pangolin -> Pangoro messages.
pub type ToPangoroMessagePayload = FromThisChainMessagePayload<WithPangoroMessageBridge>;
/// Message payload for Pangoro -> Pangolin messages.
pub type FromPangoroMessagePayload = FromBridgedChainMessagePayload<WithPangoroMessageBridge>;

/// Message verifier for Pangolin -> Pangoro messages.
pub type ToPangoroMessageVerifier<R> = FromThisChainMessageVerifier<WithPangoroMessageBridge, R>;

/// Encoded Pangolin Call as it comes from Pangoro.
pub type FromPangoroEncodedCall = FromBridgedChainEncodedMessageCall<crate::Call>;

/// Call-dispatch based message dispatch for Pangoro -> Pangolin messages.
pub type FromPangoroMessageDispatch =
	FromBridgedChainMessageDispatch<WithPangoroMessageBridge, Runtime, Ring, WithPangoroDispatch>;

/// The s2s backing pallet index in the pangoro chain runtime.
pub const PANGORO_S2S_BACKING_PALLET_INDEX: u8 = 20;

/// Initial value of `PangoroToPangolinConversionRate` parameter.
pub const INITIAL_PANGORO_TO_PANGOLIN_CONVERSION_RATE: FixedU128 =
	FixedU128::from_inner(FixedU128::DIV);

frame_support::parameter_types! {
	/// Pangoro to Pangolin conversion rate. Initially we treat both tokens as equal.
	pub storage PangoroToPangolinConversionRate: FixedU128 = INITIAL_PANGORO_TO_PANGOLIN_CONVERSION_RATE;
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct ToPangoroOutboundPayLoad;
impl CreatePayload<AccountId, AccountPublic, Signature> for ToPangoroOutboundPayLoad {
	type Payload = ToPangoroMessagePayload;

	fn create(
		origin: CallOrigin<AccountId, AccountPublic, Signature>,
		spec_version: u32,
		weight: u64,
		call_params: CallParams,
		dispatch_fee_payment: DispatchFeePayment,
	) -> Result<Self::Payload, &'static str> {
		let call = Self::encode_call(PANGORO_S2S_BACKING_PALLET_INDEX, call_params)?;
		Ok(ToPangoroMessagePayload {
			spec_version,
			weight,
			origin,
			call,
			dispatch_fee_payment,
		})
	}
}

/// Pangolin -> Pangoro message lane pallet parameters.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum PangolinToPangoroMessagesParameter {
	/// The conversion formula we use is: `PangolinTokens = PangoroTokens * conversion_rate`.
	PangoroToPangolinConversionRate(FixedU128),
}
impl Parameter for PangolinToPangoroMessagesParameter {
	fn save(&self) {
		match *self {
			PangolinToPangoroMessagesParameter::PangoroToPangolinConversionRate(
				ref conversion_rate,
			) => PangoroToPangolinConversionRate::set(conversion_rate),
		}
	}
}

/// Pangoro <-> Pangolin message bridge.
#[derive(Clone, Copy, RuntimeDebug)]
pub struct WithPangoroMessageBridge;
impl MessageBridge for WithPangoroMessageBridge {
	const RELAYER_FEE_PERCENT: u32 = 10;
	const THIS_CHAIN_ID: ChainId = PANGOLIN_CHAIN_ID;
	const BRIDGED_CHAIN_ID: ChainId = PANGORO_CHAIN_ID;
	const BRIDGED_MESSAGES_PALLET_NAME: &'static str =
		bp_pangolin::WITH_PANGOLIN_MESSAGES_PALLET_NAME;

	type ThisChain = Pangolin;
	type BridgedChain = Pangoro;

	fn bridged_balance_to_this_balance(bridged_balance: bp_pangoro::Balance) -> Balance {
		Balance::try_from(
			PangoroToPangolinConversionRate::get().saturating_mul_int(bridged_balance),
		)
		.unwrap_or(Balance::MAX)
	}
}

/// Pangolin chain from message lane point of view.
#[derive(Clone, Copy, RuntimeDebug)]
pub struct Pangolin;
impl messages::ChainWithMessages for Pangolin {
	type Hash = Hash;
	type AccountId = AccountId;
	type Signer = AccountPublic;
	type Signature = Signature;
	type Weight = Weight;
	type Balance = Balance;
}
impl messages::ThisChainWithMessages for Pangolin {
	type Call = Call;

	fn is_outbound_lane_enabled(lane: &LaneId) -> bool {
		*lane == [0, 0, 0, 0] || *lane == [0, 0, 0, 1] || *lane == PANGORO_PANGOLIN_LANE
	}

	fn maximal_pending_messages_at_outbound_lane() -> MessageNonce {
		MessageNonce::MAX
	}

	fn estimate_delivery_confirmation_transaction() -> MessageTransaction<Weight> {
		let inbound_data_size = InboundLaneData::<AccountId>::encoded_size_hint(
			bp_pangolin::MAXIMAL_ENCODED_ACCOUNT_ID_SIZE,
			1,
			1,
		)
		.unwrap_or(u32::MAX);

		MessageTransaction {
			dispatch_weight: bp_pangolin::MAX_SINGLE_MESSAGE_DELIVERY_CONFIRMATION_TX_WEIGHT,
			size: inbound_data_size
				.saturating_add(bp_pangolin::EXTRA_STORAGE_PROOF_SIZE)
				.saturating_add(bp_pangolin::TX_EXTRA_BYTES),
		}
	}

	fn transaction_payment(transaction: MessageTransaction<Weight>) -> Balance {
		// in our testnets, both per-byte fee and weight-to-fee are 1:1
		messages::transaction_payment(
			RuntimeBlockWeights::get()
				.get(DispatchClass::Normal)
				.base_extrinsic,
			1,
			FixedU128::zero(),
			|weight| weight as _,
			transaction,
		)
	}
}

/// Pangoro chain from message lane point of view.
#[derive(Clone, Copy, RuntimeDebug)]
pub struct Pangoro;
impl messages::ChainWithMessages for Pangoro {
	type Hash = bp_pangoro::Hash;
	type AccountId = bp_pangoro::AccountId;
	type Signer = bp_pangoro::AccountPublic;
	type Signature = bp_pangoro::Signature;
	type Weight = Weight;
	type Balance = bp_pangoro::Balance;
}
impl messages::BridgedChainWithMessages for Pangoro {
	fn maximal_extrinsic_size() -> u32 {
		bp_pangoro::Pangoro::max_extrinsic_size()
	}

	fn message_weight_limits(_message_payload: &[u8]) -> RangeInclusive<Weight> {
		// we don't want to relay too large messages + keep reserve for future upgrades
		let upper_limit = messages::target::maximal_incoming_message_dispatch_weight(
			bp_pangoro::Pangoro::max_extrinsic_weight(),
		);

		// we're charging for payload bytes in `WithPangoroMessageBridge::transaction_payment` function
		//
		// this bridge may be used to deliver all kind of messages, so we're not making any assumptions about
		// minimal dispatch weight here

		0..=upper_limit
	}

	fn estimate_delivery_transaction(
		message_payload: &[u8],
		include_pay_dispatch_fee_cost: bool,
		message_dispatch_weight: Weight,
	) -> MessageTransaction<Weight> {
		let message_payload_len = u32::try_from(message_payload.len()).unwrap_or(u32::MAX);
		let extra_bytes_in_payload = Weight::from(message_payload_len)
			.saturating_sub(EXPECTED_DEFAULT_MESSAGE_LENGTH.into());

		MessageTransaction {
			dispatch_weight: extra_bytes_in_payload
				.saturating_mul(bp_pangolin::ADDITIONAL_MESSAGE_BYTE_DELIVERY_WEIGHT)
				.saturating_add(bp_pangolin::DEFAULT_MESSAGE_DELIVERY_TX_WEIGHT)
				.saturating_add(message_dispatch_weight)
				.saturating_sub(if include_pay_dispatch_fee_cost {
					0
				} else {
					bp_pangolin::PAY_INBOUND_DISPATCH_FEE_WEIGHT
				}),
			size: message_payload_len
				.saturating_add(bp_pangolin::EXTRA_STORAGE_PROOF_SIZE)
				.saturating_add(bp_pangolin::TX_EXTRA_BYTES),
		}
	}

	fn transaction_payment(transaction: MessageTransaction<Weight>) -> bp_pangoro::Balance {
		// in our testnets, both per-byte fee and weight-to-fee are 1:1
		messages::transaction_payment(
			bp_pangoro::RuntimeBlockWeights::get()
				.get(DispatchClass::Normal)
				.base_extrinsic,
			1,
			FixedU128::zero(),
			|weight| weight as _,
			transaction,
		)
	}
}
impl TargetHeaderChain<ToPangoroMessagePayload, bp_pangoro::AccountId> for Pangoro {
	type Error = &'static str;
	// The proof is:
	// - hash of the header this proof has been created with;
	// - the storage proof or one or several keys;
	// - id of the lane we prove state of.
	type MessagesDeliveryProof = ToPangoroMessagesDeliveryProof;

	fn verify_message(payload: &ToPangoroMessagePayload) -> Result<(), Self::Error> {
		source::verify_chain_message::<WithPangoroMessageBridge>(payload)
	}

	fn verify_messages_delivery_proof(
		proof: Self::MessagesDeliveryProof,
	) -> Result<(LaneId, InboundLaneData<AccountId>), Self::Error> {
		source::verify_messages_delivery_proof::<
			WithPangoroMessageBridge,
			Runtime,
			WithPangoroGrandpa,
		>(proof)
	}
}
impl SourceHeaderChain<bp_pangoro::Balance> for Pangoro {
	type Error = &'static str;
	// The proof is:
	// - hash of the header this proof has been created with;
	// - the storage proof or one or several keys;
	// - id of the lane we prove messages for;
	// - inclusive range of messages nonces that are proved.
	type MessagesProof = FromPangoroMessagesProof;

	fn verify_messages_proof(
		proof: Self::MessagesProof,
		messages_count: u32,
	) -> Result<ProvedMessages<Message<bp_pangoro::Balance>>, Self::Error> {
		target::verify_messages_proof::<WithPangoroMessageBridge, Runtime, WithPangoroGrandpa>(
			proof,
			messages_count,
		)
	}
}
