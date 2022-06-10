pub use pallet_bridge_dispatch::Instance1 as WithPangolinDispatch;

// --- paritytech ---
use frame_support::traits::OriginTrait;
// --- darwinia-network ---
use crate::*;
use bp_message_dispatch::{CallFilter as CallFilterT, IntoDispatchOrigin as IntoDispatchOriginT};
use bp_messages::{LaneId, MessageNonce};
use darwinia_ethereum::{RawOrigin, Transaction};
use darwinia_support::evm::DeriveEthereumAddress;
use pallet_bridge_dispatch::Config;

pub struct CallFilter;

impl CallFilterT<Origin, Call> for CallFilter {
	fn contains(origin: &Origin, call: &Call) -> bool {
		match call {
			// Note: Only supprt Ethereum::transact(LegacyTransaction)
			Call::Ethereum(darwinia_ethereum::Call::transact { transaction: tx }) => {
				match origin.caller() {
					OriginCaller::Ethereum(RawOrigin::EthereumTransaction(id)) => match tx {
						Transaction::Legacy(_) =>
							Ethereum::validate_transaction_in_block(*id, tx).is_err(),
						_ => false,
					},
					_ => false,
				}
			},
			_ => true,
		}
	}
}

pub struct IntoDispatchOrigin;

// TODO: check the account_id type
impl IntoDispatchOriginT<bp_pangoro::AccountId, Call, Origin> for IntoDispatchOrigin {
	fn into_dispatch_origin(id: &bp_pangoro::AccountId, call: &Call) -> Origin {
		match call {
			Call::Ethereum(darwinia_ethereum::Call::transact { .. }) => {
				let derive_eth_address = id.derive_ethereum_address();
				darwinia_ethereum::RawOrigin::EthereumTransaction(derive_eth_address).into()
			},
			_ => frame_system::RawOrigin::Signed(id.clone()).into(),
		}
	}
}

impl Config<WithPangolinDispatch> for Runtime {
	type AccountIdConverter = bp_pangoro::AccountIdConverter;
	type BridgeMessageId = (LaneId, MessageNonce);
	type Call = Call;
	type CallFilter = CallFilter;
	type EncodedCall = bm_pangolin::FromPangolinEncodedCall;
	type Event = Event;
	type IntoDispatchOrigin = IntoDispatchOrigin;
	type SourceChainAccountId = bp_pangolin::AccountId;
	type TargetChainAccountPublic = bp_pangoro::AccountPublic;
	type TargetChainSignature = bp_pangoro::Signature;
}
