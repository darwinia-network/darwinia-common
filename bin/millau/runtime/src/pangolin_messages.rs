
use crate::Runtime;

use bp_messages::{
	source_chain::TargetHeaderChain,
	target_chain::{ProvedMessages, SourceHeaderChain},
	InboundLaneData, LaneId, Message, MessageNonce, Parameter as MessagesParameter,
};
use bp_runtime::{InstanceId, PANGOLIN_BRIDGE_INSTANCE};
use bridge_runtime_common::messages::{self, MessageBridge, MessageTransaction};
use codec::{Decode, Encode};
use frame_support::{
	parameter_types,
	weights::{DispatchClass, Weight},
	RuntimeDebug,
};
use sp_runtime::{FixedPointNumber, FixedU128};
use sp_std::{convert::TryFrom, ops::RangeInclusive};


/// Initial value of `PangolinToMillauConversionRate` parameter.
pub const INITIAL_PANGOLIN_TO_MILLAU_CONVERSION_RATE: FixedU128 = FixedU128::from_inner(FixedU128::DIV);

parameter_types! {
	/// Rialto to Millau conversion rate. Initially we treat both tokens as equal.
	pub storage PangolinToMillauConversionRate: FixedU128 = INITIAL_PANGOLIN_TO_MILLAU_CONVERSION_RATE;
}


/// Message payload for Millau -> Pangolin messages.
pub type ToPangolinMessagePayload = messages::source::FromThisChainMessagePayload<WithPangolinMessageBridge>;

/// Message verifier for Millau -> Pangolin messages.
pub type ToPangolinMessageVerifier = messages::source::FromThisChainMessageVerifier<WithPangolinMessageBridge>;

/// Message payload for Pangolin -> Millau messages.
pub type FromPangolinMessagePayload = messages::target::FromBridgedChainMessagePayload<WithPangolinMessageBridge>;

/// Encoded Millau Call as it comes from Pangolin.
pub type FromPangolinEncodedCall = messages::target::FromBridgedChainEncodedMessageCall<WithPangolinMessageBridge>;

/// Messages proof for Pangolin -> Millau messages.
type FromPangolinMessagesProof = messages::target::FromBridgedChainMessagesProof<drml_primitives::Hash>;

/// Messages delivery proof for Millau -> Pangolin messages.
type ToPangolinMessagesDeliveryProof = messages::source::FromBridgedChainMessagesDeliveryProof<drml_primitives::Hash>;


/// Call-dispatch based message dispatch for Pangolin -> Millau messages.
pub type FromPangolinMessageDispatch = messages::target::FromBridgedChainMessageDispatch<
	WithPangolinMessageBridge,
	crate::Runtime,
	crate::WithPangolinDispatchInstance,
>;

/// Millau <-> Pangolin message bridge.
#[derive(RuntimeDebug, Clone, Copy)]
pub struct WithPangolinMessageBridge;

impl MessageBridge for WithPangolinMessageBridge {
	const INSTANCE: InstanceId = PANGOLIN_BRIDGE_INSTANCE;

	const RELAYER_FEE_PERCENT: u32 = 10;

	type ThisChain = Millau;
	type BridgedChain = PangolinChainWithMessagesInMillau;

	fn bridged_balance_to_this_balance(bridged_balance: drml_primitives::Balance) -> bp_millau::Balance {
		bp_millau::Balance::try_from(PangolinToMillauConversionRate::get().saturating_mul_int(bridged_balance))
			.unwrap_or(bp_millau::Balance::MAX)
	}
}

/// Millau chain from message lane point of view.
#[derive(RuntimeDebug, Clone, Copy)]
pub struct Millau;

impl messages::ChainWithMessages for Millau {
	type Hash = bp_millau::Hash;
	type AccountId = bp_millau::AccountId;
	type Signer = bp_millau::AccountSigner;
	type Signature = bp_millau::Signature;
	type Weight = Weight;
	type Balance = bp_millau::Balance;

	type MessagesInstance = crate::WithPangolinMessagesInstance;
}

impl messages::ThisChainWithMessages for Millau {
	type Call = crate::Call;

	fn is_outbound_lane_enabled(lane: &LaneId) -> bool {
		*lane == [0, 0, 0, 0] || *lane == [0, 0, 0, 1]
	}

	fn maximal_pending_messages_at_outbound_lane() -> MessageNonce {
		MessageNonce::MAX
	}

	fn estimate_delivery_confirmation_transaction() -> MessageTransaction<Weight> {
		let inbound_data_size =
			InboundLaneData::<bp_millau::AccountId>::encoded_size_hint(bp_millau::MAXIMAL_ENCODED_ACCOUNT_ID_SIZE, 1)
				.unwrap_or(u32::MAX);

		MessageTransaction {
			dispatch_weight: bp_millau::MAX_SINGLE_MESSAGE_DELIVERY_CONFIRMATION_TX_WEIGHT,
			size: inbound_data_size
				.saturating_add(drml_primitives::EXTRA_STORAGE_PROOF_SIZE)
				.saturating_add(bp_millau::TX_EXTRA_BYTES),
		}
	}

	fn transaction_payment(transaction: MessageTransaction<Weight>) -> bp_millau::Balance {
		// in our testnets, both per-byte fee and weight-to-fee are 1:1
		messages::transaction_payment(
			bp_millau::BlockWeights::get().get(DispatchClass::Normal).base_extrinsic,
			1,
			FixedU128::zero(),
			|weight| weight as _,
			transaction,
		)
	}
}

/// Rialto chain from message lane point of view.
#[derive(RuntimeDebug, Clone, Copy)]
pub struct PangolinChainWithMessagesInMillau;

impl messages::ChainWithMessages for PangolinChainWithMessagesInMillau {
	type Hash = drml_primitives::Hash;
	type AccountId = drml_primitives::AccountId;
	type Signer = drml_primitives::AccountSigner;
	type Signature = drml_primitives::Signature;
	type Weight = Weight;
	type Balance = drml_primitives::Balance;

	// todo: check it use WithPangolinMessagesInstance or DefaultInstance
	type MessagesInstance = crate::WithPangolinMessagesInstance;
}

impl messages::BridgedChainWithMessages for PangolinChainWithMessagesInMillau {
	fn maximal_extrinsic_size() -> u32 {
		drml_primitives::max_extrinsic_size()
	}

	fn message_weight_limits(_message_payload: &[u8]) -> RangeInclusive<Weight> {
		// we don't want to relay too large messages + keep reserve for future upgrades
		let upper_limit = messages::target::maximal_incoming_message_dispatch_weight(drml_primitives::max_extrinsic_weight());

		// we're charging for payload bytes in `WithRialtoMessageBridge::transaction_payment` function
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
			.saturating_sub(pallet_bridge_messages::EXPECTED_DEFAULT_MESSAGE_LENGTH.into());

		MessageTransaction {
			dispatch_weight: extra_bytes_in_payload
				.saturating_mul(drml_primitives::ADDITIONAL_MESSAGE_BYTE_DELIVERY_WEIGHT)
				.saturating_add(drml_primitives::DEFAULT_MESSAGE_DELIVERY_TX_WEIGHT)
				.saturating_add(message_dispatch_weight),
			size: message_payload_len
				.saturating_add(bp_millau::EXTRA_STORAGE_PROOF_SIZE)
				.saturating_add(drml_primitives::TX_EXTRA_BYTES),
		}
	}

	fn transaction_payment(transaction: MessageTransaction<Weight>) -> drml_primitives::Balance {
		// in our testnets, both per-byte fee and weight-to-fee are 1:1
		messages::transaction_payment(
			drml_primitives::RuntimeBlockWeights::get().get(DispatchClass::Normal).base_extrinsic,
			1,
			FixedU128::zero(),
			|weight| weight as _,
			transaction,
		)
	}
}

impl TargetHeaderChain<ToPangolinMessagePayload, drml_primitives::AccountId> for PangolinChainWithMessagesInMillau {
	type Error = &'static str;
	// The proof is:
	// - hash of the header this proof has been created with;
	// - the storage proof or one or several keys;
	// - id of the lane we prove state of.
	type MessagesDeliveryProof = ToPangolinMessagesDeliveryProof;

	fn verify_message(payload: &ToPangolinMessagePayload) -> Result<(), Self::Error> {
		messages::source::verify_chain_message::<WithPangolinMessageBridge>(payload)
	}

	fn verify_messages_delivery_proof(
		proof: Self::MessagesDeliveryProof,
	) -> Result<(LaneId, InboundLaneData<bp_millau::AccountId>), Self::Error> {
		messages::source::verify_messages_delivery_proof::<WithPangolinMessageBridge, Runtime, crate::WithPangolinGrandpaInstance>(proof)
	}
}

impl SourceHeaderChain<drml_primitives::Balance> for PangolinChainWithMessagesInMillau {
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
	) -> Result<ProvedMessages<Message<drml_primitives::Balance>>, Self::Error> {
		messages::target::verify_messages_proof::<WithPangolinMessageBridge, Runtime, crate::WithPangolinGrandpaInstance>(proof, messages_count)
	}
}

/// Millau -> Pangolin message lane pallet parameters.
#[derive(RuntimeDebug, Clone, Encode, Decode, PartialEq, Eq)]
pub enum MillauToPangolinMessagesParameter {
	/// The conversion formula we use is: `MillauTokens = RialtoTokens * conversion_rate`.
	PangolinToMillauConversionRate(FixedU128),
}

impl MessagesParameter for MillauToPangolinMessagesParameter {
	fn save(&self) {
		match *self {
			MillauToPangolinMessagesParameter::PangolinToMillauConversionRate(ref conversion_rate) => {
				PangolinToMillauConversionRate::set(conversion_rate)
			}
		}
	}
}


