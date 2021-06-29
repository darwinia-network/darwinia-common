// --- crates.io ---
use codec::{Decode, Encode};
// --- substrate ---
use bp_messages::{
	source_chain::TargetHeaderChain,
	target_chain::{ProvedMessages, SourceHeaderChain},
	InboundLaneData, LaneId, Message, MessageNonce, Parameter as MessagesParameter,
};
use bp_runtime::{ChainId, MILLAU_CHAIN_ID};
use bridge_runtime_common::messages::{
	self,
	source::{
		self, FromBridgedChainMessagesDeliveryProof, FromThisChainMessagePayload,
		FromThisChainMessageVerifier,
	},
	target::{
		self, FromBridgedChainEncodedMessageCall, FromBridgedChainMessageDispatch,
		FromBridgedChainMessagePayload, FromBridgedChainMessagesProof,
	},
	MessageBridge, MessageTransaction,
};
use frame_support::{
	weights::{DispatchClass, Weight},
	RuntimeDebug,
};
use pallet_bridge_messages::EXPECTED_DEFAULT_MESSAGE_LENGTH;
use sp_runtime::{traits::Zero, FixedPointNumber, FixedU128};
use sp_std::{convert::TryFrom, ops::RangeInclusive};
// --- darwinia ---
use crate::*;
use pangolin_bridge_primitives::PANGOLIN_CHAIN_ID;

/// Message payload for Pangolin -> Millau messages.
pub type ToMillauMessagePayload = FromThisChainMessagePayload<WithMillauMessageBridge>;
/// Message verifier for Pangolin -> Millau messages.
pub type ToMillauMessageVerifier = FromThisChainMessageVerifier<WithMillauMessageBridge>;
/// Message payload for Millau -> Pangolin messages.
pub type FromMillauMessagePayload = FromBridgedChainMessagePayload<WithMillauMessageBridge>;
/// Encoded Pangolin Call as it comes from Millau.
pub type FromMillauEncodedCall = FromBridgedChainEncodedMessageCall<WithMillauMessageBridge>;
/// Messages proof for Millau -> Pangolin messages.
type FromMillauMessagesProof = FromBridgedChainMessagesProof<bp_millau::Hash>;
/// Messages delivery proof for Pangolin -> Millau messages.
type ToMillauMessagesDeliveryProof = FromBridgedChainMessagesDeliveryProof<bp_millau::Hash>;
/// Call-dispatch based message dispatch for Millau -> Pangolin messages.
pub type FromMillauMessageDispatch =
	FromBridgedChainMessageDispatch<WithMillauMessageBridge, Runtime, WithMillauDispatch>;

/// Initial value of `MillauToPangolinConversionRate` parameter.
pub const INITIAL_MILLAU_TO_PANGOLIN_CONVERSION_RATE: FixedU128 =
	FixedU128::from_inner(FixedU128::DIV);

frame_support::parameter_types! {
	/// Millau to Rialto conversion rate. Initially we treat both tokens as equal.
	pub storage MillauToPangolinConversionRate: FixedU128 = INITIAL_MILLAU_TO_PANGOLIN_CONVERSION_RATE;
}

/// Pangolin -> Millau message lane pallet parameters.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug)]
pub enum PangolinToMillauMessagesParameter {
	/// The conversion formula we use is: `PangolinTokens = MillauTokens * conversion_rate`.
	MillauToPangolinConversionRate(FixedU128),
}
impl MessagesParameter for PangolinToMillauMessagesParameter {
	fn save(&self) {
		match *self {
			PangolinToMillauMessagesParameter::MillauToPangolinConversionRate(
				ref conversion_rate,
			) => MillauToPangolinConversionRate::set(conversion_rate),
		}
	}
}

/// Millau <-> Pangolin message bridge.
#[derive(Clone, Copy, RuntimeDebug)]
pub struct WithMillauMessageBridge;
impl MessageBridge for WithMillauMessageBridge {
	const RELAYER_FEE_PERCENT: u32 = 10;

	type ThisChain = Pangolin;
	type BridgedChain = Millau;

	fn bridged_balance_to_this_balance(bridged_balance: bp_millau::Balance) -> Balance {
		Balance::try_from(MillauToPangolinConversionRate::get().saturating_mul_int(bridged_balance))
			.unwrap_or(Balance::MAX)
	}
}

/// Pangolin chain from message lane point of view.
#[derive(Clone, Copy, RuntimeDebug)]
pub struct Pangolin;
impl messages::ChainWithMessages for Pangolin {
	const ID: ChainId = PANGOLIN_CHAIN_ID;

	type Hash = Hash;
	type AccountId = AccountId;
	type Signer = AccountPublic;
	type Signature = Signature;
	type Weight = Weight;
	type Balance = Balance;

	type MessagesInstance = WithMillauMessages;
}
impl messages::ThisChainWithMessages for Pangolin {
	type Call = Call;

	fn is_outbound_lane_enabled(lane: &LaneId) -> bool {
		*lane == [0, 0, 0, 0] || *lane == [0, 0, 0, 1]
	}

	fn maximal_pending_messages_at_outbound_lane() -> MessageNonce {
		MessageNonce::MAX
	}

	fn estimate_delivery_confirmation_transaction() -> MessageTransaction<Weight> {
		let inbound_data_size = InboundLaneData::<AccountId>::encoded_size_hint(
			pangolin_bridge_primitives::MAXIMAL_ENCODED_ACCOUNT_ID_SIZE,
			1,
		)
		.unwrap_or(u32::MAX);

		MessageTransaction {
			dispatch_weight:
				pangolin_bridge_primitives::MAX_SINGLE_MESSAGE_DELIVERY_CONFIRMATION_TX_WEIGHT,
			size: inbound_data_size
				.saturating_add(bp_millau::EXTRA_STORAGE_PROOF_SIZE)
				.saturating_add(pangolin_bridge_primitives::TX_EXTRA_BYTES),
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

/// Millau chain from message lane point of view.
#[derive(Clone, Copy, RuntimeDebug)]
pub struct Millau;
impl messages::ChainWithMessages for Millau {
	const ID: ChainId = MILLAU_CHAIN_ID;

	type Hash = bp_millau::Hash;
	type AccountId = bp_millau::AccountId;
	type Signer = bp_millau::AccountSigner;
	type Signature = bp_millau::Signature;
	type Weight = Weight;
	type Balance = bp_millau::Balance;

	type MessagesInstance = WithMillauMessages;
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
			.saturating_sub(EXPECTED_DEFAULT_MESSAGE_LENGTH.into());

		MessageTransaction {
			dispatch_weight: extra_bytes_in_payload
				.saturating_mul(bp_millau::ADDITIONAL_MESSAGE_BYTE_DELIVERY_WEIGHT)
				.saturating_add(bp_millau::DEFAULT_MESSAGE_DELIVERY_TX_WEIGHT)
				.saturating_add(message_dispatch_weight),
			size: message_payload_len
				.saturating_add(pangolin_bridge_primitives::EXTRA_STORAGE_PROOF_SIZE)
				.saturating_add(bp_millau::TX_EXTRA_BYTES),
		}
	}

	fn transaction_payment(transaction: MessageTransaction<Weight>) -> bp_millau::Balance {
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
	// - the storage proof or one or several keys;
	// - id of the lane we prove state of.
	type MessagesDeliveryProof = ToMillauMessagesDeliveryProof;

	fn verify_message(payload: &ToMillauMessagePayload) -> Result<(), Self::Error> {
		source::verify_chain_message::<WithMillauMessageBridge>(payload)
	}

	fn verify_messages_delivery_proof(
		proof: Self::MessagesDeliveryProof,
	) -> Result<(LaneId, InboundLaneData<AccountId>), Self::Error> {
		source::verify_messages_delivery_proof::<WithMillauMessageBridge, Runtime, WithMillauGrandpa>(
			proof,
		)
	}
}
impl SourceHeaderChain<bp_millau::Balance> for Millau {
	type Error = &'static str;
	// The proof is:
	// - hash of the header this proof has been created with;
	// - the storage proof or one or several keys;
	// - id of the lane we prove messages for;
	// - inclusive range of messages nonces that are proved.
	type MessagesProof = FromMillauMessagesProof;

	fn verify_messages_proof(
		proof: Self::MessagesProof,
		messages_count: u32,
	) -> Result<ProvedMessages<Message<bp_millau::Balance>>, Self::Error> {
		target::verify_messages_proof::<WithMillauMessageBridge, Runtime, WithMillauGrandpa>(
			proof,
			messages_count,
		)
	}
}
