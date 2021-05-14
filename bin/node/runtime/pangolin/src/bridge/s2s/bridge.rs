// --- darwinia ---
use crate::*;
use darwinia_balances::Instance1 as RingInstance;
use pangolin_bridge_primitives as s2s_params;

// --- s2s bridger ---
use bp_messages::{LaneId, MessageNonce, UnrewardedRelayersState};
pub use pallet_bridge_grandpa::Call as BridgeGrandpaMillauCall;
pub use pallet_bridge_messages::Call as MessagesCall;
// --- frame ---
use frame_support::{Parameter, RuntimeDebug};
use sp_runtime::traits::Convert;
use sp_runtime::{MultiSignature, MultiSigner};

pub type WithMillauMessagesInstance = pallet_bridge_messages::Instance4;
pub type WithMillauGrandpaInstance = pallet_bridge_grandpa::Instance4;
pub type WithMillauDispatchInstance = pallet_bridge_dispatch::Instance4;

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
	pub const HeadersToKeep: u32 = 7 * s2s_params::time_units::DAYS as u32;
}

frame_support::parameter_types! {
	pub const MaxMessagesToPruneAtOnce: bp_messages::MessageNonce = 8;
	pub const MaxUnrewardedRelayerEntriesAtInboundLane: bp_messages::MessageNonce =
		s2s_params::MAX_UNREWARDED_RELAYER_ENTRIES_AT_INBOUND_LANE;
	pub const MaxUnconfirmedMessagesAtInboundLane: bp_messages::MessageNonce =
		s2s_params::MAX_UNCONFIRMED_MESSAGES_AT_INBOUND_LANE;
	// `IdentityFee` is used by Rialto => we may use weight directly
	pub const GetDeliveryConfirmationTransactionFee: Balance =
		s2s_params::MAX_SINGLE_MESSAGE_DELIVERY_CONFIRMATION_TX_WEIGHT as _;
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
	let encoded_id = bp_runtime::derive_account_id(bp_runtime::MILLAU_CHAIN_ID, id);
	s2s_params::AccountIdConverter::convert(encoded_id)
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

	type AccountIdConverter = s2s_params::AccountIdConverter;

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
	type AccountIdConverter = s2s_params::AccountIdConverter;
}
