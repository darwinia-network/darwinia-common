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
use frame_support::{weights::Weight, RuntimeDebug};
use sp_runtime::{FixedPointNumber, FixedU128};
// --- darwinia-network ---
use crate::*;
use bp_messages::{
	source_chain::TargetHeaderChain,
	target_chain::{ProvedMessages, SourceHeaderChain},
	InboundLaneData, LaneId, Message, MessageNonce, Parameter,
};
use bp_polkadot_core::parachains::ParaId;
use bp_runtime::{Chain, ChainId, PANGOLIN_CHAIN_ID, PANGOLIN_PARACHAIN_ALPHA_CHAIN_ID};
use bridge_runtime_common::{
	lanes::PANGOLIN_PANGOLIN_PARACHAIN_ALPHA_LANE,
	messages::{
		source::{
			self, FromBridgedChainMessagesDeliveryProof, FromThisChainMessagePayload,
			FromThisChainMessageVerifier,
		},
		target::{
			self, FromBridgedChainEncodedMessageCall, FromBridgedChainMessageDispatch,
			FromBridgedChainMessagePayload, FromBridgedChainMessagesProof,
		},
		BridgedChainWithMessages, ChainWithMessages, MessageBridge, ThisChainWithMessages,
	},
	pallets::WITH_PANGOLIN_MESSAGES_PALLET_NAME,
};

/// Message delivery proof for Pangolin -> PangolinParachainAlpha messages.
type ToPangolinParachainAlphaMessagesDeliveryProof =
	FromBridgedChainMessagesDeliveryProof<bp_darwinia_core::Hash>;
/// Message proof for PangolinParachainAlpha -> Pangolin  messages.
type FromPangolinParachainAlphaMessagesProof =
	FromBridgedChainMessagesProof<bp_darwinia_core::Hash>;

/// Message payload for Pangolin -> PangolinParachainAlpha messages.
pub type ToPangolinParachainAlphaMessagePayload =
	FromThisChainMessagePayload<WithPangolinParachainAlphaMessageBridge>;
/// Message payload for PangolinParachainAlpha -> Pangolin messages.
pub type FromPangolinParachainAlphaMessagePayload =
	FromBridgedChainMessagePayload<WithPangolinParachainAlphaMessageBridge>;

/// Message verifier for Pangolin -> PangolinParachain messages.
pub type ToPangolinParachainAlphaMessageVerifier = FromThisChainMessageVerifier<
	WithPangolinParachainAlphaMessageBridge,
	Runtime,
	WithPangolinParachainAlphaFeeMarket,
>;

/// Encoded Pangolin Call as it comes from PangolinParachainAlpha
pub type FromPangolinParachainAlphaEncodedCall = FromBridgedChainEncodedMessageCall<Call>;

/// Call-dispatch based message dispatch for PangolinParachainAlpha -> Pangolin messages.
pub type FromPangolinParachainAlphaMessageDispatch = FromBridgedChainMessageDispatch<
	WithPangolinParachainAlphaMessageBridge,
	Runtime,
	Ring,
	WithPangolinParachainAlphaDispatch,
>;

/// Identifier of PangolinParachainAlpha registered in the moonbase relay chain.
pub const PANGOLIN_PARACHAIN_ALPHA_ID: u32 = 2105;

pub const INITIAL_PANGOLIN_PARACHAIN_ALPHA_TO_PANGOLIN_CONVERSION_RATE: FixedU128 =
	FixedU128::from_inner(FixedU128::DIV);

frame_support::parameter_types! {
	/// PangolinParachainAlpha to Pangolin conversion rate. Initially we trate both tokens as equal.
	pub storage PangolinParachainAlphaToPangolinConversionRate: FixedU128 = INITIAL_PANGOLIN_PARACHAIN_ALPHA_TO_PANGOLIN_CONVERSION_RATE;
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum PangolinToPangolinParachainAlphaParameter {
	/// The conversion formula we use is: `PangolinTokens = PangolinParachainAlphaTokens *
	/// conversion_rate`.
	PangolinParachainAlphaToPangolinConversionRate(FixedU128),
}
impl Parameter for PangolinToPangolinParachainAlphaParameter {
	fn save(&self) {
		match *self {
			PangolinToPangolinParachainAlphaParameter::PangolinParachainAlphaToPangolinConversionRate(
				ref conversion_rate,
			) => PangolinParachainAlphaToPangolinConversionRate::set(conversion_rate),
		}
	}
}

/// Pangolin <-> PangolinParachainAlpha message bridge.
#[derive(Clone, Copy, RuntimeDebug)]
pub struct WithPangolinParachainAlphaMessageBridge;
impl MessageBridge for WithPangolinParachainAlphaMessageBridge {
	type BridgedChain = PangolinParachainAlpha;
	type ThisChain = Pangolin;

	const BRIDGED_CHAIN_ID: ChainId = PANGOLIN_PARACHAIN_ALPHA_CHAIN_ID;
	const BRIDGED_MESSAGES_PALLET_NAME: &'static str = WITH_PANGOLIN_MESSAGES_PALLET_NAME;
	const RELAYER_FEE_PERCENT: u32 = 10;
	const THIS_CHAIN_ID: ChainId = PANGOLIN_CHAIN_ID;
}

/// Pangolin chain from message lane point of view.
#[derive(Clone, Copy, RuntimeDebug)]
pub struct Pangolin;
impl ChainWithMessages for Pangolin {
	type AccountId = bp_darwinia_core::AccountId;
	type Balance = bp_darwinia_core::Balance;
	type Hash = bp_darwinia_core::Hash;
	type Signature = bp_darwinia_core::Signature;
	type Signer = bp_darwinia_core::AccountPublic;
	type Weight = Weight;
}
impl ThisChainWithMessages for Pangolin {
	type Call = Call;
	type Origin = Origin;

	fn is_message_accepted(_send_origin: &Self::Origin, lane: &LaneId) -> bool {
		*lane == PANGOLIN_PANGOLIN_PARACHAIN_ALPHA_LANE
	}

	fn maximal_pending_messages_at_outbound_lane() -> MessageNonce {
		MessageNonce::MAX
	}
}

#[derive(Clone, Copy, RuntimeDebug)]
pub struct PangolinParachainAlpha;
impl ChainWithMessages for PangolinParachainAlpha {
	type AccountId = bp_darwinia_core::AccountId;
	type Balance = bp_darwinia_core::Balance;
	type Hash = bp_darwinia_core::Hash;
	type Signature = bp_darwinia_core::Signature;
	type Signer = bp_darwinia_core::AccountPublic;
	type Weight = Weight;
}
impl BridgedChainWithMessages for PangolinParachainAlpha {
	fn maximal_extrinsic_size() -> u32 {
		bp_darwinia_core::DarwiniaLike::max_extrinsic_size()
	}

	fn verify_dispatch_weight(_message_payload: &[u8], payload_weight: &Weight) -> bool {
		let upper_limit = target::maximal_incoming_message_dispatch_weight(
			bp_darwinia_core::DarwiniaLike::max_extrinsic_weight(),
		);

		*payload_weight <= upper_limit
	}
}
impl
	TargetHeaderChain<
		ToPangolinParachainAlphaMessagePayload,
		<Self as ChainWithMessages>::AccountId,
	> for PangolinParachainAlpha
{
	type Error = &'static str;
	type MessagesDeliveryProof = ToPangolinParachainAlphaMessagesDeliveryProof;

	fn verify_message(payload: &ToPangolinParachainAlphaMessagePayload) -> Result<(), Self::Error> {
		source::verify_chain_message::<WithPangolinParachainAlphaMessageBridge>(payload)
	}

	fn verify_messages_delivery_proof(
		proof: Self::MessagesDeliveryProof,
	) -> Result<(LaneId, InboundLaneData<bp_darwinia_core::AccountId>), Self::Error> {
		source::verify_messages_delivery_proof_from_parachain::<
			WithPangolinParachainAlphaMessageBridge,
			bp_darwinia_core::Header,
			Runtime,
			WithMoonbaseRelayParachainInstance,
		>(ParaId(PANGOLIN_PARACHAIN_ALPHA_ID), proof)
	}
}
impl SourceHeaderChain<<Self as ChainWithMessages>::Balance> for PangolinParachainAlpha {
	type Error = &'static str;
	type MessagesProof = FromPangolinParachainAlphaMessagesProof;

	fn verify_messages_proof(
		proof: Self::MessagesProof,
		messages_count: u32,
	) -> Result<ProvedMessages<Message<<Self as ChainWithMessages>::Balance>>, Self::Error> {
		target::verify_messages_proof_from_parachain::<
			WithPangolinParachainAlphaMessageBridge,
			bp_darwinia_core::Header,
			Runtime,
			WithMoonbaseRelayParachainInstance,
		>(ParaId(PANGOLIN_PARACHAIN_ALPHA_ID), proof, messages_count)
	}
}
