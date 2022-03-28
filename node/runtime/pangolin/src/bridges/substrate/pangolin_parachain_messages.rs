use bp_rococo::parachains::ParaId;
// --- crates.io ---
use codec::{Decode, Encode};
use scale_info::TypeInfo;
// --- paritytech ---
use bp_messages::{
	source_chain::TargetHeaderChain, target_chain::SourceHeaderChain,
	Parameter as MessagesParameter,
};
use bp_runtime::{Chain, PANGOLIN_CHAIN_ID, PANGOLIN_PARACHAIN_CHAIN_ID};
use bridge_runtime_common::messages::{
	self,
	source::{self, FromBridgedChainMessagesDeliveryProof, FromThisChainMessagePayload},
	target::{
		self, FromBridgedChainEncodedMessageCall, FromBridgedChainMessageDispatch,
		FromBridgedChainMessagePayload, FromBridgedChainMessagesProof,
	},
	MessageBridge,
};
use drml_common_runtime::FromThisChainMessageVerifier;
use frame_support::{
	weights::{DispatchClass, Weight},
	RuntimeDebug,
};
use pallet_bridge_messages::EXPECTED_DEFAULT_MESSAGE_LENGTH;
use sp_runtime::{traits::Zero, FixedPointNumber, FixedU128};
use sp_std::ops::RangeInclusive;
// --- darwinia-network ---
use crate::{bridges::pangoro_messages::Pangolin, *};

/// Identifier of PangolinParachain in the relay chain.
pub const PANGOLIN_PARACHAIN_ID: u32 = 2071;

/// Message verifier for Pangolin -> PangolinParachain messages.
pub type ToPangolinParachainMessageVerifier<R> =
	FromThisChainMessageVerifier<WithPangolinParachainMessageBridge, R>;
/// Message payload for Pangolin -> PangolinParachain messages.
pub type ToPangolinParachainMessagePayload =
	FromThisChainMessagePayload<WithPangolinParachainMessageBridge>;

/// Message payload for PangolinParachain -> Pangolin messages.
pub type FromPangolinParachainMessagePayload =
	FromBridgedChainMessagePayload<WithPangolinParachainMessageBridge>;
/// Call-dispatch based message dispatch for PangolinParachain -> Pangolin messages.
pub type FromPangolinParachainMessageDispatch = FromBridgedChainMessageDispatch<
	WithPangolinParachainMessageBridge,
	Runtime,
	Ring,
	WithPangolinParachainDispatch,
>;

/// Message proof for PangolinParachain -> Pangolin  messages.
type FromPangolinParachainMessagesProof =
	FromBridgedChainMessagesProof<bp_pangolin_parachain::Hash>;
/// Message delivery proof for Pangolin -> PangolinParachain messages.
type ToPangolinParachainMessagesDeliveryProof =
	FromBridgedChainMessagesDeliveryProof<bp_pangolin_parachain::Hash>;

/// Encoded Pangolin Call as it comes from PangolinParachain
pub type FromPangolinParachainEncodedCall = FromBridgedChainEncodedMessageCall<crate::Call>;

pub const INITIAL_PANGOLIN_PARACHAIN_TO_PANGOLIN_CONVERSION_RATE: FixedU128 =
	FixedU128::from_inner(FixedU128::DIV);

frame_support::parameter_types! {
	/// PangolinParachain to Pangolin conversion rate. Initially we trate both tokens as equal.
	pub storage PangolinParachainToPangolinConversionRate: FixedU128 = INITIAL_PANGOLIN_PARACHAIN_TO_PANGOLIN_CONVERSION_RATE;
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum PangolinToPangolinParachainParameter {
	/// The conversion formula we use is: `PangolinTokens = PangolinParachainTokens * conversion_rate`.
	PangolinParachainToPangolinConversionRate(FixedU128),
}

impl MessagesParameter for PangolinToPangolinParachainParameter {
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
	const RELAYER_FEE_PERCENT: u32 = 10;
	const THIS_CHAIN_ID: bp_runtime::ChainId = PANGOLIN_CHAIN_ID;
	// todo change to pangolin parachain id
	const BRIDGED_CHAIN_ID: bp_runtime::ChainId = PANGOLIN_PARACHAIN_CHAIN_ID;
	const BRIDGED_MESSAGES_PALLET_NAME: &'static str =
		bp_pangolin::WITH_PANGOLIN_MESSAGES_PALLET_NAME;

	type ThisChain = Pangolin;

	type BridgedChain = PangolinParachain;

	fn bridged_balance_to_this_balance(
		bridged_balance: messages::BalanceOf<messages::BridgedChain<Self>>,
	) -> Balance {
		Balance::try_from(
			PangolinParachainToPangolinConversionRate::get().saturating_mul_int(bridged_balance),
		)
		.unwrap_or(Balance::MAX)
	}
}

#[derive(Clone, Copy, RuntimeDebug)]
pub struct PangolinParachain;
impl messages::ChainWithMessages for PangolinParachain {
	type Hash = bp_pangolin_parachain::Hash;
	type AccountId = bp_pangolin_parachain::AccountId;
	type Signer = bp_pangolin_parachain::AccountPublic;
	type Signature = bp_pangolin_parachain::Signature;
	type Weight = Weight;
	type Balance = bp_pangolin_parachain::Balance;
}
impl messages::BridgedChainWithMessages for PangolinParachain {
	fn maximal_extrinsic_size() -> u32 {
		bp_pangolin_parachain::PangolinParachain::max_extrinsic_size()
	}

	fn message_weight_limits(_message_payload: &[u8]) -> RangeInclusive<Self::Weight> {
		let upper_limit = messages::target::maximal_incoming_message_dispatch_weight(
			bp_pangolin_parachain::PangolinParachain::max_extrinsic_weight(),
		);
		0..=upper_limit
	}

	fn estimate_delivery_transaction(
		message_payload: &[u8],
		include_pay_dispatch_fee_cost: bool,
		message_dispatch_weight: Weight,
	) -> messages::MessageTransaction<Weight> {
		let message_payload_len = u32::try_from(message_payload.len()).unwrap_or(u32::MAX);
		let extra_bytes_in_payload = Weight::from(message_payload_len)
			.saturating_sub(EXPECTED_DEFAULT_MESSAGE_LENGTH.into());

		messages::MessageTransaction {
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

	fn transaction_payment(
		transaction: messages::MessageTransaction<Weight>,
	) -> bp_pangolin_parachain::Balance {
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
impl TargetHeaderChain<ToPangolinParachainMessagePayload, bp_pangolin_parachain::AccountId>
	for PangolinParachain
{
	type Error = &'static str;

	type MessagesDeliveryProof = ToPangolinParachainMessagesDeliveryProof;

	fn verify_message(payload: &ToPangolinParachainMessagePayload) -> Result<(), Self::Error> {
		source::verify_chain_message::<WithPangolinParachainMessageBridge>(payload)
	}

	fn verify_messages_delivery_proof(
		proof: Self::MessagesDeliveryProof,
	) -> Result<
		(
			bp_messages::LaneId,
			bp_messages::InboundLaneData<bp_pangolin_parachain::AccountId>,
		),
		Self::Error,
	> {
		source::verify_messages_delivery_proof_from_parachain::<
			WithPangolinParachainMessageBridge,
			bp_pangolin_parachain::Header,
			Runtime,
			crate::WithRococoParachainsInstance,
		>(ParaId(PANGOLIN_PARACHAIN_ID), proof)
	}
}
impl SourceHeaderChain<bp_pangolin_parachain::Balance> for PangolinParachain {
	type Error = &'static str;

	type MessagesProof = FromPangolinParachainMessagesProof;

	fn verify_messages_proof(
		proof: Self::MessagesProof,
		messages_count: u32,
	) -> Result<
		bp_messages::target_chain::ProvedMessages<
			bp_messages::Message<bp_pangolin_parachain::Balance>,
		>,
		Self::Error,
	> {
		target::verify_messages_proof_from_parachain::<
			WithPangolinParachainMessageBridge,
			bp_pangolin_parachain::Header,
			Runtime,
			crate::WithRococoParachainsInstance,
		>(ParaId(PANGOLIN_PARACHAIN_ID), proof, messages_count)
	}
}
