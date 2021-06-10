// --- substrate ---
use frame_support::PalletId;
// --- darwinia ---
// use crate::substrate::millau_messages::{MillauCallToPayload, ToMillauMessagePayload};
use crate::*;
use bp_millau::AccountIdConverter;
use darwinia_s2s_relay::{Config, Instance1 as ToMillauRelay, TruncateToEthAddress};
use dp_asset::BridgedAssetReceiver;
use pallet_bridge_messages::Instance1 as Millau;

pub struct ToMillauMessageRelayCall;
impl MessageRelayCall<ToMillauMessagePayload, Call> for ToMillauMessageRelayCall {
	fn encode_call(payload: ToMillauMessagePayload) -> Call {
		return BridgeMessagesCall::<Runtime, Millau>::send_message([0; 4], payload, 0u128.into())
			.into();
	}
}

pub struct MillauBackingReceiver;
impl BridgedAssetReceiver<RelayAccount<AccountId>> for MillauBackingReceiver {
	fn encode_call(token: Token, recipient: RelayAccount<AccountId>) -> Result<Vec<u8>, ()> {
		match recipient {
			RelayAccount::<AccountId>::DarwiniaAccount(r) => {
				return Ok(MillauRuntime::Sub2SubBacking(
					MillauSub2SubBackingCall::cross_receive_and_unlock((token, r)),
				)
				.encode())
			}
			_ => Err(()),
		}
	}
}

frame_support::parameter_types! {
	pub const S2sRelayPalletId: PalletId = PalletId(*b"da/s2sre");
	pub const MillauChainId: bp_runtime::ChainId = bp_runtime::MILLAU_CHAIN_ID;
}

impl Config<ToMillauRelay> for Runtime {
	type PalletId = S2sRelayPalletId;
	type Event = Event;
	type WeightInfo = ();

	type BridgedChainId = MillauChainId;
	type OutboundPayload = ToMillauMessagePayload;
	type OutboundMessageFee = Balance;

	type CallToPayload = MillauCallToPayload;
	type BridgedAssetReceiverT = MillauBackingReceiver;
	type BridgedAccountIdConverter = AccountIdConverter;
	type ToEthAddressT = TruncateToEthAddress;
	type MessageRelayCallT = ToMillauMessageRelayCall;
}
