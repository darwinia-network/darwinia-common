//! DRML types shared between the runtime and the Node-side code.

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]

// --- substrate ---
use bp_message_lane::{LaneId, MessageNonce, UnrewardedRelayersState};
use bp_runtime::Chain;
use frame_support::weights::Weight;
use sp_core::H256;
use sp_runtime::{
	generic,
	traits::{BlakeTwo256, Convert, IdentifyAccount, Verify},
	MultiSignature, MultiSigner, OpaqueExtrinsic,
};
use sp_std::prelude::*;

/// An index to a block.
/// 32-bits will allow for 136 years of blocks assuming 1 block per second.
pub type BlockNumber = u32;

/// An instant or duration in time.
pub type Moment = u64;

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
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

/// Block type.
pub type OpaqueBlock = generic::Block<Header, OpaqueExtrinsic>;

/// The type of an object that can produce hashes on Song.
pub type Hasher = BlakeTwo256;

/// Public key of the chain account that may be used to verify signatures.
pub type AccountSigner = MultiSigner;

/// Song chain
pub struct Song;

impl Chain for Song {
	type BlockNumber = BlockNumber;
	type Hash = Hash;
	// TODO: why this hasher
	type Hasher = Hasher;
	type Header = Header;
}

/// Maximal weight of single Rialto block.
pub const MAXIMUM_BLOCK_WEIGHT: Weight = 2_000_000_000_000;
/// Portion of block reserved for regular transactions.
pub const AVAILABLE_BLOCK_RATIO: u32 = 75;
/// Maximal weight of single Rialto extrinsic (65% of maximum block weight = 75% for regular
/// transactions minus 10% for initialization).
pub const MAXIMUM_EXTRINSIC_WEIGHT: Weight =
	MAXIMUM_BLOCK_WEIGHT / 100 * (AVAILABLE_BLOCK_RATIO as Weight - 10);
/// Maximal size of Song block.
pub const MAXIMUM_BLOCK_SIZE: u32 = 5 * 1024 * 1024;
/// Maximal size of single normal Song extrinsic (75% of maximal block size).
pub const MAXIMUM_EXTRINSIC_SIZE: u32 = MAXIMUM_BLOCK_SIZE / 100 * AVAILABLE_BLOCK_RATIO;

// TODO: may need to be updated after https://github.com/paritytech/parity-bridges-common/issues/78
/// Maximal number of messages in single delivery transaction.
pub const MAX_MESSAGES_IN_DELIVERY_TRANSACTION: MessageNonce = 128;
/// Maximal number of unrewarded relayer entries at inbound lane.
pub const MAX_UNREWARDED_RELAYER_ENTRIES_AT_INBOUND_LANE: MessageNonce = 128;
/// Maximal number of unconfirmed messages at inbound lane.
pub const MAX_UNCONFIRMED_MESSAGES_AT_INBOUND_LANE: MessageNonce = 128;

/// Convert a 256-bit hash into an AccountId.
pub struct AccountIdConverter;

impl Convert<sp_core::H256, AccountId> for AccountIdConverter {
	fn convert(hash: sp_core::H256) -> AccountId {
		hash.to_fixed_bytes().into()
	}
}

sp_api::decl_runtime_apis! {
	/// API for querying information about Song headers from the Bridge Pallet instance.
	///
	/// This API is implemented by runtimes that are bridging with Song chain, not the
	/// Song runtime itself.
	pub trait SongHeaderApi {
		/// Returns number and hash of the best blocks known to the bridge module.
		///
		/// Will return multiple headers if there are many headers at the same "best" height.
		///
		/// The caller should only submit an `import_header` transaction that makes
		/// (or leads to making) other header the best one.
		fn best_blocks() -> Vec<(BlockNumber, Hash)>;
		/// Returns number and hash of the best finalized block known to the bridge module.
		fn finalized_block() -> (BlockNumber, Hash);
		/// Returns numbers and hashes of headers that require finality proofs.
		///
		/// An empty response means that there are no headers which currently require a
		/// finality proof.
		fn incomplete_headers() -> Vec<(BlockNumber, Hash)>;
		/// Returns true if the header is known to the runtime.
		fn is_known_block(hash: Hash) -> bool;
		/// Returns true if the header is considered finalized by the runtime.
		fn is_finalized_block(hash: Hash) -> bool;
	}

	/// Outbound message lane API for messages that are sent to Song chain.
	///
	/// This API is implemented by runtimes that are sending messages to Song chain, not the
	/// Song runtime itself.
	pub trait ToSongOutboundLaneApi {
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

	/// Inbound message lane API for messages sent by Song chain.
	///
	/// This API is implemented by runtimes that are receiving messages from Song chain, not the
	/// Song runtime itself.
	pub trait FromSongInboundLaneApi {
		/// Returns nonce of the latest message, received by given lane.
		fn latest_received_nonce(lane: LaneId) -> MessageNonce;
		/// Nonce of latest message that has been confirmed to the bridged chain.
		fn latest_confirmed_nonce(lane: LaneId) -> MessageNonce;
		/// State of the unrewarded relayers set at given lane.
		fn unrewarded_relayers_state(lane: LaneId) -> UnrewardedRelayersState;
	}
}
