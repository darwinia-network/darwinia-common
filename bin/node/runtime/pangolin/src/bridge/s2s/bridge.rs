// --- darwinia ---
use crate::*;
use darwinia_balances::Instance0 as RingInstance;

// --- s2s bridger ---
use bp_messages::{LaneId, MessageNonce, UnrewardedRelayersState};
pub use pallet_bridge_grandpa::Call as BridgeGrandpaMillauCall;
pub use pallet_bridge_messages::Call as MessagesCall;
// --- frame ---
use frame_support::{Parameter, RuntimeDebug};
use sp_runtime::traits::Convert;
use sp_runtime::{MultiSignature, MultiSigner};

pub type WithMillauMessagesInstance = pallet_bridge_messages::Instance2;
pub type WithMillauGrandpaInstance = pallet_bridge_grandpa::Instance2;
pub type WithMillauDispatchInstance = pallet_bridge_dispatch::Instance2;

frame_support::parameter_types! {
	// This is a pretty unscientific cap.
	//
	// Note that once this is hit the pallet will essentially throttle incoming requests down to one
	// call per block.
	pub const MaxRequests: u32 = 50;

	// Number of headers to keep.
	//
	// Assuming the worst case of every header being finalized, we will keep headers at least for a
	// week.
	pub const HeadersToKeep: u32 = 7 * crate::time_units::DAYS as u32;
}

frame_support::parameter_types! {
	pub const MaxMessagesToPruneAtOnce: bp_messages::MessageNonce = 8;
	pub const MaxUnrewardedRelayerEntriesAtInboundLane: bp_messages::MessageNonce =
		MAX_UNREWARDED_RELAYER_ENTRIES_AT_INBOUND_LANE;
	pub const MaxUnconfirmedMessagesAtInboundLane: bp_messages::MessageNonce =
		MAX_UNCONFIRMED_MESSAGES_AT_INBOUND_LANE;
	// `IdentityFee` is used by Rialto => we may use weight directly
	pub const GetDeliveryConfirmationTransactionFee: Balance =
		MAX_SINGLE_MESSAGE_DELIVERY_CONFIRMATION_TX_WEIGHT as _;
	pub const RootAccountForPayments: Option<AccountId> = None;
}

// We use this to get the account on Rialto (target) which is derived from Millau's (source)
// account. We do this so we can fund the derived account on Rialto at Genesis to it can pay
// transaction fees.
//
// The reason we can use the same `AccountId` type for both chains is because they share the same
// development seed phrase.
//
// Note that this should only be used for testing.
pub fn derive_account_from_millau_id(id: bp_runtime::SourceAccount<AccountId>) -> AccountId {
	let encoded_id = bp_runtime::derive_account_id(bp_runtime::MILLAU_BRIDGE_INSTANCE, id);
	AccountIdConverter::convert(encoded_id)
}

impl pallet_bridge_grandpa::Config<WithMillauGrandpaInstance> for Runtime {
	type BridgedChain = bp_millau::Millau;
	type MaxRequests = MaxRequests;
	type HeadersToKeep = HeadersToKeep;
	// todo: there need use real weight for pangolin
	type WeightInfo = pallet_bridge_grandpa::weights::RialtoWeight<Runtime>;
}

impl pallet_shift_session_manager::Config for Runtime {}

impl pallet_bridge_messages::Config<WithMillauMessagesInstance> for Runtime {
	type Event = Event;
	// todo: there need use real weight for pangolin
	type WeightInfo = pallet_bridge_messages::weights::RialtoWeight<Runtime>;
	type Parameter = millau_messages::RialtoToMillauMessagesParameter;
	type MaxMessagesToPruneAtOnce = MaxMessagesToPruneAtOnce;
	type MaxUnrewardedRelayerEntriesAtInboundLane = MaxUnrewardedRelayerEntriesAtInboundLane;
	type MaxUnconfirmedMessagesAtInboundLane = MaxUnconfirmedMessagesAtInboundLane;

	type OutboundPayload = crate::millau_messages::ToMillauMessagePayload;
	type OutboundMessageFee = Balance;

	type InboundPayload = crate::millau_messages::FromMillauMessagePayload;
	type InboundMessageFee = bp_millau::Balance;
	type InboundRelayer = bp_millau::AccountId;

	type AccountIdConverter = AccountIdConverter;

	type TargetHeaderChain = crate::millau_messages::Millau;
	type LaneMessageVerifier = crate::millau_messages::ToMillauMessageVerifier;
	type MessageDeliveryAndDispatchPayment =
		pallet_bridge_messages::instant_payments::InstantCurrencyPayments<
			Runtime,
			darwinia_balances::Pallet<Runtime, RingInstance>,
			GetDeliveryConfirmationTransactionFee,
			RootAccountForPayments,
		>;

	type SourceHeaderChain = crate::millau_messages::Millau;
	type MessageDispatch = crate::millau_messages::FromMillauMessageDispatch;
}

impl pallet_bridge_dispatch::Config<WithMillauDispatchInstance> for Runtime {
	type Event = Event;
	type MessageId = (bp_messages::LaneId, bp_messages::MessageNonce);
	type Call = Call;
	type CallFilter = ();
	type EncodedCall = crate::millau_messages::FromMillauEncodedCall;
	type SourceChainAccountId = bp_millau::AccountId;
	type TargetChainAccountPublic = MultiSigner;
	type TargetChainSignature = MultiSignature;
	type AccountIdConverter = AccountIdConverter;
}

/// The target length of a session (how often authorities change) on Millau measured in of number of
/// blocks.
///
/// Note that since this is a target sessions may change before/after this time depending on network
/// conditions.
pub const SESSION_LENGTH: BlockNumber = 5 * time_units::MINUTES;

/// Human readable time units defined in terms of number of blocks.
pub mod time_units {
	use super::BlockNumber;

	pub const MILLISECS_PER_BLOCK: u64 = 6000;
	pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

	pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
	pub const HOURS: BlockNumber = MINUTES * 60;
	pub const DAYS: BlockNumber = HOURS * 24;
}

/// Number of bytes, included in the signed Pangolin transaction apart from the encoded call itself.
///
/// Can be computed by subtracting encoded call size from raw transaction size.
pub const TX_EXTRA_BYTES: u32 = 103;

/// Maximal size (in bytes) of encoded (using `Encode::encode()`) account id.
pub const MAXIMAL_ENCODED_ACCOUNT_ID_SIZE: u32 = 32;

/// Maximal number of unrewarded relayer entries at inbound lane.
pub const MAX_UNREWARDED_RELAYER_ENTRIES_AT_INBOUND_LANE: MessageNonce = 128;

/// Maximal number of unconfirmed messages at inbound lane.
pub const MAX_UNCONFIRMED_MESSAGES_AT_INBOUND_LANE: MessageNonce = 128;

/// Maximal weight of single message delivery confirmation transaction on Pangolin chain.
///
/// This value is a result of `pallet_bridge_messages::Pallet::receive_messages_delivery_proof` weight formula computation
/// for the case when single message is confirmed. The result then must be rounded up to account possible future
/// runtime upgrades.
pub const MAX_SINGLE_MESSAGE_DELIVERY_CONFIRMATION_TX_WEIGHT: Weight = 2_000_000_000;

/// Number of extra bytes (excluding size of storage value itself) of storage proof, built at
/// Pangolin chain. This mostly depends on number of entries (and their density) in the storage trie.
/// Some reserve is reserved to account future chain growth.
pub const EXTRA_STORAGE_PROOF_SIZE: u32 = 1024;

/// Name of the `PangolinFinalityApi::best_finalized` runtime method.
pub const BEST_FINALIZED_PANGOLIN_HEADER_METHOD: &str = "PangolinFinalityApi_best_finalized";

/// Name of the `FromPangolinInboundLaneApi::latest_received_nonce` runtime method.
pub const FROM_PANGOLIN_LATEST_RECEIVED_NONCE_METHOD: &str =
	"FromPangolinInboundLaneApi_latest_received_nonce";
/// Name of the `FromPangolinInboundLaneApi::latest_onfirmed_nonce` runtime method.
pub const FROM_PANGOLIN_LATEST_CONFIRMED_NONCE_METHOD: &str =
	"FromPangolinInboundLaneApi_latest_confirmed_nonce";
/// Name of the `FromPangolinInboundLaneApi::unrewarded_relayers_state` runtime method.
pub const FROM_PANGOLIN_UNREWARDED_RELAYERS_STATE: &str =
	"FromPangolinInboundLaneApi_unrewarded_relayers_state";

/// Name of the `ToPangolinOutboundLaneApi::estimate_message_delivery_and_dispatch_fee` runtime method.
pub const TO_PANGOLIN_ESTIMATE_MESSAGE_FEE_METHOD: &str =
	"ToPangolinOutboundLaneApi_estimate_message_delivery_and_dispatch_fee";
/// Name of the `ToPangolinOutboundLaneApi::messages_dispatch_weight` runtime method.
pub const TO_PANGOLIN_MESSAGES_DISPATCH_WEIGHT_METHOD: &str =
	"ToPangolinOutboundLaneApi_messages_dispatch_weight";
/// Name of the `ToPangolinOutboundLaneApi::latest_generated_nonce` runtime method.
pub const TO_PANGOLIN_LATEST_GENERATED_NONCE_METHOD: &str =
	"ToPangolinOutboundLaneApi_latest_generated_nonce";
/// Name of the `ToPangolinOutboundLaneApi::latest_received_nonce` runtime method.
pub const TO_PANGOLIN_LATEST_RECEIVED_NONCE_METHOD: &str =
	"ToPangolinOutboundLaneApi_latest_received_nonce";

/// Increase of delivery transaction weight on Pangolin chain with every additional message byte.
///
/// This value is a result of `pallet_bridge_messages::WeightInfoExt::storage_proof_size_overhead(1)` call. The
/// result then must be rounded up to account possible future runtime upgrades.
pub const ADDITIONAL_MESSAGE_BYTE_DELIVERY_WEIGHT: Weight = 25_000;

/// Weight of single regular message delivery transaction on Pangolin chain.
///
/// This value is a result of `pallet_bridge_messages::Pallet::receive_messages_proof_weight()` call
/// for the case when single message of `pallet_bridge_messages::EXPECTED_DEFAULT_MESSAGE_LENGTH` bytes is delivered.
/// The message must have dispatch weight set to zero. The result then must be rounded up to account
/// possible future runtime upgrades.
pub const DEFAULT_MESSAGE_DELIVERY_TX_WEIGHT: Weight = 1_000_000_000;

/// Convert a 256-bit hash into an AccountId.
pub struct AccountIdConverter;

impl Convert<sp_core::H256, AccountId> for AccountIdConverter {
	fn convert(hash: sp_core::H256) -> AccountId {
		hash.to_fixed_bytes().into()
	}
}

/// Pangolin chain.
#[derive(RuntimeDebug)]
pub struct PangolinSubstrateChain;

impl bp_runtime::Chain for PangolinSubstrateChain {
	type BlockNumber = BlockNumber;
	type Hash = Hash;
	type Hasher = Hashing;
	type Header = Header;
}

sp_api::decl_runtime_apis! {
	/// API for querying information about the finalized Pangolin headers.
	///
	/// This API is implemented by runtimes that are bridging with the Pangolin chain, not the
	/// Millau runtime itself.
	pub trait PangolinFinalityApi {
		/// Returns number and hash of the best finalized header known to the bridge module.
		fn best_finalized() -> (BlockNumber, Hash);
		/// Returns true if the header is known to the runtime.
		fn is_known_header(hash: Hash) -> bool;
	}

	/// Outbound message lane API for messages that are sent to Pangolin chain.
	///
	/// This API is implemented by runtimes that are sending messages to Pangolin chain, not the
	/// Pangolin runtime itself.
	pub trait ToPangolinOutboundLaneApi<OutboundMessageFee: Parameter, OutboundPayload: Parameter> {
		/// Estimate message delivery and dispatch fee that needs to be paid by the sender on
		/// this chain.
		///
		/// Returns `None` if message is too expensive to be sent to Pangolin from this chain.
		///
		/// Please keep in mind that this method returns lowest message fee required for message
		/// to be accepted to the lane. It may be good idea to pay a bit over this price to account
		/// future exchange rate changes and guarantee that relayer would deliver your message
		/// to the target chain.
		fn estimate_message_delivery_and_dispatch_fee(
			lane_id: LaneId,
			payload: OutboundPayload,
		) -> Option<OutboundMessageFee>;
		/// Returns total dispatch weight and encoded payload size of all messages in given inclusive range.
		///
		/// If some (or all) messages are missing from the storage, they'll also will
		/// be missing from the resulting vector. The vector is ordered by the nonce.
		fn messages_dispatch_weight(
			lane: LaneId,
			begin: MessageNonce,
			end: MessageNonce,
		) -> Vec<(MessageNonce, Weight, u32)>;
		/// Returns nonce of the latest message, received by bridged chain.
		fn latest_received_nonce(lane: LaneId) -> MessageNonce;
		/// Returns nonce of the latest message, generated by given lane.
		fn latest_generated_nonce(lane: LaneId) -> MessageNonce;
	}

	/// Inbound message lane API for messages sent by Pangolin chain.
	///
	/// This API is implemented by runtimes that are receiving messages from Pangolin chain, not the
	/// Pangolin runtime itself.
	pub trait FromPangolinInboundLaneApi {
		/// Returns nonce of the latest message, received by given lane.
		fn latest_received_nonce(lane: LaneId) -> MessageNonce;
		/// Nonce of latest message that has been confirmed to the bridged chain.
		fn latest_confirmed_nonce(lane: LaneId) -> MessageNonce;
		/// State of the unrewarded relayers set at given lane.
		fn unrewarded_relayers_state(lane: LaneId) -> UnrewardedRelayersState;
	}
}
