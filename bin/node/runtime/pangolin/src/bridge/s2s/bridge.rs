// --- darwinia ---
use crate::*;
use darwinia_balances::{Instance0 as RingInstance};

// --- s2s bridger ---
pub use pallet_bridge_grandpa::Call as BridgeGrandpaMillauCall;
pub use pallet_bridge_messages::Call as MessagesCall;
// --- frame ---
use frame_system::limits;
use sp_runtime::{MultiSigner, MultiSignature};
use sp_runtime::traits::Convert;

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
	pub const HeadersToKeep: u32 = 7 * drml_primitives::time_units::DAYS as u32;
}


frame_support::parameter_types! {
	pub const MaxMessagesToPruneAtOnce: bp_messages::MessageNonce = 8;
	pub const MaxUnrewardedRelayerEntriesAtInboundLane: bp_messages::MessageNonce =
		drml_primitives::MAX_UNREWARDED_RELAYER_ENTRIES_AT_INBOUND_LANE;
	pub const MaxUnconfirmedMessagesAtInboundLane: bp_messages::MessageNonce =
		drml_primitives::MAX_UNCONFIRMED_MESSAGES_AT_INBOUND_LANE;
	// `IdentityFee` is used by Rialto => we may use weight directly
	pub const GetDeliveryConfirmationTransactionFee: Balance =
		drml_primitives::MAX_SINGLE_MESSAGE_DELIVERY_CONFIRMATION_TX_WEIGHT as _;
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
	type MessageDeliveryAndDispatchPayment = pallet_bridge_messages::instant_payments::InstantCurrencyPayments<
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

