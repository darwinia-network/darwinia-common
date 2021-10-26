// --- crates.io ---
use codec::{Decode, Encode};
// --- paritytech ---
use bp_messages::{
	source_chain::TargetHeaderChain,
	target_chain::{ProvedMessages, SourceHeaderChain},
	InboundLaneData, LaneId, Message, MessageNonce, Parameter as MessagesParameter,
};
use bp_runtime::ChainId;
use bridge_runtime_common::messages::{
	self,
	source::{self, FromBridgedChainMessagesDeliveryProof, FromThisChainMessagePayload},
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
// --- darwinia-network ---
use crate::*;
use bridge_primitives::{
	DarwiniaFromThisChainMessageVerifier, PANGOLIN_CHAIN_ID, PANGORO_CHAIN_ID,
	PANGORO_PANGOLIN_LANE, WITH_PANGOLIN_MESSAGES_PALLET_NAME,
};
use darwinia_support::to_bytes32;
use dp_asset::token::Token;
use dp_s2s::{CallParams, EncodeCall};

/// Message payload for Pangolin -> Pangoro messages.
pub type ToPangoroMessagePayload = FromThisChainMessagePayload<WithPangoroMessageBridge>;
/// Message verifier for Pangolin -> Pangoro messages.
pub type ToPangoroMessageVerifier<R> =
	DarwiniaFromThisChainMessageVerifier<WithPangoroMessageBridge, R>;
/// Message payload for Pangoro -> Pangolin messages.
pub type FromPangoroMessagePayload = FromBridgedChainMessagePayload<WithPangoroMessageBridge>;
/// Encoded Pangolin Call as it comes from Pangoro.
pub type FromPangoroEncodedCall = FromBridgedChainEncodedMessageCall<crate::Call>;
/// Messages proof for Pangoro -> Pangolin messages.
type FromPangoroMessagesProof = FromBridgedChainMessagesProof<pangoro_primitives::Hash>;
/// Messages delivery proof for Pangolin -> Pangoro messages.
type ToPangoroMessagesDeliveryProof =
	FromBridgedChainMessagesDeliveryProof<pangoro_primitives::Hash>;
/// Call-dispatch based message dispatch for Pangoro -> Pangolin messages.
pub type FromPangoroMessageDispatch =
	FromBridgedChainMessageDispatch<WithPangoroMessageBridge, Runtime, Ring, ()>;

/// Initial value of `PangoroToPangolinConversionRate` parameter.
pub const INITIAL_PANGORO_TO_PANGOLIN_CONVERSION_RATE: FixedU128 =
	FixedU128::from_inner(FixedU128::DIV);

frame_support::parameter_types! {
	/// Pangoro to Pangolin conversion rate. Initially we treat both tokens as equal.
	pub storage PangoroToPangolinConversionRate: FixedU128 = INITIAL_PANGORO_TO_PANGOLIN_CONVERSION_RATE;
}

/// Pangolin -> Pangoro message lane pallet parameters.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug)]
pub enum PangolinToPangoroMessagesParameter {
	/// The conversion formula we use is: `PangolinTokens = PangoroTokens * conversion_rate`.
	PangoroToPangolinConversionRate(FixedU128),
}
impl MessagesParameter for PangolinToPangoroMessagesParameter {
	fn save(&self) {
		match *self {
			PangolinToPangoroMessagesParameter::PangoroToPangolinConversionRate(
				ref conversion_rate,
			) => PangoroToPangolinConversionRate::set(conversion_rate),
		}
	}
}

/// Pangoro <-> Pangolin message bridge.
#[derive(Clone, Copy, RuntimeDebug)]
pub struct WithPangoroMessageBridge;
impl MessageBridge for WithPangoroMessageBridge {
	const RELAYER_FEE_PERCENT: u32 = 10;
	const THIS_CHAIN_ID: ChainId = PANGOLIN_CHAIN_ID;
	const BRIDGED_CHAIN_ID: ChainId = PANGORO_CHAIN_ID;
	const BRIDGED_MESSAGES_PALLET_NAME: &'static str = WITH_PANGOLIN_MESSAGES_PALLET_NAME;

	type ThisChain = Pangolin;
	type BridgedChain = Pangoro;

	fn bridged_balance_to_this_balance(bridged_balance: pangoro_primitives::Balance) -> Balance {
		Balance::try_from(
			PangoroToPangolinConversionRate::get().saturating_mul_int(bridged_balance),
		)
		.unwrap_or(Balance::MAX)
	}
}

/// Pangolin chain from message lane point of view.
#[derive(Clone, Copy, RuntimeDebug)]
pub struct Pangolin;
impl messages::ChainWithMessages for Pangolin {
	type Hash = Hash;
	type AccountId = AccountId;
	type Signer = AccountPublic;
	type Signature = Signature;
	type Weight = Weight;
	type Balance = Balance;
}
impl messages::ThisChainWithMessages for Pangolin {
	type Call = Call;

	fn is_outbound_lane_enabled(lane: &LaneId) -> bool {
		*lane == [0, 0, 0, 0] || *lane == [0, 0, 0, 1] || *lane == PANGORO_PANGOLIN_LANE
	}

	fn maximal_pending_messages_at_outbound_lane() -> MessageNonce {
		MessageNonce::MAX
	}

	fn estimate_delivery_confirmation_transaction() -> MessageTransaction<Weight> {
		let inbound_data_size = InboundLaneData::<AccountId>::encoded_size_hint(
			bridge_primitives::MAXIMAL_ENCODED_ACCOUNT_ID_SIZE,
			1,
			1,
		)
		.unwrap_or(u32::MAX);

		MessageTransaction {
			dispatch_weight: bridge_primitives::MAX_SINGLE_MESSAGE_DELIVERY_CONFIRMATION_TX_WEIGHT,
			size: inbound_data_size
				.saturating_add(bridge_primitives::EXTRA_STORAGE_PROOF_SIZE)
				.saturating_add(bridge_primitives::TX_EXTRA_BYTES),
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

/// Pangoro chain from message lane point of view.
#[derive(Clone, Copy, RuntimeDebug)]
pub struct Pangoro;
impl messages::ChainWithMessages for Pangoro {
	type Hash = pangoro_primitives::Hash;
	type AccountId = pangoro_primitives::AccountId;
	type Signer = pangoro_primitives::AccountPublic;
	type Signature = pangoro_primitives::Signature;
	type Weight = Weight;
	type Balance = pangoro_primitives::Balance;
}
impl messages::BridgedChainWithMessages for Pangoro {
	fn maximal_extrinsic_size() -> u32 {
		pangoro_runtime_system_params::max_extrinsic_size()
	}

	fn message_weight_limits(_message_payload: &[u8]) -> RangeInclusive<Weight> {
		// we don't want to relay too large messages + keep reserve for future upgrades
		let upper_limit = messages::target::maximal_incoming_message_dispatch_weight(
			pangoro_runtime_system_params::max_extrinsic_weight(),
		);

		// we're charging for payload bytes in `WithPangoroMessageBridge::transaction_payment` function
		//
		// this bridge may be used to deliver all kind of messages, so we're not making any assumptions about
		// minimal dispatch weight here

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
				.saturating_mul(bridge_primitives::ADDITIONAL_MESSAGE_BYTE_DELIVERY_WEIGHT)
				.saturating_add(bridge_primitives::DEFAULT_MESSAGE_DELIVERY_TX_WEIGHT)
				.saturating_add(message_dispatch_weight)
				.saturating_sub(if include_pay_dispatch_fee_cost {
					0
				} else {
					bridge_primitives::PAY_INBOUND_DISPATCH_FEE_WEIGHT
				}),
			size: message_payload_len
				.saturating_add(bridge_primitives::EXTRA_STORAGE_PROOF_SIZE)
				.saturating_add(bridge_primitives::TX_EXTRA_BYTES),
		}
	}

	fn transaction_payment(transaction: MessageTransaction<Weight>) -> pangoro_primitives::Balance {
		// in our testnets, both per-byte fee and weight-to-fee are 1:1
		messages::transaction_payment(
			pangoro_runtime_system_params::RuntimeBlockWeights::get()
				.get(DispatchClass::Normal)
				.base_extrinsic,
			1,
			FixedU128::zero(),
			|weight| weight as _,
			transaction,
		)
	}
}
impl TargetHeaderChain<ToPangoroMessagePayload, pangoro_primitives::AccountId> for Pangoro {
	type Error = &'static str;
	// The proof is:
	// - hash of the header this proof has been created with;
	// - the storage proof or one or several keys;
	// - id of the lane we prove state of.
	type MessagesDeliveryProof = ToPangoroMessagesDeliveryProof;

	fn verify_message(payload: &ToPangoroMessagePayload) -> Result<(), Self::Error> {
		source::verify_chain_message::<WithPangoroMessageBridge>(payload)
	}

	fn verify_messages_delivery_proof(
		proof: Self::MessagesDeliveryProof,
	) -> Result<(LaneId, InboundLaneData<AccountId>), Self::Error> {
		source::verify_messages_delivery_proof::<
			WithPangoroMessageBridge,
			Runtime,
			WithPangoroGrandpa,
		>(proof)
	}
}
impl SourceHeaderChain<pangoro_primitives::Balance> for Pangoro {
	type Error = &'static str;
	// The proof is:
	// - hash of the header this proof has been created with;
	// - the storage proof or one or several keys;
	// - id of the lane we prove messages for;
	// - inclusive range of messages nonces that are proved.
	type MessagesProof = FromPangoroMessagesProof;

	fn verify_messages_proof(
		proof: Self::MessagesProof,
		messages_count: u32,
	) -> Result<ProvedMessages<Message<pangoro_primitives::Balance>>, Self::Error> {
		target::verify_messages_proof::<WithPangoroMessageBridge, Runtime, WithPangoroGrandpa>(
			proof,
			messages_count,
		)
	}
}

/// Pangoro chain's dispatch call info
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub enum PangoroRuntime {
	/// NOTE: The index must be the same as the backing pallet in the pangoro runtime
	#[codec(index = 20)]
	Sub2SubBacking(PangoroSub2SubBackingCall),
}

/// Something important to note:
/// The index below represent the call order in the pangolin issuing pallet call.
/// You must update the index here if you change the call order in Pangolin runtime.
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[allow(non_camel_case_types)]
pub enum PangoroSub2SubBackingCall {
	/// NOTE: The index depends on the call order in the s2s backing pallet.
	#[codec(index = 2)]
	unlock_from_remote(Token, AccountId),
}

/// Generate concrete dispatch call data
pub struct PangoroRuntimeCallsEncoder;
impl EncodeCall<AccountId> for PangoroRuntimeCallsEncoder {
	fn encode_call(call_params: CallParams<AccountId>) -> Result<Vec<u8>, ()> {
		let call = match call_params {
			CallParams::UnlockFromRemote(_account_id, unlock_info) => {
				if unlock_info.recipient.len() != 32 {
					return Err(());
				}

				let recipient_id: AccountId = to_bytes32(unlock_info.recipient.as_slice()).into();
				PangoroRuntime::Sub2SubBacking(PangoroSub2SubBackingCall::unlock_from_remote(
					unlock_info.token,
					recipient_id,
				))
				.encode()
			}
			_ => return Err(()),
		};
		Ok(call)
	}
}
