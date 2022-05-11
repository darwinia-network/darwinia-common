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

//! Everything required to serve Pangoro <-> Pangolin messages.

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
use bp_runtime::{messages::*, ChainId, *};
use bridge_runtime_common::{
	lanes::*,
	messages::{
		self,
		source::{self, *},
		target::{self, *},
		BalanceOf, *,
	},
};
use dp_asset::TokenMetadata;
use dp_s2s::{CreatePayload, IssuingParamsEncoder};
use drml_common_runtime::impls::FromThisChainMessageVerifier;
use pallet_bridge_messages::EXPECTED_DEFAULT_MESSAGE_LENGTH;

/// Messages delivery proof for Pangoro -> Pangolin messages.
type ToPangolinMessagesDeliveryProof = FromBridgedChainMessagesDeliveryProof<bp_pangolin::Hash>;
/// Messages proof for Pangolin -> Pangoro messages.
type FromPangolinMessagesProof = FromBridgedChainMessagesProof<bp_pangolin::Hash>;

/// Message payload for Pangoro -> Pangolin messages.
pub type ToPangolinMessagePayload = FromThisChainMessagePayload<WithPangolinMessageBridge>;
/// Message payload for Pangolin -> Pangoro messages.
pub type FromPangolinMessagePayload = FromBridgedChainMessagePayload<WithPangolinMessageBridge>;

/// Message verifier for Pangoro -> Pangolin messages.
pub type ToPangolinMessageVerifier =
	FromThisChainMessageVerifier<WithPangolinMessageBridge, Runtime, WithPangolinFeeMarket>;

/// Encoded Pangoro Call as it comes from Pangolin.
pub type FromPangolinEncodedCall = FromBridgedChainEncodedMessageCall<Call>;

/// Call-dispatch based message dispatch for Pangolin -> Pangoro messages.
pub type FromPangolinMessageDispatch =
	FromBridgedChainMessageDispatch<WithPangolinMessageBridge, Runtime, Ring, WithPangolinDispatch>;

/// The s2s issuing pallet index in the pangolin chain runtime
pub const PANGOLIN_S2S_ISSUING_PALLET_INDEX: u8 = 49;

/// Initial value of `PangolinToPangoroConversionRate` parameter.
pub const INITIAL_PANGOLIN_TO_PANGORO_CONVERSION_RATE: FixedU128 =
	FixedU128::from_inner(FixedU128::DIV);

frame_support::parameter_types! {
	/// Pangolin to Pangoro conversion rate. Initially we treat both tokens as equal.
	pub storage PangolinToPangoroConversionRate: FixedU128 = INITIAL_PANGOLIN_TO_PANGORO_CONVERSION_RATE;
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub enum PangolinIssuingParamsEncoder {
	#[codec(index = 0)]
	S2sIssuingPalletRegisterFromRemote(TokenMetadata),
	#[codec(index = 1)]
	S2sIssuingPalletIssueFromRemote(H160, U256, H160),
}

impl IssuingParamsEncoder for PangolinIssuingParamsEncoder {
	fn encode_register_from_remote(meta: TokenMetadata) -> Vec<u8> {
		Self::S2sIssuingPalletRegisterFromRemote(meta).encode()
	}

	fn encode_issue_from_remote(
		token: H160,
		amount: U256,
		recipient: Vec<u8>,
	) -> Result<Vec<u8>, &'static str> {
		if recipient.len() != 20 {
			return Err("Invalid Address Len".into());
		}
		let recipient = H160::from_slice(recipient.as_slice());
		Ok(Self::S2sIssuingPalletIssueFromRemote(token, amount, recipient).encode())
	}
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct ToPangolinOutboundPayload;
impl CreatePayload<bp_pangoro::AccountId, bp_pangoro::AccountPublic, bp_pangoro::Signature>
	for ToPangolinOutboundPayload
{
	type Payload = ToPangolinMessagePayload;

	fn create(
		origin: CallOrigin<bp_pangoro::AccountId, bp_pangoro::AccountPublic, bp_pangoro::Signature>,
		spec_version: u32,
		weight: u64,
		call_params: Vec<u8>,
		dispatch_fee_payment: DispatchFeePayment,
	) -> Result<Self::Payload, &'static str> {
		let call = Self::encode_call(PANGOLIN_S2S_ISSUING_PALLET_INDEX, call_params)?;
		return Ok(ToPangolinMessagePayload {
			spec_version,
			weight,
			origin,
			call,
			dispatch_fee_payment,
		});
	}
}

/// Pangoro -> Pangolin message lane pallet parameters.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum PangoroToPangolinMessagesParameter {
	/// The conversion formula we use is: `PangoroTokens = PangolinTokens * conversion_rate`.
	PangolinToPangoroConversionRate(FixedU128),
}
impl Parameter for PangoroToPangolinMessagesParameter {
	fn save(&self) {
		match *self {
			PangoroToPangolinMessagesParameter::PangolinToPangoroConversionRate(
				ref conversion_rate,
			) => PangolinToPangoroConversionRate::set(conversion_rate),
		}
	}
}

/// Pangoro <-> Pangolin message bridge.
#[derive(Clone, Copy, RuntimeDebug)]
pub struct WithPangolinMessageBridge;
impl MessageBridge for WithPangolinMessageBridge {
	type BridgedChain = Pangolin;
	type ThisChain = Pangoro;

	const BRIDGED_CHAIN_ID: ChainId = PANGOLIN_CHAIN_ID;
	const BRIDGED_MESSAGES_PALLET_NAME: &'static str =
		bp_pangoro::WITH_PANGORO_MESSAGES_PALLET_NAME;
	const RELAYER_FEE_PERCENT: u32 = 10;
	const THIS_CHAIN_ID: ChainId = PANGORO_CHAIN_ID;

	fn bridged_balance_to_this_balance(
		bridged_balance: BalanceOf<Self::BridgedChain>,
	) -> BalanceOf<Self::ThisChain> {
		<BalanceOf<Self::ThisChain>>::try_from(
			PangolinToPangoroConversionRate::get().saturating_mul_int(bridged_balance),
		)
		.unwrap_or(<BalanceOf<Self::ThisChain>>::MAX)
	}
}

/// Pangoro chain from message lane point of view.
#[derive(Clone, Copy, RuntimeDebug)]
pub struct Pangoro;
impl ChainWithMessages for Pangoro {
	type AccountId = bp_pangoro::AccountId;
	type Balance = bp_pangoro::Balance;
	type Hash = bp_pangoro::Hash;
	type Signature = bp_pangoro::Signature;
	type Signer = bp_pangoro::AccountPublic;
	type Weight = Weight;
}
impl ThisChainWithMessages for Pangoro {
	type Call = Call;

	fn is_outbound_lane_enabled(lane: &LaneId) -> bool {
		*lane == [0, 0, 0, 0] || *lane == [0, 0, 0, 1] || *lane == PANGORO_PANGOLIN_LANE
	}

	fn maximal_pending_messages_at_outbound_lane() -> MessageNonce {
		MessageNonce::MAX
	}

	fn estimate_delivery_confirmation_transaction() -> MessageTransaction<Weight> {
		let inbound_data_size = InboundLaneData::<Self::AccountId>::encoded_size_hint(
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

	fn transaction_payment(transaction: MessageTransaction<Weight>) -> Self::Balance {
		// in our testnets, both per-byte fee and weight-to-fee are 1:1
		messages::transaction_payment(
			bp_pangoro::RuntimeBlockWeights::get().get(DispatchClass::Normal).base_extrinsic,
			1,
			FixedU128::zero(),
			|weight| weight as _,
			transaction,
		)
	}
}

/// Pangolin chain from message lane point of view.
#[derive(Clone, Copy, RuntimeDebug)]
pub struct Pangolin;
impl ChainWithMessages for Pangolin {
	type AccountId = bp_pangolin::AccountId;
	type Balance = bp_pangolin::Balance;
	type Hash = bp_pangolin::Hash;
	type Signature = bp_pangolin::Signature;
	type Signer = bp_pangolin::AccountPublic;
	type Weight = Weight;
}
impl BridgedChainWithMessages for Pangolin {
	fn maximal_extrinsic_size() -> u32 {
		bp_pangolin::Pangolin::max_extrinsic_size()
	}

	fn message_weight_limits(_message_payload: &[u8]) -> RangeInclusive<Weight> {
		// we don't want to relay too large messages + keep reserve for future upgrades
		let upper_limit = target::maximal_incoming_message_dispatch_weight(
			bp_pangolin::Pangolin::max_extrinsic_weight(),
		);

		// we're charging for payload bytes in `WithPangolinMessageBridge::transaction_payment`
		// function
		//
		// this bridge may be used to deliver all kind of messages, so we're not making any
		// assumptions about minimal dispatch weight here

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

	fn transaction_payment(transaction: MessageTransaction<Weight>) -> Self::Balance {
		// in our testnets, both per-byte fee and weight-to-fee are 1:1
		messages::transaction_payment(
			bp_pangolin::RuntimeBlockWeights::get().get(DispatchClass::Normal).base_extrinsic,
			1,
			FixedU128::zero(),
			|weight| weight as _,
			transaction,
		)
	}
}
impl TargetHeaderChain<ToPangolinMessagePayload, <Self as ChainWithMessages>::AccountId>
	for Pangolin
{
	type Error = &'static str;
	// The proof is:
	// - hash of the header this proof has been created with;
	// - the storage proof or one or several keys;
	// - id of the lane we prove state of.
	type MessagesDeliveryProof = ToPangolinMessagesDeliveryProof;

	fn verify_message(payload: &ToPangolinMessagePayload) -> Result<(), Self::Error> {
		source::verify_chain_message::<WithPangolinMessageBridge>(payload)
	}

	fn verify_messages_delivery_proof(
		proof: Self::MessagesDeliveryProof,
	) -> Result<(LaneId, InboundLaneData<bp_pangoro::AccountId>), Self::Error> {
		source::verify_messages_delivery_proof::<
			WithPangolinMessageBridge,
			Runtime,
			WithPangolinGrandpa,
		>(proof)
	}
}
impl SourceHeaderChain<<Self as ChainWithMessages>::Balance> for Pangolin {
	type Error = &'static str;
	// The proof is:
	// - hash of the header this proof has been created with;
	// - the storage proof or one or several keys;
	// - id of the lane we prove messages for;
	// - inclusive range of messages nonces that are proved.
	type MessagesProof = FromPangolinMessagesProof;

	fn verify_messages_proof(
		proof: Self::MessagesProof,
		messages_count: u32,
	) -> Result<ProvedMessages<Message<<Self as ChainWithMessages>::Balance>>, Self::Error> {
		target::verify_messages_proof::<WithPangolinMessageBridge, Runtime, WithPangolinGrandpa>(
			proof,
			messages_count,
		)
	}
}
