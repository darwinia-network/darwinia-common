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
use sp_std::{ops::RangeInclusive, prelude::*};
// --- darwinia-network ---
use crate::*;
use bp_messages::{source_chain::*, target_chain::*, *};
use bp_runtime::{ChainId, *};
use bridge_runtime_common::{
	lanes::*,
	messages::{
		self,
		source::{self, *},
		target::{self, *},
		*,
	},
};
use darwinia_support::evm::{ConcatConverter, DeriveSubstrateAddress};
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
	type Origin = Origin;

	fn is_message_accepted(_send_origin: &Self::Origin, lane: &LaneId) -> bool {
		*lane == PANGORO_PANGOLIN_LANE
	}

	fn maximal_pending_messages_at_outbound_lane() -> MessageNonce {
		MessageNonce::MAX
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

impl SenderOrigin<crate::AccountId> for crate::Origin {
	fn linked_account(&self) -> Option<crate::AccountId> {
		match self.caller {
			crate::OriginCaller::system(frame_system::RawOrigin::Signed(ref submitter)) =>
				Some(submitter.clone()),
			crate::OriginCaller::system(frame_system::RawOrigin::Root) => {
				// 0x726f6f7400000000000000000000000000000000, b"root"
				Some(ConcatConverter::<_>::derive_substrate_address(&H160([
					0x72, 0x6f, 0x6f, 0x74, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
				])))
			},
			_ => None,
		}
	}
}
