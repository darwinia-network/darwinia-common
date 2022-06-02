pub use pallet_bridge_dispatch::{
	Instance1 as WithPangoroDispatch, Instance2 as WithPangolinParachainDispatch,
};

// --- paritytech ---
use frame_support::{traits::Everything, weights::PostDispatchInfo};
use sp_runtime::transaction_validity::TransactionValidityError;
// --- darwinia-network ---
use crate::*;
use bp_messages::{LaneId, MessageNonce};
use darwinia_support::evm::DeriveEthereumAddress;
use pallet_bridge_dispatch::{Config, EthereumCallDispatch};

pub struct EthereumCallDispatcher;
impl EthereumCallDispatch for EthereumCallDispatcher {
	type AccountId = bp_pangolin::AccountId;
	type Call = Call;

	fn dispatch(
		c: &Call,
		origin: &bp_pangolin::AccountId,
	) -> Result<
		Option<sp_runtime::DispatchResultWithInfo<PostDispatchInfo>>,
		TransactionValidityError,
	> {
		match c {
			call @ Call::Ethereum(darwinia_ethereum::Call::transact { transaction: tx }) => {
				let derive_eth_address = origin.derive_ethereum_address();
				if let Err(validate_err) =
					Ethereum::validate_transaction_in_block(derive_eth_address, tx)
				{
					return Err(validate_err);
				}

				Ok(Some(call.clone().dispatch(Origin::from(
					darwinia_ethereum::RawOrigin::EthereumTransaction(derive_eth_address),
				))))
			},
			_ => Ok(None),
		}
	}
}

pub struct EmptyEthereumCallDispatcher;
impl EthereumCallDispatch for EmptyEthereumCallDispatcher {
	type AccountId = bp_pangolin_parachain::AccountId;
	type Call = Call;

	fn dispatch(
		_c: &Self::Call,
		_origin: &Self::AccountId,
	) -> Result<
		Option<sp_runtime::DispatchResultWithInfo<PostDispatchInfo>>,
		TransactionValidityError,
	> {
		Ok(None)
	}
}

impl Config<WithPangoroDispatch> for Runtime {
	type AccountIdConverter = bp_pangolin::AccountIdConverter;
	type BridgeMessageId = (LaneId, MessageNonce);
	type Call = Call;
	type CallFilter = Everything;
	type EncodedCall = bm_pangoro::FromPangoroEncodedCall;
	type EthereumCallDispatcher = EthereumCallDispatcher;
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
	type EthereumCallDispatcher = EmptyEthereumCallDispatcher;
	type Event = Event;
	type SourceChainAccountId = bp_pangolin_parachain::AccountId;
	type TargetChainAccountPublic = bp_pangolin::AccountPublic;
	type TargetChainSignature = bp_pangolin::Signature;
}
