//! The Pangoro runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

pub mod pallets;
pub use pallets::*;

pub mod impls {
	pub use darwinia_balances::{Instance1 as RingInstance, Instance2 as KtonInstance};

	// --- substrate ---
	use sp_runtime::RuntimeDebug;
	// --- darwinia ---
	use crate::*;

	darwinia_support::impl_account_data! {
		struct AccountData<Balance>
		for
			RingInstance,
			KtonInstance
		where
			Balance = Balance
		{
			// other data
		}
	}
}
pub use impls::*;

// <--- pangolin
pub mod pangolin_messages;
use pangolin_messages::{ToPangolinMessagePayload, WithPangolinMessageBridge};
// pangolin --->

pub use common_primitives::{self as pangoro_primitives, self as pangolin_primitives};

pub use pangolin_constants::*;

pub use darwinia_balances::Call as BalancesCall;
pub use frame_system::Call as SystemCall;
pub use pallet_bridge_grandpa::Call as BridgeGrandpaCall;
pub use pallet_bridge_messages::Call as BridgeMessagesCall;
pub use pallet_sudo::Call as SudoCall;

// --- crates.io ---
use codec::{Decode, Encode};
// --- substrate ---
use bp_runtime::ChainId;
use bridge_runtime_common::messages::{
	source::{estimate_message_dispatch_and_delivery_fee, FromThisChainMessagePayload},
	MessageBridge,
};
use frame_support::{
	construct_runtime, parameter_types,
	traits::KeyOwnerProofSystem,
	weights::{IdentityFee, PostDispatchInfo, Weight},
	PalletId,
};
use frame_system::RawOrigin;
use pallet_bridge_messages::Instance1 as Pangolin;
use pallet_grandpa::{
	fg_primitives, AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList,
};
use pallet_transaction_payment::{FeeDetails, RuntimeDispatchInfo};
use sp_api::impl_runtime_apis;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata, H160};
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{Block as BlockT, Dispatchable, NumberFor, OpaqueKeys},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, DispatchErrorWithPostInfo, MultiAddress, MultiSignature, MultiSigner,
};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;
// --- darwinia ---
use bridge_primitives::{PANGOLIN_CHAIN_ID, PANGORO_CHAIN_ID};
use common_primitives::*;
use darwinia_s2s_backing::EncodeCall;
use darwinia_support::s2s::{to_bytes32, RelayMessageCaller};
use dp_asset::{token::Token, RecipientAccount};

pub type Address = MultiAddress<AccountId, ()>;
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
pub type SignedBlock = generic::SignedBlock<Block>;
pub type SignedExtra = (
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPallets,
>;
pub type SignedPayload = generic::SignedPayload<Call, SignedExtra>;

pub type Ring = Balances;

pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("pangoro-runtime"),
	impl_name: create_runtime_str!("pangoro-runtime"),
	authoring_version: 1,
	spec_version: 1,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
};

#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion {
		runtime_version: VERSION,
		can_author_with: Default::default(),
	}
}

parameter_types! {
	pub const TransactionBaseFee: Balance = 0;
	pub const TransactionByteFee: Balance = 1;
}
impl pallet_transaction_payment::Config for Runtime {
	type OnChargeTransaction = pallet_transaction_payment::CurrencyAdapter<Balances, ()>;
	type TransactionByteFee = TransactionByteFee;
	type WeightToFee = IdentityFee<Balance>;
	type FeeMultiplierUpdate = ();
}

impl_opaque_keys! {
	pub struct SessionKeys {
		pub aura: Aura,
		pub grandpa: Grandpa,
	}
}
parameter_types! {
	pub const Period: BlockNumber = pangoro_constants::SESSION_LENGTH as _;
	pub const Offset: BlockNumber = 0;
}
impl pallet_session::Config for Runtime {
	type Event = Event;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type ValidatorIdOf = ();
	type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
	type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
	type SessionManager = pallet_shift_session_manager::Pallet<Runtime>;
	type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
	type DisabledValidatorsThreshold = ();
	type WeightInfo = ();
}

impl pallet_grandpa::Config for Runtime {
	type Event = Event;
	type Call = Call;
	type KeyOwnerProofSystem = ();
	type KeyOwnerProof =
		<Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;
	type KeyOwnerIdentification = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
		KeyTypeId,
		GrandpaId,
	)>>::IdentificationTuple;
	type HandleEquivocation = ();
	type WeightInfo = ();
}

impl pallet_sudo::Config for Runtime {
	type Event = Event;
	type Call = Call;
}

// <--- pangolin
parameter_types! {
	pub const MaxMessagesToPruneAtOnce: bp_messages::MessageNonce = 8;
	pub const MaxUnrewardedRelayerEntriesAtInboundLane: bp_messages::MessageNonce =
		bridge_primitives::MAX_UNREWARDED_RELAYER_ENTRIES_AT_INBOUND_LANE;
	pub const MaxUnconfirmedMessagesAtInboundLane: bp_messages::MessageNonce =
		bridge_primitives::MAX_UNCONFIRMED_MESSAGES_AT_INBOUND_LANE;
	// `IdentityFee` is used by Pangoro => we may use weight directly
	pub const GetDeliveryConfirmationTransactionFee: Balance =
		bridge_primitives::MAX_SINGLE_MESSAGE_DELIVERY_CONFIRMATION_TX_WEIGHT as _;
	pub RootAccountForPayments: Option<AccountId> = Some(to_bytes32(b"root").into());
}
pub type WithPangolinMessages = pallet_bridge_messages::Instance1;
impl pallet_bridge_messages::Config<WithPangolinMessages> for Runtime {
	type Event = Event;
	// FIXME
	type WeightInfo = pallet_bridge_messages::weights::RialtoWeight<Runtime>;
	type Parameter = pangolin_messages::PangoroToPangolinMessagesParameter;
	type MaxMessagesToPruneAtOnce = MaxMessagesToPruneAtOnce;
	type MaxUnrewardedRelayerEntriesAtInboundLane = MaxUnrewardedRelayerEntriesAtInboundLane;
	type MaxUnconfirmedMessagesAtInboundLane = MaxUnconfirmedMessagesAtInboundLane;

	type OutboundPayload = pangolin_messages::ToPangolinMessagePayload;
	type OutboundMessageFee = Balance;

	type InboundPayload = pangolin_messages::FromPangolinMessagePayload;
	type InboundMessageFee = pangolin_primitives::Balance;
	type InboundRelayer = pangolin_primitives::AccountId;

	type AccountIdConverter = bridge_primitives::AccountIdConverter;

	type TargetHeaderChain = pangolin_messages::Pangolin;
	type LaneMessageVerifier = pangolin_messages::ToPangolinMessageVerifier;
	type MessageDeliveryAndDispatchPayment =
		pallet_bridge_messages::instant_payments::InstantCurrencyPayments<
			Runtime,
			darwinia_balances::Pallet<Runtime, RingInstance>,
			GetDeliveryConfirmationTransactionFee,
			RootAccountForPayments,
		>;

	type SourceHeaderChain = pangolin_messages::Pangolin;
	type MessageDispatch = pangolin_messages::FromPangolinMessageDispatch;
}

pub type WithPangolinDispatch = pallet_bridge_dispatch::Instance1;
impl pallet_bridge_dispatch::Config<WithPangolinDispatch> for Runtime {
	type Event = Event;
	type MessageId = (bp_messages::LaneId, bp_messages::MessageNonce);
	type Call = Call;
	type CallFilter = ();
	type EncodedCall = pangolin_messages::FromPangolinEncodedCall;
	type SourceChainAccountId = pangolin_primitives::AccountId;
	type TargetChainAccountPublic = MultiSigner;
	type TargetChainSignature = MultiSignature;
	type AccountIdConverter = bridge_primitives::AccountIdConverter;
}

parameter_types! {
	// This is a pretty unscientific cap.
	//
	// Note that once this is hit the pallet will essentially throttle incoming requests down to one
	// call per block.
	pub const MaxRequests: u32 = 50;
	// Number of headers to keep.
	//
	// Assuming the worst case of every header being finalized, we will keep headers for at least a
	// week.
	pub const HeadersToKeep: u32 = 7 * pangoro_constants::DAYS as u32;
}
pub type WithPangolinGrandpa = pallet_bridge_grandpa::Instance1;
impl pallet_bridge_grandpa::Config<WithPangolinGrandpa> for Runtime {
	type BridgedChain = bridge_primitives::Pangolin;
	type MaxRequests = MaxRequests;
	type HeadersToKeep = HeadersToKeep;
	// FIXME
	type WeightInfo = pallet_bridge_grandpa::weights::RialtoWeight<Runtime>;
}
// pangolin --->

impl pallet_shift_session_manager::Config for Runtime {}

// <--- s2s backing ---
/// Bridged chain pangolin call info
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub enum PangolinRuntime {
	/// Note: this index must be the same as the backing pallet in pangolin chain runtime
	#[codec(index = 49)]
	Sub2SubIssuing(PangolinSub2SubIssuingCall),
}

/// Something important to note:
/// The index below represent the call order in the pangolin issuing pallet call.
/// For example, `index = 1` point to the `register_from_remote` (second)call in pangolin runtime.
/// You must update the index here if you change the call order in Pangolin runtime.
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[allow(non_camel_case_types)]
pub enum PangolinSub2SubIssuingCall {
	#[codec(index = 1)]
	register_from_remote(Token),
	#[codec(index = 2)]
	issue_from_remote(Token, H160),
}

pub struct PangolinCallEncoder;
impl EncodeCall<AccountId, ToPangolinMessagePayload> for PangolinCallEncoder {
	/// Encode issuing pallet remote_register call
	fn encode_remote_register(
		spec_version: u32,
		weight: u64,
		token: Token,
	) -> ToPangolinMessagePayload {
		let call = PangolinRuntime::Sub2SubIssuing(
			PangolinSub2SubIssuingCall::register_from_remote(token),
		)
		.encode();
		Self::to_payload(spec_version, weight, call)
	}
	/// Encode issuing pallet remote_issue call
	fn encode_remote_issue(
		spec_version: u32,
		weight: u64,
		token: Token,
		recipient: RecipientAccount<AccountId>,
	) -> Result<ToPangolinMessagePayload, ()> {
		let call = match recipient {
			RecipientAccount::<AccountId>::EthereumAccount(r) => PangolinRuntime::Sub2SubIssuing(
				PangolinSub2SubIssuingCall::issue_from_remote(token, r),
			)
			.encode(),
			_ => return Err(()),
		};
		Ok(Self::to_payload(spec_version, weight, call))
	}
}

impl PangolinCallEncoder {
	/// Transfer call to message payload
	fn to_payload(spec_version: u32, weight: u64, call: Vec<u8>) -> ToPangolinMessagePayload {
		return FromThisChainMessagePayload::<WithPangolinMessageBridge> {
			spec_version,
			weight,
			origin: bp_message_dispatch::CallOrigin::SourceRoot,
			call,
		};
	}
}

pub const PANGORO_PANGOLIN_LANE: [u8; 4] = *b"mtpl";

pub struct ToPangolinMessageRelayCaller;
impl RelayMessageCaller<ToPangolinMessagePayload, Balance> for ToPangolinMessageRelayCaller {
	fn send_message(
		payload: ToPangolinMessagePayload,
		fee: Balance,
	) -> Result<PostDispatchInfo, DispatchErrorWithPostInfo<PostDispatchInfo>> {
		let call: Call = BridgeMessagesCall::<Runtime, Pangolin>::send_message(
			PANGORO_PANGOLIN_LANE,
			payload,
			fee,
		)
		.into();
		call.dispatch(RawOrigin::Root.into())
	}
}

parameter_types! {
	pub const PangolinChainId: ChainId = PANGOLIN_CHAIN_ID;
	pub const S2sBackingPalletId: PalletId = PalletId(*b"da/s2sba");
	pub const RingLockLimit: Balance = 10_000_000 * 1_000_000_000;
}

impl darwinia_s2s_backing::Config for Runtime {
	type PalletId = S2sBackingPalletId;
	type Event = Event;
	type WeightInfo = ();
	type RingLockMaxLimit = RingLockLimit;
	type RingCurrency = Ring;

	type BridgedAccountIdConverter = bridge_primitives::AccountIdConverter;
	type BridgedChainId = PangolinChainId;

	type OutboundPayload = ToPangolinMessagePayload;
	type CallEncoder = PangolinCallEncoder;

	type FeeAccount = RootAccountForPayments;
	type MessageSender = ToPangolinMessageRelayCaller;
}
// --- s2s backing --->

construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = OpaqueBlock,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>} = 0,
		RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Pallet, Call, Storage} = 1,

		Aura: pallet_aura::{Pallet, Config<T>} = 2,
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent} = 3,
		Balances: darwinia_balances::<Instance1>::{Pallet, Call, Storage, Config<T>, Event<T>} = 4,
		Kton: darwinia_balances::<Instance2>::{Pallet, Call, Storage, Config<T>, Event<T>} = 5,
		TransactionPayment: pallet_transaction_payment::{Pallet, Storage} = 6,

		Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>} = 7,
		Grandpa: pallet_grandpa::{Pallet, Call, Storage, Config, Event} = 8,

		Sudo: pallet_sudo::{Pallet, Call, Config<T>, Storage, Event<T>} = 9,

		// <--- pangolin
		BridgePangolinMessages: pallet_bridge_messages::<Instance1>::{Pallet, Call, Storage, Event<T>} = 10,
		BridgePangolinDispatch: pallet_bridge_dispatch::<Instance1>::{Pallet, Event<T>} = 11,
		BridgePangolinGrandpa: pallet_bridge_grandpa::<Instance1>::{Pallet, Call, Storage} = 12,
		// pangolin --->
		ShiftSessionManager: pallet_shift_session_manager::{Pallet} = 13,

		Substrate2SubstrateBacking: darwinia_s2s_backing::{Pallet, Call, Storage, Event<T>} = 14,
	}
);

impl_runtime_apis! {
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block);
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			Runtime::metadata().into()
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: Block,
			data: sp_inherents::InherentData,
		) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		fn account_nonce(account: AccountId) -> Nonce {
			System::account_nonce(account)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> sp_consensus_aura::SlotDuration {
			sp_consensus_aura::SlotDuration::from_millis(Aura::slot_duration())
		}

		fn authorities() -> Vec<AuraId> {
			Aura::authorities()
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<
		Block,
		Balance,
	> for Runtime {
		fn query_info(uxt: <Block as BlockT>::Extrinsic, len: u32) -> RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(uxt: <Block as BlockT>::Extrinsic, len: u32) -> FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, sp_core::crypto::KeyTypeId)>> {
			SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl fg_primitives::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> GrandpaAuthorityList {
			Grandpa::grandpa_authorities()
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			equivocation_proof: fg_primitives::EquivocationProof<
				<Block as BlockT>::Hash,
				NumberFor<Block>,
			>,
			key_owner_proof: fg_primitives::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			let key_owner_proof = key_owner_proof.decode()?;

			Grandpa::submit_unsigned_equivocation_report(
				equivocation_proof,
				key_owner_proof,
			)
		}

		fn generate_key_ownership_proof(
			_set_id: fg_primitives::SetId,
			_authority_id: GrandpaId,
		) -> Option<fg_primitives::OpaqueKeyOwnershipProof> {
			// NOTE: this is the only implementation possible since we've
			// defined our key owner proof type as a bottom type (i.e. a type
			// with no values).
			None
		}
	}

	// <--- pangolin
	impl bridge_primitives::PangolinFinalityApi<Block> for Runtime {
		fn best_finalized() -> (pangolin_primitives::BlockNumber, pangolin_primitives::Hash) {
			let header = BridgePangolinGrandpa::best_finalized();
			(header.number, header.hash())
		}

		fn is_known_header(hash: pangolin_primitives::Hash) -> bool {
			BridgePangolinGrandpa::is_known_header(hash)
		}
	}

	impl bridge_primitives::ToPangolinOutboundLaneApi<Block, Balance, ToPangolinMessagePayload> for Runtime {
		fn estimate_message_delivery_and_dispatch_fee(
			_lane_id: bp_messages::LaneId,
			payload: ToPangolinMessagePayload,
		) -> Option<Balance> {
			estimate_message_dispatch_and_delivery_fee::<WithPangolinMessageBridge>(
				&payload,
				WithPangolinMessageBridge::RELAYER_FEE_PERCENT,
			).ok()
		}

		fn messages_dispatch_weight(
			lane: bp_messages::LaneId,
			begin: bp_messages::MessageNonce,
			end: bp_messages::MessageNonce,
		) -> Vec<(bp_messages::MessageNonce, Weight, u32)> {
			(begin..=end).filter_map(|nonce| {
				let encoded_payload = BridgePangolinMessages::outbound_message_payload(lane, nonce)?;
				let decoded_payload = pangolin_messages::ToPangolinMessagePayload::decode(
					&mut &encoded_payload[..]
				).ok()?;
				Some((nonce, decoded_payload.weight, encoded_payload.len() as _))
			})
			.collect()
		}

		fn latest_received_nonce(lane: bp_messages::LaneId) -> bp_messages::MessageNonce {
			BridgePangolinMessages::outbound_latest_received_nonce(lane)
		}

		fn latest_generated_nonce(lane: bp_messages::LaneId) -> bp_messages::MessageNonce {
			BridgePangolinMessages::outbound_latest_generated_nonce(lane)
		}
	}

	impl bridge_primitives::FromPangolinInboundLaneApi<Block> for Runtime {
		fn latest_received_nonce(lane: bp_messages::LaneId) -> bp_messages::MessageNonce {
			BridgePangolinMessages::inbound_latest_received_nonce(lane)
		}

		fn latest_confirmed_nonce(lane: bp_messages::LaneId) -> bp_messages::MessageNonce {
			BridgePangolinMessages::inbound_latest_confirmed_nonce(lane)
		}

		fn unrewarded_relayers_state(lane: bp_messages::LaneId) -> bp_messages::UnrewardedRelayersState {
			BridgePangolinMessages::inbound_unrewarded_relayers_state(lane)
		}
	}
	// pangolin --->
}

// <--- pangolin
/// Pangolin account ownership digest from Pangoro.
///
/// The byte vector returned by this function should be signed with a Pangolin account private key.
/// This way, the owner of `pangoro_account_id` on Pangoro proves that the Pangolin account private key
/// is also under his control.
pub fn pangoro_to_pangolin_account_ownership_digest<Call, AccountId, SpecVersion>(
	pangolin_call: &Call,
	pangoro_account_id: AccountId,
	pangolin_spec_version: SpecVersion,
) -> sp_std::vec::Vec<u8>
where
	Call: Encode,
	AccountId: Encode,
	SpecVersion: Encode,
{
	pallet_bridge_dispatch::account_ownership_digest(
		pangolin_call,
		pangoro_account_id,
		pangolin_spec_version,
		PANGORO_CHAIN_ID,
		PANGOLIN_CHAIN_ID,
	)
}
// pangolin --->
