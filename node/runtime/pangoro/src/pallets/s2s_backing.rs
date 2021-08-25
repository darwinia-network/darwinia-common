// --- paritytech ---
use bp_message_dispatch::CallOrigin;
// --- darwinia-network ---
use crate::*;
use bridge_primitives::AccountIdConverter;
use darwinia_s2s_backing::Config;

pub const PANGORO_PANGOLIN_LANE: [u8; 4] = *b"mtpl";

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
impl PangolinCallEncoder {
	/// Transfer call to message payload
	fn to_payload(spec_version: u32, weight: u64, call: Vec<u8>) -> ToPangolinMessagePayload {
		return FromThisChainMessagePayload::<WithPangolinMessageBridge> {
			spec_version,
			weight,
			origin: CallOrigin::SourceRoot,
			call,
		};
	}
}
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

impl Config for Runtime {
	type PalletId = S2sBackingPalletId;
	type Event = Event;
	type WeightInfo = ();
	type RingLockMaxLimit = RingLockLimit;
	type RingCurrency = Ring;

	type BridgedAccountIdConverter = AccountIdConverter;
	type BridgedChainId = PangolinChainId;

	type OutboundPayload = ToPangolinMessagePayload;
	type CallEncoder = PangolinCallEncoder;

	type FeeAccount = RootAccountForPayments;
	type MessageSender = ToPangolinMessageRelayCaller;
}
