//! Everything required to serve Pangoro <-> Pangolin messages.

// --- crates.io ---
use codec::{Decode, Encode};
use scale_info::TypeInfo;
// --- paritytech ---
use bp_message_dispatch::CallOrigin;
use bp_messages::{
	source_chain::TargetHeaderChain,
	target_chain::{ProvedMessages, SourceHeaderChain},
	InboundLaneData, LaneId, Message, MessageNonce, Parameter as MessagesParameter,
};
use bp_runtime::{messages::DispatchFeePayment, Chain, ChainId};
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
	traits::PalletInfoAccess,
	weights::{DispatchClass, Weight},
	RuntimeDebug,
};
use pallet_bridge_messages::EXPECTED_DEFAULT_MESSAGE_LENGTH;
use sp_runtime::{traits::Zero, FixedPointNumber, FixedU128};
use sp_std::{convert::TryFrom, ops::RangeInclusive};
// --- darwinia-network ---
use crate::*;
pub use darwinia_balances::{Instance1 as RingInstance, Instance2 as KtonInstance};
use darwinia_support::s2s::{LatestMessageNoncer, RelayMessageSender};
use dp_s2s::{CallParams, CreatePayload};
use drml_bridge_primitives::{
	FromThisChainMessageVerifier, PANGOLIN_CHAIN_ID, PANGORO_CHAIN_ID, PANGORO_PANGOLIN_LANE,
	WITH_PANGORO_MESSAGES_PALLET_NAME,
};

/// Message payload for Pangoro -> Pangolin messages.
pub type ToPangolinMessagePayload = FromThisChainMessagePayload<WithPangolinMessageBridge>;
/// The s2s issuing pallet index in the pangolin chain runtime
pub const PANGOLIN_S2S_ISSUING_PALLET_INDEX: u8 = 49;

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct ToPangolinMessageSender;
impl CreatePayload<AccountId, AccountPublic, Signature> for ToPangolinMessageSender {
	type Payload = ToPangolinMessagePayload;

	fn create(
		origin: CallOrigin<AccountId, AccountPublic, Signature>,
		spec_version: u32,
		weight: u64,
		call_params: CallParams,
		dispatch_fee_payment: DispatchFeePayment,
	) -> Result<Self::Payload, &'static str> {
		let call = Self::encode_call(PANGOLIN_S2S_ISSUING_PALLET_INDEX, call_params)?;
		return Ok(ToPangolinMessagePayload {
			spec_version,
			weight,
			origin,
			call,
			dispatch_fee_payment,
		});
	}
}

impl RelayMessageSender for ToPangolinMessageSender {
	fn encode_send_message(
		message_pallet_index: u32,
		lane_id: LaneId,
		payload: Vec<u8>,
		fee: u128,
	) -> Result<Vec<u8>, &'static str> {
		let payload = ToPangolinMessagePayload::decode(&mut payload.as_slice())
			.map_err(|_| "decode pangolin payload failed")?;

		let call: Call = match message_pallet_index {
			_ if message_pallet_index as usize
				== <BridgePangolinMessages as PalletInfoAccess>::index() =>
			{
				BridgeMessagesCall::<Runtime, WithPangolinMessages>::send_message {
					lane_id,
					payload,
					delivery_and_dispatch_fee: fee.saturated_into(),
				}
				.into()
			}
			_ => {
				return Err("invalid pallet index".into());
			}
		};
		Ok(call.encode())
	}
}

impl LatestMessageNoncer for ToPangolinMessageSender {
	fn outbound_latest_generated_nonce(lane_id: LaneId) -> u64 {
		BridgePangolinMessages::outbound_latest_generated_nonce(lane_id).into()
	}

	fn inbound_latest_received_nonce(lane_id: LaneId) -> u64 {
		BridgePangolinMessages::inbound_latest_received_nonce(lane_id).into()
	}
}

/// Message verifier for Pangoro -> Pangolin messages.
pub type ToPangolinMessageVerifier<R> = FromThisChainMessageVerifier<WithPangolinMessageBridge, R>;
/// Message payload for Pangolin -> Pangoro messages.
pub type FromPangolinMessagePayload = FromBridgedChainMessagePayload<WithPangolinMessageBridge>;
/// Encoded Pangoro Call as it comes from Pangolin.
pub type FromPangolinEncodedCall = FromBridgedChainEncodedMessageCall<Call>;
/// Messages proof for Pangolin -> Pangoro messages.
type FromPangolinMessagesProof = FromBridgedChainMessagesProof<pangolin_primitives::Hash>;
/// Messages delivery proof for Pangoro -> Pangolin messages.
type ToPangolinMessagesDeliveryProof =
	FromBridgedChainMessagesDeliveryProof<pangolin_primitives::Hash>;
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
impl MessagesParameter for PangoroToPangolinMessagesParameter {
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
	const RELAYER_FEE_PERCENT: u32 = 10;
	const THIS_CHAIN_ID: ChainId = PANGORO_CHAIN_ID;
	const BRIDGED_CHAIN_ID: ChainId = PANGOLIN_CHAIN_ID;
	const BRIDGED_MESSAGES_PALLET_NAME: &'static str = WITH_PANGORO_MESSAGES_PALLET_NAME;

	type ThisChain = Pangoro;
	type BridgedChain = Pangolin;

	fn bridged_balance_to_this_balance(
		bridged_balance: pangolin_primitives::Balance,
		// TODO: S2S
		_bridged_to_this_conversion_rate_override: Option<FixedU128>,
	) -> pangoro_primitives::Balance {
		pangoro_primitives::Balance::try_from(
			PangolinToPangoroConversionRate::get().saturating_mul_int(bridged_balance),
		)
		.unwrap_or(pangoro_primitives::Balance::MAX)
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
impl messages::ThisChainWithMessages for Pangoro {
	type Call = Call;

	fn is_outbound_lane_enabled(lane: &LaneId) -> bool {
		*lane == [0, 0, 0, 0] || *lane == [0, 0, 0, 1] || *lane == PANGORO_PANGOLIN_LANE
	}

	fn maximal_pending_messages_at_outbound_lane() -> MessageNonce {
		MessageNonce::MAX
	}

	fn estimate_delivery_confirmation_transaction() -> MessageTransaction<Weight> {
		let inbound_data_size =
			InboundLaneData::<pangoro_primitives::AccountId>::encoded_size_hint(
				drml_bridge_primitives::MAXIMAL_ENCODED_ACCOUNT_ID_SIZE,
				1,
				1,
			)
			.unwrap_or(u32::MAX);

		MessageTransaction {
			dispatch_weight:
				drml_bridge_primitives::MAX_SINGLE_MESSAGE_DELIVERY_CONFIRMATION_TX_WEIGHT,
			size: inbound_data_size
				.saturating_add(drml_bridge_primitives::EXTRA_STORAGE_PROOF_SIZE)
				.saturating_add(drml_bridge_primitives::TX_EXTRA_BYTES),
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

/// Pangolin chain from message lane point of view.
#[derive(Clone, Copy, RuntimeDebug)]
pub struct Pangolin;
impl messages::ChainWithMessages for Pangolin {
	type Hash = pangolin_primitives::Hash;
	type AccountId = pangolin_primitives::AccountId;
	type Signer = pangolin_primitives::AccountPublic;
	type Signature = pangolin_primitives::Signature;
	type Weight = Weight;
	type Balance = pangolin_primitives::Balance;
}
impl messages::BridgedChainWithMessages for Pangolin {
	fn maximal_extrinsic_size() -> u32 {
		drml_bridge_primitives::Pangolin::max_extrinsic_size()
	}

	fn message_weight_limits(_message_payload: &[u8]) -> RangeInclusive<Weight> {
		// we don't want to relay too large messages + keep reserve for future upgrades
		let upper_limit = messages::target::maximal_incoming_message_dispatch_weight(
			drml_bridge_primitives::Pangolin::max_extrinsic_weight(),
		);

		// we're charging for payload bytes in `WithPangolinMessageBridge::transaction_payment` function
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
				.saturating_mul(drml_bridge_primitives::ADDITIONAL_MESSAGE_BYTE_DELIVERY_WEIGHT)
				.saturating_add(drml_bridge_primitives::DEFAULT_MESSAGE_DELIVERY_TX_WEIGHT)
				.saturating_add(message_dispatch_weight)
				.saturating_sub(if include_pay_dispatch_fee_cost {
					0
				} else {
					drml_bridge_primitives::PAY_INBOUND_DISPATCH_FEE_WEIGHT
				}),
			size: message_payload_len
				.saturating_add(drml_bridge_primitives::EXTRA_STORAGE_PROOF_SIZE)
				.saturating_add(drml_bridge_primitives::TX_EXTRA_BYTES),
		}
	}

	fn transaction_payment(
		transaction: MessageTransaction<Weight>,
	) -> pangolin_primitives::Balance {
		// in our testnets, both per-byte fee and weight-to-fee are 1:1
		messages::transaction_payment(
			pangolin_runtime_system_params::RuntimeBlockWeights::get()
				.get(DispatchClass::Normal)
				.base_extrinsic,
			1,
			FixedU128::zero(),
			|weight| weight as _,
			transaction,
		)
	}
}
impl TargetHeaderChain<ToPangolinMessagePayload, pangolin_primitives::AccountId> for Pangolin {
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
	) -> Result<(LaneId, InboundLaneData<pangoro_primitives::AccountId>), Self::Error> {
		source::verify_messages_delivery_proof::<
			WithPangolinMessageBridge,
			Runtime,
			WithPangolinGrandpa,
		>(proof)
	}
}
impl SourceHeaderChain<pangolin_primitives::Balance> for Pangolin {
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
	) -> Result<ProvedMessages<Message<pangolin_primitives::Balance>>, Self::Error> {
		target::verify_messages_proof::<WithPangolinMessageBridge, Runtime, WithPangolinGrandpa>(
			proof,
			messages_count,
		)
	}
}
