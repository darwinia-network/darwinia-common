//! Millau types shared between the runtime and the Node-side code.

#![cfg_attr(not(feature = "std"), no_std)]

pub mod time {
	// --- darwinia ---
	use crate::BlockNumber;

	pub const MILLISECS_PER_BLOCK: u64 = 6000;
	pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

	pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
	pub const HOURS: BlockNumber = MINUTES * 60;
	pub const DAYS: BlockNumber = HOURS * 24;
}
pub use time::*;

// --- paritytech ---
use bp_messages::{LaneId, MessageNonce, UnrewardedRelayersState};
use bp_runtime::Chain;
use frame_support::{
	weights::{
		constants::{BlockExecutionWeight, ExtrinsicBaseWeight, WEIGHT_PER_SECOND},
		DispatchClass, Weight,
	},
	Parameter, RuntimeDebug,
};
use frame_system::limits::{BlockLength, BlockWeights};
#[cfg(feature = "std")]
use sp_core::sr25519::Pair;
use sp_core::H256;
use sp_runtime::{
	generic,
	traits::{BlakeTwo256, Convert, IdentifyAccount, Verify},
	MultiSignature, OpaqueExtrinsic, Perbill,
};
use sp_std::prelude::*;

/// An index to a block.
/// 32-bits will allow for 136 years of blocks assuming 1 block per second.
pub type BlockNumber = u32;

/// An instant or duration in time.
pub type Moment = u64;

/// DRML signing params.
#[cfg(feature = "std")]
pub type SigningParams = Pair;

/// Alias to type for a signature for a transaction on the relay chain. This allows one of several
/// kinds of underlying crypto to be used, so isn't a fixed size when encoded.
pub type Signature = MultiSignature;

/// Alias to the public key used for this chain, actually a `MultiSigner`. Like the signature, this
/// also isn't a fixed size when encoded, as different cryptos have different size public keys.
pub type AccountPublic = <Signature as Verify>::Signer;

/// Alias to the opaque account ID type for this chain, actually a `AccountId32`. This is always
/// 32 bytes.
pub type AccountId = <AccountPublic as IdentifyAccount>::AccountId;

/// The type for looking up accounts. We don't expect more than 4 billion of them.
pub type AccountIndex = u32;

/// A hash of some data used by the relay chain.
pub type Hash = H256;

/// Hashing algorithm used by the chain.
pub type Hashing = BlakeTwo256;

/// Index of a transaction in the relay chain. 32-bit should be plenty.
pub type Nonce = u32;

/// The balance of an account.
/// 128-bits (or 38 significant decimal figures) will allow for 10m currency (10^7) at a resolution
/// to all for one second's worth of an annualised 50% reward be paid to a unit holder (10^11 unit
/// denomination), or 10^18 total atomic units, to grow at 50%/year for 51 years (10^9 multiplier)
/// for an eventual total of 10^27 units (27 significant decimal figures).
/// We round denomination to 10^12 (12 sdf), and leave the other redundancy at the upper end so
/// that 32 bits may be multiplied with a balance in 128 bits without worrying about overflow.
pub type Balance = u128;

/// The power of an account.
pub type Power = u32;

/// Header type.
pub type Header = generic::Header<BlockNumber, Hashing>;

/// Block type.
pub type OpaqueBlock = generic::Block<Header, OpaqueExtrinsic>;

/// Number of extra bytes (excluding size of storage value itself) of storage proof, built at
/// Millau chain. This mostly depends on number of entries (and their density) in the storage trie.
/// Some reserve is reserved to account future chain growth.
pub const EXTRA_STORAGE_PROOF_SIZE: u32 = 1024;

/// Number of bytes, included in the signed Millau transaction apart from the encoded call itself.
///
/// Can be computed by subtracting encoded call size from raw transaction size.
pub const TX_EXTRA_BYTES: u32 = 103;

/// Maximal size (in bytes) of encoded (using `Encode::encode()`) account id.
pub const MAXIMAL_ENCODED_ACCOUNT_ID_SIZE: u32 = 32;

/// Maximum weight of single Millau block.
///
/// This represents 0.5 seconds of compute assuming a target block time of six seconds.
pub const MAXIMUM_BLOCK_WEIGHT: Weight = WEIGHT_PER_SECOND / 2;

/// Represents the average portion of a block's weight that will be used by an
/// `on_initialize()` runtime call.
pub const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);

/// Represents the portion of a block that will be used by Normal extrinsics.
pub const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

/// Maximal number of unrewarded relayer entries at inbound lane.
pub const MAX_UNREWARDED_RELAYER_ENTRIES_AT_INBOUND_LANE: MessageNonce = 1024;

/// Maximal number of unconfirmed messages at inbound lane.
pub const MAX_UNCONFIRMED_MESSAGES_AT_INBOUND_LANE: MessageNonce = 1024;

/// Weight of single regular message delivery transaction on Millau chain.
///
/// This value is a result of `pallet_bridge_messages::Pallet::receive_messages_proof_weight()` call
/// for the case when single message of `pallet_bridge_messages::EXPECTED_DEFAULT_MESSAGE_LENGTH` bytes is delivered.
/// The message must have dispatch weight set to zero. The result then must be rounded up to account
/// possible future runtime upgrades.
pub const DEFAULT_MESSAGE_DELIVERY_TX_WEIGHT: Weight = 1_000_000_000;

/// Increase of delivery transaction weight on Millau chain with every additional message byte.
///
/// This value is a result of `pallet_bridge_messages::WeightInfoExt::storage_proof_size_overhead(1)` call. The
/// result then must be rounded up to account possible future runtime upgrades.
pub const ADDITIONAL_MESSAGE_BYTE_DELIVERY_WEIGHT: Weight = 25_000;

/// Maximal weight of single message delivery confirmation transaction on Millau chain.
///
/// This value is a result of `pallet_bridge_messages::Pallet::receive_messages_delivery_proof` weight formula computation
/// for the case when single message is confirmed. The result then must be rounded up to account possible future
/// runtime upgrades.
pub const MAX_SINGLE_MESSAGE_DELIVERY_CONFIRMATION_TX_WEIGHT: Weight = 2_000_000_000;

/// The target length of a session (how often authorities change) on Millau measured in of number of
/// blocks.
///
/// Note that since this is a target sessions may change before/after this time depending on network
/// conditions.
pub const SESSION_LENGTH: BlockNumber = 5 * MINUTES;

/// Name of the `MillauFinalityApi::best_finalized` runtime method.
pub const BEST_FINALIZED_MILLAU_HEADER_METHOD: &str = "MillauFinalityApi_best_finalized";

/// Name of the `ToMillauOutboundLaneApi::estimate_message_delivery_and_dispatch_fee` runtime method.
pub const TO_MILLAU_ESTIMATE_MESSAGE_FEE_METHOD: &str =
	"ToMillauOutboundLaneApi_estimate_message_delivery_and_dispatch_fee";
/// Name of the `ToMillauOutboundLaneApi::messages_dispatch_weight` runtime method.
pub const TO_MILLAU_MESSAGES_DISPATCH_WEIGHT_METHOD: &str =
	"ToMillauOutboundLaneApi_messages_dispatch_weight";
/// Name of the `ToMillauOutboundLaneApi::latest_received_nonce` runtime method.
pub const TO_MILLAU_LATEST_RECEIVED_NONCE_METHOD: &str =
	"ToMillauOutboundLaneApi_latest_received_nonce";
/// Name of the `ToMillauOutboundLaneApi::latest_generated_nonce` runtime method.
pub const TO_MILLAU_LATEST_GENERATED_NONCE_METHOD: &str =
	"ToMillauOutboundLaneApi_latest_generated_nonce";

/// Name of the `FromMillauInboundLaneApi::latest_received_nonce` runtime method.
pub const FROM_MILLAU_LATEST_RECEIVED_NONCE_METHOD: &str =
	"FromMillauInboundLaneApi_latest_received_nonce";
/// Name of the `FromMillauInboundLaneApi::latest_onfirmed_nonce` runtime method.
pub const FROM_MILLAU_LATEST_CONFIRMED_NONCE_METHOD: &str =
	"FromMillauInboundLaneApi_latest_confirmed_nonce";
/// Name of the `FromMillauInboundLaneApi::unrewarded_relayers_state` runtime method.
pub const FROM_MILLAU_UNREWARDED_RELAYERS_STATE: &str =
	"FromMillauInboundLaneApi_unrewarded_relayers_state";

/// Millau chain.
#[derive(RuntimeDebug)]
pub struct Millau;
impl Chain for Millau {
	type BlockNumber = BlockNumber;
	type Hash = Hash;
	type Hasher = Hashing;
	type Header = Header;
}

/// Convert a 256-bit hash into an AccountId.
pub struct AccountIdConverter;
impl Convert<H256, AccountId> for AccountIdConverter {
	fn convert(hash: H256) -> AccountId {
		hash.to_fixed_bytes().into()
	}
}

frame_support::parameter_types! {
	pub RuntimeBlockLength: BlockLength =
		BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
		.base_block(BlockExecutionWeight::get())
		.for_class(DispatchClass::all(), |weights| {
			weights.base_extrinsic = ExtrinsicBaseWeight::get();
		})
		.for_class(DispatchClass::Normal, |weights| {
			weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
		})
		.for_class(DispatchClass::Operational, |weights| {
			weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
			// Operational transactions have some extra reserved space, so that they
			// are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
			weights.reserved = Some(
				MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
			);
		})
		.avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
		.build_or_panic();
}

/// Get the maximum weight (compute time) that a Normal extrinsic on the Millau chain can use.
pub fn max_extrinsic_weight() -> Weight {
	RuntimeBlockWeights::get()
		.get(DispatchClass::Normal)
		.max_extrinsic
		.unwrap_or(Weight::MAX)
}

/// Get the maximum length in bytes that a Normal extrinsic on the Millau chain requires.
pub fn max_extrinsic_size() -> u32 {
	*RuntimeBlockLength::get().max.get(DispatchClass::Normal)
}

sp_api::decl_runtime_apis! {
	/// API for querying information about the finalized Millau headers.
	///
	/// This API is implemented by runtimes that are bridging with the Millau chain, not the
	/// Millau runtime itself.
	pub trait MillauFinalityApi {
		/// Returns number and hash of the best finalized header known to the bridge module.
		fn best_finalized() -> (BlockNumber, Hash);
		/// Returns true if the header is known to the runtime.
		fn is_known_header(hash: Hash) -> bool;
	}

	/// Outbound message lane API for messages that are sent to Millau chain.
	///
	/// This API is implemented by runtimes that are sending messages to Millau chain, not the
	/// Millau runtime itself.
	pub trait ToMillauOutboundLaneApi<OutboundMessageFee: Parameter, OutboundPayload: Parameter> {
		/// Estimate message delivery and dispatch fee that needs to be paid by the sender on
		/// this chain.
		///
		/// Returns `None` if message is too expensive to be sent to Millau from this chain.
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

	/// Inbound message lane API for messages sent by Millau chain.
	///
	/// This API is implemented by runtimes that are receiving messages from Millau chain, not the
	/// Millau runtime itself.
	pub trait FromMillauInboundLaneApi {
		/// Returns nonce of the latest message, received by given lane.
		fn latest_received_nonce(lane: LaneId) -> MessageNonce;
		/// Nonce of latest message that has been confirmed to the bridged chain.
		fn latest_confirmed_nonce(lane: LaneId) -> MessageNonce;
		/// State of the unrewarded relayers set at given lane.
		fn unrewarded_relayers_state(lane: LaneId) -> UnrewardedRelayersState;
	}
}
