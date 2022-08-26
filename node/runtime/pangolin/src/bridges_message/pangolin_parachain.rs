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
use sp_std::ops::RangeInclusive;
// --- darwinia-network ---
use crate::*;
use bp_messages::{source_chain::*, target_chain::*, *};
use bp_rococo::parachains::ParaId;
use bp_runtime::{ChainId, *};
use bridge_runtime_common::{
	lanes::*,
	messages::{
		self,
		source::{self, *},
		target::{self, *},
		BalanceOf, *,
	},
};
use drml_common_runtime::impls::FromThisChainMessageVerifier;
use pallet_bridge_messages::EXPECTED_DEFAULT_MESSAGE_LENGTH;

/// Message delivery proof for Pangolin -> PangolinParachain messages.
type ToPangolinParachainMessagesDeliveryProof =
	FromBridgedChainMessagesDeliveryProof<bp_pangolin_parachain::Hash>;
/// Message proof for PangolinParachain -> Pangolin  messages.
type FromPangolinParachainMessagesProof =
	FromBridgedChainMessagesProof<bp_pangolin_parachain::Hash>;

/// Message payload for Pangolin -> PangolinParachain messages.
pub type ToPangolinParachainMessagePayload =
	FromThisChainMessagePayload<WithPangolinParachainMessageBridge>;
/// Message payload for PangolinParachain -> Pangolin messages.
pub type FromPangolinParachainMessagePayload =
	FromBridgedChainMessagePayload<WithPangolinParachainMessageBridge>;

/// Message verifier for Pangolin -> PangolinParachain messages.
pub type ToPangolinParachainMessageVerifier = FromThisChainMessageVerifier<
	WithPangolinParachainMessageBridge,
	Runtime,
	WithPangolinParachainFeeMarket,
>;

/// Encoded Pangolin Call as it comes from PangolinParachain
pub type FromPangolinParachainEncodedCall = FromBridgedChainEncodedMessageCall<Call>;

/// Call-dispatch based message dispatch for PangolinParachain -> Pangolin messages.
pub type FromPangolinParachainMessageDispatch = FromBridgedChainMessageDispatch<
	WithPangolinParachainMessageBridge,
	Runtime,
	Ring,
	WithPangolinParachainDispatch,
>;

/// Identifier of PangolinParachain registered in the rococo relay chain.
pub const PANGOLIN_PARACHAIN_ID: u32 = 2105;

pub const INITIAL_PANGOLIN_PARACHAIN_TO_PANGOLIN_CONVERSION_RATE: FixedU128 =
	FixedU128::from_inner(FixedU128::DIV);

frame_support::parameter_types! {
	/// PangolinParachain to Pangolin conversion rate. Initially we trate both tokens as equal.
	pub storage PangolinParachainToPangolinConversionRate: FixedU128 = INITIAL_PANGOLIN_PARACHAIN_TO_PANGOLIN_CONVERSION_RATE;
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum PangolinToPangolinParachainParameter {
	/// The conversion formula we use is: `PangolinTokens = PangolinParachainTokens *
	/// conversion_rate`.
	PangolinParachainToPangolinConversionRate(FixedU128),
}
impl Parameter for PangolinToPangolinParachainParameter {
	fn save(&self) {
		match *self {
			PangolinToPangolinParachainParameter::PangolinParachainToPangolinConversionRate(
				ref conversion_rate,
			) => PangolinParachainToPangolinConversionRate::set(conversion_rate),
		}
	}
}

/// Pangolin <-> PangolinParachain message bridge.
#[derive(Clone, Copy, RuntimeDebug)]
pub struct WithPangolinParachainMessageBridge;
impl MessageBridge for WithPangolinParachainMessageBridge {
	type BridgedChain = PangolinParachain;
	type ThisChain = Pangolin;

	const BRIDGED_CHAIN_ID: ChainId = PANGOLIN_PARACHAIN_CHAIN_ID;
	const BRIDGED_MESSAGES_PALLET_NAME: &'static str =
		bp_pangolin::WITH_PANGOLIN_MESSAGES_PALLET_NAME;
	const RELAYER_FEE_PERCENT: u32 = 10;
	const THIS_CHAIN_ID: ChainId = PANGOLIN_CHAIN_ID;

	fn bridged_balance_to_this_balance(
		bridged_balance: BalanceOf<Self::BridgedChain>,
	) -> BalanceOf<Self::ThisChain> {
		PangolinParachainToPangolinConversionRate::get().saturating_mul_int(bridged_balance)
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
impl ThisChainWithMessages for Pangolin {
	type Call = Call;

	fn is_outbound_lane_enabled(lane: &LaneId) -> bool {
		*lane == PANGOLIN_PANGOLIN_PARACHAIN_LANE
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
			RuntimeBlockWeights::get().get(DispatchClass::Normal).base_extrinsic,
			1,
			FixedU128::zero(),
			|weight| weight as _,
			transaction,
		)
	}
}

#[derive(Clone, Copy, RuntimeDebug)]
pub struct PangolinParachain;
impl ChainWithMessages for PangolinParachain {
	type AccountId = bp_pangolin_parachain::AccountId;
	type Balance = bp_pangolin_parachain::Balance;
	type Hash = bp_pangolin_parachain::Hash;
	type Signature = bp_pangolin_parachain::Signature;
	type Signer = bp_pangolin_parachain::AccountPublic;
	type Weight = Weight;
}
impl BridgedChainWithMessages for PangolinParachain {
	fn maximal_extrinsic_size() -> u32 {
		bp_pangolin_parachain::PangolinParachain::max_extrinsic_size()
	}

	fn message_weight_limits(_message_payload: &[u8]) -> RangeInclusive<Self::Weight> {
		let upper_limit = target::maximal_incoming_message_dispatch_weight(
			bp_pangolin_parachain::PangolinParachain::max_extrinsic_weight(),
		);
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
			bp_pangolin_parachain::RuntimeBlockWeights::get()
				.get(DispatchClass::Normal)
				.base_extrinsic,
			1,
			FixedU128::zero(),
			|weight| weight as _,
			transaction,
		)
	}
}
impl TargetHeaderChain<ToPangolinParachainMessagePayload, <Self as ChainWithMessages>::AccountId>
	for PangolinParachain
{
	type Error = &'static str;
	type MessagesDeliveryProof = ToPangolinParachainMessagesDeliveryProof;

	fn verify_message(payload: &ToPangolinParachainMessagePayload) -> Result<(), Self::Error> {
		source::verify_chain_message::<WithPangolinParachainMessageBridge>(payload)
	}

	fn verify_messages_delivery_proof(
		proof: Self::MessagesDeliveryProof,
	) -> Result<(LaneId, InboundLaneData<bp_pangolin::AccountId>), Self::Error> {
		source::verify_messages_delivery_proof_from_parachain::<
			WithPangolinParachainMessageBridge,
			bp_pangolin_parachain::Header,
			Runtime,
			WithRococoParachainsInstance,
		>(ParaId(PANGOLIN_PARACHAIN_ID), proof)
	}
}
impl SourceHeaderChain<<Self as ChainWithMessages>::Balance> for PangolinParachain {
	type Error = &'static str;
	type MessagesProof = FromPangolinParachainMessagesProof;

	fn verify_messages_proof(
		proof: Self::MessagesProof,
		messages_count: u32,
	) -> Result<ProvedMessages<Message<<Self as ChainWithMessages>::Balance>>, Self::Error> {
		target::verify_messages_proof_from_parachain::<
			WithPangolinParachainMessageBridge,
			bp_pangolin_parachain::Header,
			Runtime,
			WithRococoParachainsInstance,
		>(ParaId(PANGOLIN_PARACHAIN_ID), proof, messages_count)
	}
}
