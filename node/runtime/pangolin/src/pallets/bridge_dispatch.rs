pub use pallet_bridge_dispatch::{
	Instance1 as WithPangoroDispatch, Instance2 as WithPangolinParachainDispatch,
};

// --- paritytech ---
use frame_support::{traits::Everything, weights::PostDispatchInfo};
// --- darwinia-network ---
use crate::*;
use bp_messages::{LaneId, MessageNonce};
use core::str::FromStr;
use pallet_bridge_dispatch::{Config, EthereumTransactCall};

pub struct EthereumCallDispatcher;

impl EthereumTransactCall<Call> for EthereumCallDispatcher {
	fn is_ethereum_call(t: &Call) -> bool {
		true
	}

	fn validate(t: &Call) -> bool {
		true
	}

	fn dispatch(c: &Call) -> Option<sp_runtime::DispatchResultWithInfo<PostDispatchInfo>> {
		match c {
			call @ Call::Ethereum(darwinia_ethereum::Call::transact { transaction: tx }) => {
				let origin = H160::from_str("1000000000000000000000000000000000000001").unwrap();
				Ethereum::validate_transaction_in_block(origin, tx);

				Some(call.clone().dispatch(
					// the 160 should passed from the dispatch
					Origin::from(darwinia_ethereum::RawOrigin::EthereumTransaction(origin)),
				))
			},
			_ => None,
		}
	}
}

impl Config<WithPangoroDispatch> for Runtime {
	type AccountIdConverter = bp_pangolin::AccountIdConverter;
	type BridgeMessageId = (LaneId, MessageNonce);
	type Call = Call;
	type CallFilter = Everything;
	type EncodedCall = bm_pangoro::FromPangoroEncodedCall;
	type EthereumTransactValidator = EthereumCallDispatcher;
	type Event = Event;
	type SourceChainAccountId = bp_pangoro::AccountId;
	type TargetChainAccountPublic = bp_pangolin::AccountPublic;
	type TargetChainSignature = bp_pangolin::Signature;
}
impl Config<WithPangolinParachainDispatch> for Runtime {
	type AccountIdConverter = bp_pangolin::AccountIdConverter;
	type BridgeMessageId = (LaneId, MessageNonce);
	type Call = Call;
	type CallFilter = Everything;
	type EncodedCall = bm_pangolin_parachain::FromPangolinParachainEncodedCall;
	type EthereumTransactValidator = EthereumCallDispatcher;
	type Event = Event;
	type SourceChainAccountId = bp_pangolin_parachain::AccountId;
	type TargetChainAccountPublic = bp_pangolin::AccountPublic;
	type TargetChainSignature = bp_pangolin::Signature;
}
