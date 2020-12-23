use crate::Runtime;

use bp_message_lane::{
	source_chain::TargetHeaderChain,
	target_chain::{ProvedMessages, SourceHeaderChain},
	InboundLaneData, LaneId, Message, MessageNonce,
};
use bp_runtime::{InstanceId, MILLAU_BRIDGE_INSTANCE};
use bridge_runtime_common::messages::{self, ChainWithMessageLanes, MessageBridge};
use frame_support::{
	weights::{Weight, WeightToFeePolynomial},
	RuntimeDebug,
};
use sp_core::storage::StorageKey;
use sp_std::{convert::TryFrom, ops::RangeInclusive};

/// Message payload for Song -> Tang messages.
pub type ToTangMessagePayload =
	messages::source::FromThisChainMessagePayload<WithTangMessageBridge>;

/// Message verifier for Song -> Tang messages.
pub type ToTangMessageVerifier =
	messages::source::FromThisChainMessageVerifier<WithTangMessageBridge>;

/// Message payload for Tang -> Song messages.
pub type FromTangMessagePayload =
	messages::target::FromBridgedChainMessagePayload<WithTangMessageBridge>;

/// Call-dispatch based message dispatch for Tang -> Song messages.
pub type FromTangMessageDispatch = messages::target::FromBridgedChainMessageDispatch<
	WithTangMessageBridge,
	crate::Runtime,
	pallet_bridge_call_dispatch::DefaultInstance,
>;

/// Messages proof for Tang -> Song messages.
type FromTangMessagesProof = messages::target::FromBridgedChainMessagesProof<WithTangMessageBridge>;

/// Messages delivery proof for Song -> Tang messages.
type ToTangMessagesDeliveryProof =
	messages::source::FromBridgedChainMessagesDeliveryProof<WithTangMessageBridge>;

/// Bridge-with-Song instance id.
pub const Song_BRIDGE_INSTANCE: InstanceId = *b"song";

/// Tang <-> Song message bridge.
#[derive(RuntimeDebug, Clone, Copy)]
pub struct WithTangMessageBridge;

impl MessageBridge for WithTangMessageBridge {
	const INSTANCE: InstanceId = MILLAU_BRIDGE_INSTANCE;

	const RELAYER_FEE_PERCENT: u32 = 10;

	type ThisChain = Song;
	type BridgedChain = Tang;

	fn maximal_extrinsic_size_on_target_chain() -> u32 {
		tang_node_primitives::MAXIMUM_EXTRINSIC_SIZE
	}

	fn weight_limits_of_message_on_bridged_chain(message_payload: &[u8]) -> RangeInclusive<Weight> {
		// we don't want to relay too large messages + keep reserve for future upgrades
		let upper_limit = tang_node_primitives::MAXIMUM_EXTRINSIC_WEIGHT / 2;

		// given Millau chain parameters (`TransactionByteFee`, `WeightToFee`, `FeeMultiplierUpdate`),
		// the minimal weight of the message may be computed as message.length()
		let lower_limit = Weight::try_from(message_payload.len()).unwrap_or(Weight::MAX);

		lower_limit..=upper_limit
	}

	fn weight_of_delivery_transaction() -> Weight {
		0 // TODO: https://github.com/paritytech/parity-bridges-common/issues/391
	}

	fn weight_of_delivery_confirmation_transaction_on_this_chain() -> Weight {
		0 // TODO: https://github.com/paritytech/parity-bridges-common/issues/391
	}

	fn weight_of_reward_confirmation_transaction_on_target_chain() -> Weight {
		0 // TODO: https://github.com/paritytech/parity-bridges-common/issues/391
	}

	fn this_weight_to_this_balance(weight: Weight) -> song_node_primitives::Balance {
		<crate::Runtime as pallet_transaction_payment::Trait>::WeightToFee::calc(&weight)
	}

	fn bridged_weight_to_bridged_balance(weight: Weight) -> tang_node_primitives::Balance {
		// we're using the same weights in both chains now
		<crate::Runtime as pallet_transaction_payment::Trait>::WeightToFee::calc(&weight) as _
	}

	fn this_balance_to_bridged_balance(
		this_balance: song_node_primitives::Balance,
	) -> tang_node_primitives::Balance {
		// 1:1 conversion that will probably change in the future
		this_balance as _
	}
}

/// Song chain from message lane point of view.
#[derive(RuntimeDebug, Clone, Copy)]
pub struct Song;

impl messages::ChainWithMessageLanes for Song {
	type Hash = song_node_primitives::Hash;
	type AccountId = song_node_primitives::AccountId;
	type Signer = song_node_primitives::AccountSigner;
	type Signature = song_node_primitives::Signature;
	type Call = crate::Call;
	type Weight = Weight;
	type Balance = song_node_primitives::Balance;

	type MessageLaneInstance = pallet_message_lane::DefaultInstance;
}

/// Tang chain from message lane point of view.
#[derive(RuntimeDebug, Clone, Copy)]
pub struct Tang;

impl messages::ChainWithMessageLanes for Tang {
	type Hash = tang_node_primitives::Hash;
	type AccountId = tang_node_primitives::AccountId;
	type Signer = tang_node_primitives::AccountSigner;
	type Signature = tang_node_primitives::Signature;
	type Call = (); // unknown to us
	type Weight = Weight;
	type Balance = tang_node_primitives::Balance;

	type MessageLaneInstance = pallet_message_lane::DefaultInstance;
}

impl TargetHeaderChain<ToTangMessagePayload, tang_node_primitives::AccountId> for Tang {
	type Error = &'static str;
	// The proof is:
	// - hash of the header this proof has been created with;
	// - the storage proof of one or several keys;
	// - id of the lane we prove state of.
	type MessagesDeliveryProof = ToTangMessagesDeliveryProof;

	fn verify_message(payload: &ToTangMessagePayload) -> Result<(), Self::Error> {
		messages::source::verify_chain_message::<WithTangMessageBridge>(payload)
	}

	fn verify_messages_delivery_proof(
		proof: Self::MessagesDeliveryProof,
	) -> Result<(LaneId, InboundLaneData<song_node_primitives::AccountId>), Self::Error> {
		messages::source::verify_messages_delivery_proof::<WithTangMessageBridge, Runtime>(proof)
	}
}

impl SourceHeaderChain<tang_node_primitives::Balance> for Tang {
	type Error = &'static str;
	// The proof is:
	// - hash of the header this proof has been created with;
	// - the storage proof of one or several keys;
	// - id of the lane we prove messages for;
	// - inclusive range of messages nonces that are proved.
	type MessagesProof = FromTangMessagesProof;

	fn verify_messages_proof(
		proof: Self::MessagesProof,
		max_messages: MessageNonce,
	) -> Result<ProvedMessages<Message<tang_node_primitives::Balance>>, Self::Error> {
		messages::target::verify_messages_proof::<WithTangMessageBridge, Runtime>(
			proof,
			max_messages,
		)
	}
}
