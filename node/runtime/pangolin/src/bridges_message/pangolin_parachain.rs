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
use bp_runtime::{Chain, ChainId, PANGOLIN_CHAIN_ID, PANGOLIN_PARACHAIN_CHAIN_ID};
use bridge_runtime_common::{
	lanes::PANGOLIN_PANGOLIN_PARACHAIN_LANE,
	messages::{
		source::{
			self, FromBridgedChainMessagesDeliveryProof, FromThisChainMaximalOutboundPayloadSize,
			FromThisChainMessagePayload, FromThisChainMessageVerifier,
		},
		target::{
			self, FromBridgedChainEncodedMessageCall, FromBridgedChainMessageDispatch,
			FromBridgedChainMessagePayload, FromBridgedChainMessagesProof,
		},
		BridgedChainWithMessages, ChainWithMessages, MessageBridge, ThisChainWithMessages,
	},
	pallets::WITH_PANGOLIN_MESSAGES_PALLET_NAME,
};
use drml_common_runtime::{bp_pangolin, bp_pangolin_parachain};

/// Message delivery proof for Pangolin -> PangolinParachain messages.
type ToPangolinParachainMessagesDeliveryProof =
	FromBridgedChainMessagesDeliveryProof<bp_pangolin_parachain::Hash>;
/// Message proof for PangolinParachain -> Pangolin  messages.
type FromPangolinParachainMessagesProof =
	FromBridgedChainMessagesProof<bp_pangolin_parachain::Hash>;
/// Outbound payload size limit for Pangolin -> PangolinParachain messages.
pub type ToPangolinParachainMaximalOutboundPayloadSize =
	FromThisChainMaximalOutboundPayloadSize<WithPangolinParachainMessageBridge>;

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
	const BRIDGED_MESSAGES_PALLET_NAME: &'static str = WITH_PANGOLIN_MESSAGES_PALLET_NAME;
	const RELAYER_FEE_PERCENT: u32 = 10;
	const THIS_CHAIN_ID: ChainId = PANGOLIN_CHAIN_ID;
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
	type Origin = Origin;

	fn is_message_accepted(_send_origin: &Self::Origin, lane: &LaneId) -> bool {
		*lane == PANGOLIN_PANGOLIN_PARACHAIN_LANE
	}

	fn maximal_pending_messages_at_outbound_lane() -> MessageNonce {
		MessageNonce::MAX
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
		drml_common_runtime::PangolinParachain::max_extrinsic_size()
	}

	fn verify_dispatch_weight(_message_payload: &[u8], payload_weight: &Weight) -> bool {
		let upper_limit = target::maximal_incoming_message_dispatch_weight(
			drml_common_runtime::PangolinParachain::max_extrinsic_weight(),
		);

		*payload_weight <= upper_limit
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
			WithRococoParachainInstance,
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
			WithRococoParachainInstance,
		>(ParaId(PANGOLIN_PARACHAIN_ID), proof, messages_count)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bridge_runtime_common::{
		assert_complete_bridge_types,
		integrity::{
			assert_complete_bridge_constants, AssertBridgeMessagesPalletConstants,
			AssertBridgePalletNames, AssertChainConstants, AssertCompleteBridgeConstants,
		},
		pallets::{
			WITH_PANGOLIN_MESSAGES_PALLET_NAME, WITH_PANGOLIN_PARACHAIN_GRANDPA_PALLET_NAME,
			WITH_PANGOLIN_PARACHAIN_MESSAGES_PALLET_NAME,
		},
	};

	#[test]
	fn ensure_bridge_integrity() {
		assert_complete_bridge_types!(
			runtime: Runtime,
			with_bridged_chain_grandpa_instance: WithRococoGrandpa,
			with_bridged_chain_messages_instance: WithPangolinParachainMessages,
			bridge: WithPangolinParachainMessageBridge,
			this_chain: Pangolin,
			bridged_chain: Rococo,
		);

		assert_complete_bridge_constants::<
			Runtime,
			WithRococoGrandpa,
			WithPangolinParachainMessages,
			WithPangolinParachainMessageBridge,
			Pangolin,
		>(AssertCompleteBridgeConstants {
			this_chain_constants: AssertChainConstants {
				block_length: bp_pangolin::RuntimeBlockLength::get(),
				block_weights: bp_pangolin::RuntimeBlockWeights::get(),
			},
			messages_pallet_constants: AssertBridgeMessagesPalletConstants {
				max_unrewarded_relayers_in_bridged_confirmation_tx:
					bp_pangolin_parachain::MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX,
				max_unconfirmed_messages_in_bridged_confirmation_tx:
					bp_pangolin_parachain::MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX,
				bridged_chain_id: bp_runtime::PANGOLIN_PARACHAIN_CHAIN_ID,
			},
			pallet_names: AssertBridgePalletNames {
				with_this_chain_messages_pallet_name: WITH_PANGOLIN_MESSAGES_PALLET_NAME,
				with_bridged_chain_grandpa_pallet_name: WITH_PANGOLIN_PARACHAIN_GRANDPA_PALLET_NAME,
				with_bridged_chain_messages_pallet_name:
					WITH_PANGOLIN_PARACHAIN_MESSAGES_PALLET_NAME,
			},
		});
	}
}
