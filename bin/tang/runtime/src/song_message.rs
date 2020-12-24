use crate::Runtime;

use bp_message_lane::{
	source_chain::TargetHeaderChain,
	target_chain::{ProvedMessages, SourceHeaderChain},
	InboundLaneData, LaneId, Message, MessageNonce,
};
use bp_runtime::InstanceId;
use bridge_runtime_common::messages::{self, ChainWithMessageLanes, MessageBridge};
use frame_support::{
	weights::{Weight, WeightToFeePolynomial},
	RuntimeDebug,
};
use sp_core::storage::StorageKey;
use sp_std::{convert::TryFrom, ops::RangeInclusive};

/// Bridge-with-Song instance id.
pub const SONG_BRIDGE_INSTANCE: InstanceId = *b"song";

/// Message payload for Tang -> Song messages.
pub type ToSongMessagePayload =
	messages::source::FromThisChainMessagePayload<WithSongMessageBridge>;

/// Message payload for Song -> Tang messages.
pub type FromSongMessagePayload =
	messages::target::FromBridgedChainMessagePayload<WithSongMessageBridge>;

/// Message verifier for Tang -> Song messages.
pub type ToSongMessageVerifier =
	messages::source::FromThisChainMessageVerifier<WithSongMessageBridge>;

/// Call-dispatch based message dispatch for Song -> Tang messages.
pub type FromSongMessageDispatch = messages::target::FromBridgedChainMessageDispatch<
	WithSongMessageBridge,
	crate::Runtime,
	pallet_bridge_call_dispatch::DefaultInstance,
>;

/// Messages proof for Song -> Tang messages.
type FromSongMessagesProof = messages::target::FromBridgedChainMessagesProof<WithSongMessageBridge>;

/// Messages delivery proof for Tang -> Song messages.
type ToSongMessagesDeliveryProof =
	messages::source::FromBridgedChainMessagesDeliveryProof<WithSongMessageBridge>;

/// Tang <-> Song message bridge.
#[derive(RuntimeDebug, Clone, Copy)]
pub struct WithSongMessageBridge;

impl MessageBridge for WithSongMessageBridge {
	const INSTANCE: InstanceId = SONG_BRIDGE_INSTANCE;

	const RELAYER_FEE_PERCENT: u32 = 10;

	type ThisChain = Tang;
	type BridgedChain = Song;

	fn maximal_extrinsic_size_on_target_chain() -> u32 {
		song_node_primitives::MAXIMUM_EXTRINSIC_SIZE
	}

	fn weight_limits_of_message_on_bridged_chain(message_payload: &[u8]) -> RangeInclusive<Weight> {
		// we don't want to relay too large messages + keep reserve for future upgrades
		let upper_limit = song_node_primitives::MAXIMUM_EXTRINSIC_WEIGHT / 2;

		// given Song chain parameters (`TransactionByteFee`, `WeightToFee`, `FeeMultiplierUpdate`),
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

	fn this_weight_to_this_balance(weight: Weight) -> tang_node_primitives::Balance {
		<crate::Runtime as pallet_transaction_payment::Trait>::WeightToFee::calc(&weight)
	}

	fn bridged_weight_to_bridged_balance(weight: Weight) -> song_node_primitives::Balance {
		// we're using the same weights in both chains now
		<crate::Runtime as pallet_transaction_payment::Trait>::WeightToFee::calc(&weight) as _
	}

	fn this_balance_to_bridged_balance(
		this_balance: tang_node_primitives::Balance,
	) -> song_node_primitives::Balance {
		// 1:1 conversion that will probably change in the future
		this_balance as _
	}
}

/// Tang chain from message lane point of view.
#[derive(RuntimeDebug, Clone, Copy)]
pub struct Tang;

impl messages::ChainWithMessageLanes for Tang {
	type Hash = tang_node_primitives::Hash;
	type AccountId = tang_node_primitives::AccountId;
	type Signer = tang_node_primitives::AccountSigner;
	type Signature = tang_node_primitives::Signature;
	type Call = crate::Call;
	type Weight = Weight;
	type Balance = tang_node_primitives::Balance;

	type MessageLaneInstance = pallet_message_lane::DefaultInstance;
}

/// Song chain from message lane point of view.
#[derive(RuntimeDebug, Clone, Copy)]
pub struct Song;

impl messages::ChainWithMessageLanes for Song {
	type Hash = song_node_primitives::Hash;
	type AccountId = song_node_primitives::AccountId;
	type Signer = song_node_primitives::AccountSigner;
	type Signature = song_node_primitives::Signature;
	type Call = (); // unknown to us
	type Weight = Weight;
	type Balance = song_node_primitives::Balance;

	type MessageLaneInstance = pallet_message_lane::DefaultInstance;
}

impl TargetHeaderChain<ToSongMessagePayload, song_node_primitives::AccountId> for Song {
	type Error = &'static str;
	// The proof is:
	// - hash of the header this proof has been created with;
	// - the storage proof or one or several keys;
	// - id of the lane we prove state of.
	type MessagesDeliveryProof = ToSongMessagesDeliveryProof;

	fn verify_message(payload: &ToSongMessagePayload) -> Result<(), Self::Error> {
		messages::source::verify_chain_message::<WithSongMessageBridge>(payload)
	}

	fn verify_messages_delivery_proof(
		proof: Self::MessagesDeliveryProof,
	) -> Result<(LaneId, InboundLaneData<tang_node_primitives::AccountId>), Self::Error> {
		messages::source::verify_messages_delivery_proof::<WithSongMessageBridge, Runtime>(proof)
	}
}

impl SourceHeaderChain<song_node_primitives::Balance> for Song {
	type Error = &'static str;
	// The proof is:
	// - hash of the header this proof has been created with;
	// - the storage proof or one or several keys;
	// - id of the lane we prove messages for;
	// - inclusive range of messages nonces that are proved.
	type MessagesProof = FromSongMessagesProof;

	fn verify_messages_proof(
		proof: Self::MessagesProof,
		max_messages: MessageNonce,
	) -> Result<ProvedMessages<Message<song_node_primitives::Balance>>, Self::Error> {
		messages::target::verify_messages_proof::<WithSongMessageBridge, Runtime>(
			proof,
			max_messages,
		)
	}
}

/// Storage key of the Tang -> Song message in the runtime storage.
pub fn message_key(lane: &LaneId, nonce: MessageNonce) -> StorageKey {
	pallet_message_lane::storage_keys::message_key::<
		Runtime,
		<Tang as ChainWithMessageLanes>::MessageLaneInstance,
	>(lane, nonce)
}

/// Storage key of the Tang -> Song message lane state in the runtime storage.
pub fn outbound_lane_data_key(lane: &LaneId) -> StorageKey {
	pallet_message_lane::storage_keys::outbound_lane_data_key::<
		<Tang as ChainWithMessageLanes>::MessageLaneInstance,
	>(lane)
}

/// Storage key of the Song -> Tang message lane state in the runtime storage.
pub fn inbound_lane_data_key(lane: &LaneId) -> StorageKey {
	pallet_message_lane::storage_keys::inbound_lane_data_key::<
		Runtime,
		<Tang as ChainWithMessageLanes>::MessageLaneInstance,
	>(lane)
}
