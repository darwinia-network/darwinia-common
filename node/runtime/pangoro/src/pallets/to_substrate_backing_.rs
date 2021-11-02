pub use pallet_bridge_messages::Instance1 as Pangolin;

// --- paritytech ---
use bp_messages::LaneId;
use bp_runtime::{messages::DispatchFeePayment, ChainId};
use bridge_runtime_common::messages::source::FromThisChainMessagePayload;
use frame_support::PalletId;
use pangoro_primitives::AccountId;
use sp_core::{H160, U256};
// --- darwinia-network ---
use crate::*;
use bridge_primitives::{AccountIdConverter, PANGORO_PANGOLIN_LANE};
use darwinia_support::s2s::LatestMessageNoncer;
use dp_asset::{token::TokenMetadata, RecipientAccount};
use to_substrate_backing::{Config, EncodeCall};

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
	#[codec(index = 0)]
	register_from_remote(TokenMetadata),
	#[codec(index = 1)]
	issue_from_remote(H160, U256, H160),
}

pub struct PangolinCallEncoder;
impl PangolinCallEncoder {
	/// Transfer call to message payload
	fn to_payload(
		submitter: AccountId,
		spec_version: u32,
		weight: u64,
		call: Vec<u8>,
	) -> ToPangolinMessagePayload {
		return FromThisChainMessagePayload::<WithPangolinMessageBridge> {
			spec_version,
			weight,
			origin: bp_message_dispatch::CallOrigin::SourceAccount(submitter),
			call,
			dispatch_fee_payment: DispatchFeePayment::AtSourceChain,
		};
	}
}
impl EncodeCall<AccountId, ToPangolinMessagePayload> for PangolinCallEncoder {
	/// Encode issuing pallet remote_register call
	fn encode_remote_register(
		submitter: AccountId,
		spec_version: u32,
		weight: u64,
		token_metadata: TokenMetadata,
	) -> ToPangolinMessagePayload {
		let call = PangolinRuntime::Sub2SubIssuing(
			PangolinSub2SubIssuingCall::register_from_remote(token_metadata),
		)
		.encode();
		Self::to_payload(submitter, spec_version, weight, call)
	}
	/// Encode issuing pallet remote_issue call
	fn encode_remote_issue(
		submitter: AccountId,
		spec_version: u32,
		weight: u64,
		token_address: H160,
		amount: U256,
		recipient: RecipientAccount<AccountId>,
	) -> Result<ToPangolinMessagePayload, ()> {
		let call = match recipient {
			RecipientAccount::<AccountId>::EthereumAccount(r) => PangolinRuntime::Sub2SubIssuing(
				PangolinSub2SubIssuingCall::issue_from_remote(token_address, amount, r),
			)
			.encode(),
			_ => return Err(()),
		};
		Ok(Self::to_payload(submitter, spec_version, weight, call))
	}
}

pub struct PangolinMessageNoncer;
impl LatestMessageNoncer for PangolinMessageNoncer {
	fn outbound_latest_generated_nonce(lane_id: LaneId) -> u64 {
		BridgePangolinMessages::outbound_latest_generated_nonce(lane_id).into()
	}

	fn inbound_latest_received_nonce(lane_id: LaneId) -> u64 {
		BridgePangolinMessages::inbound_latest_received_nonce(lane_id).into()
	}
}

frame_support::parameter_types! {
	pub const PangolinChainId: ChainId = PANGOLIN_CHAIN_ID;
	pub const RingPalletId: PalletId = PalletId(*b"da/bring");
	pub const S2sBackingPalletId: PalletId = PalletId(*b"da/s2sba");
	pub const MaxLockRingAmountPerTx: Balance = 10_000 * COIN;
	pub const BridgePangolinLaneId: LaneId = PANGORO_PANGOLIN_LANE;
}

impl Config for Runtime {
	type Event = Event;
	type WeightInfo = ();

	type PalletId = S2sBackingPalletId;

	type RingPalletId = RingPalletId;
	type MaxLockRingAmountPerTx = MaxLockRingAmountPerTx;
	type RingCurrency = Ring;

	type BridgedAccountIdConverter = AccountIdConverter;
	type BridgedChainId = PangolinChainId;

	type OutboundPayload = ToPangolinMessagePayload;
	type CallEncoder = PangolinCallEncoder;

	type MessageNoncer = PangolinMessageNoncer;

	type MessageLaneId = BridgePangolinLaneId;

	type OutboundMessageFee = Balance;
	type MessagesBridge = BridgePangolinMessages;
}
