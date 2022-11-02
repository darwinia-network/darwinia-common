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
use frame_support::{weights::Weight, RuntimeDebug};
use sp_runtime::{FixedPointNumber, FixedU128};
use sp_std::prelude::*;
// --- darwinia-network ---
use crate::*;
use bp_messages::{
	source_chain::{SenderOrigin, TargetHeaderChain},
	target_chain::{ProvedMessages, SourceHeaderChain},
	InboundLaneData, LaneId, Message, MessageNonce, Parameter,
};
use bp_runtime::{Chain, ChainId, PANGOLIN_CHAIN_ID, PANGORO_CHAIN_ID};
use bridge_runtime_common::{
	lanes::PANGORO_PANGOLIN_LANE,
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
	pallets::WITH_PANGORO_MESSAGES_PALLET_NAME,
};
use darwinia_support::evm::{ConcatConverter, DeriveSubstrateAddress};

/// Messages delivery proof for Pangoro -> Pangolin messages.
type ToPangolinMessagesDeliveryProof = FromBridgedChainMessagesDeliveryProof<bp_pangolin::Hash>;
/// Messages proof for Pangolin -> Pangoro messages.
type FromPangolinMessagesProof = FromBridgedChainMessagesProof<bp_pangolin::Hash>;
/// Outbound payload size limit for Pangolin -> Pangoro messages.
pub type ToPangolinMaximalOutboundPayloadSize =
	FromThisChainMaximalOutboundPayloadSize<WithPangolinMessageBridge>;

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
	const BRIDGED_MESSAGES_PALLET_NAME: &'static str = WITH_PANGORO_MESSAGES_PALLET_NAME;
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
		drml_common_runtime::Pangolin::max_extrinsic_size()
	}

	fn verify_dispatch_weight(_message_payload: &[u8], payload_weight: &Weight) -> bool {
		let upper_limit = target::maximal_incoming_message_dispatch_weight(
			drml_common_runtime::Pangolin::max_extrinsic_weight(),
		);

		*payload_weight <= upper_limit
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

#[cfg(test)]
mod tests {
	use super::*;
	use bridge_runtime_common::{
		self, assert_complete_bridge_types,
		integrity::{
			assert_complete_bridge_constants, AssertBridgeMessagesPalletConstants,
			AssertBridgePalletNames, AssertChainConstants, AssertCompleteBridgeConstants,
		},
		pallets::{WITH_PANGOLIN_GRANDPA_PALLET_NAME, WITH_PANGOLIN_MESSAGES_PALLET_NAME},
	};

	#[test]
	fn ensure_bridge_integrity() {
		assert_complete_bridge_types!(
			runtime: Runtime,
			with_bridged_chain_grandpa_instance: WithPangolinGrandpa,
			with_bridged_chain_messages_instance: WithPangolinMessages,
			bridge: WithPangolinMessageBridge,
			this_chain: Pangoro,
			bridged_chain: Pangolin,
		);

		assert_complete_bridge_constants::<
			Runtime,
			WithPangolinGrandpa,
			WithPangolinMessages,
			WithPangolinMessageBridge,
			Pangoro,
		>(AssertCompleteBridgeConstants {
			this_chain_constants: AssertChainConstants {
				block_length: bp_pangoro::RuntimeBlockLength::get(),
				block_weights: bp_pangoro::RuntimeBlockWeights::get(),
			},
			messages_pallet_constants: AssertBridgeMessagesPalletConstants {
				max_unrewarded_relayers_in_bridged_confirmation_tx:
					bp_pangolin::MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX,
				max_unconfirmed_messages_in_bridged_confirmation_tx:
					bp_pangolin::MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX,
				bridged_chain_id: bp_runtime::PANGOLIN_CHAIN_ID,
			},
			pallet_names: AssertBridgePalletNames {
				with_this_chain_messages_pallet_name: WITH_PANGORO_MESSAGES_PALLET_NAME,
				with_bridged_chain_grandpa_pallet_name: WITH_PANGOLIN_GRANDPA_PALLET_NAME,
				with_bridged_chain_messages_pallet_name: WITH_PANGOLIN_MESSAGES_PALLET_NAME,
			},
		});
	}
}
