pub use pallet_bridge_dispatch::Instance1 as WithPangolinDispatch;

// --- paritytech ---
use frame_support::{
	ensure,
	traits::{OriginTrait, WithdrawReasons},
};
use sp_runtime::transaction_validity::{InvalidTransaction, TransactionValidityError};
// --- darwinia-network ---
use crate::*;
use bp_message_dispatch::{CallValidate, IntoDispatchOrigin as IntoDispatchOriginT};
use bp_messages::{LaneId, MessageNonce};
use darwinia_ethereum::{RawOrigin, Transaction};
use darwinia_evm::CurrencyAdapt;
use darwinia_support::evm::{DeriveEthereumAddress, DeriveSubstrateAddress};
use pallet_bridge_dispatch::Config;

pub struct CallValidator;
impl CallValidate<bp_darwinia_core::AccountId, Origin, Call> for CallValidator {
	fn check_receiving_before_dispatch(
		relayer_account: &bp_darwinia_core::AccountId,
		call: &Call,
	) -> Result<(), &'static str> {
		match call {
			Call::Ethereum(darwinia_ethereum::Call::message_transact {
				transaction: Transaction::Legacy(t),
			}) => {
				ensure!(t.value.is_zero(), "Only non-payable transaction supported.");
				ensure!(
					t.gas_limit <= <Runtime as darwinia_evm::Config>::BlockGasLimit::get(),
					"Tx gas limit over block limit"
				);

				// Use fixed gas price now.
				let gas_price = <Runtime as darwinia_evm::Config>::FeeCalculator::min_gas_price();
				let fee = t.gas_limit.saturating_mul(gas_price);

				// Ensure the relayer's account has enough balance to withdraw. If not,
				// reject the call before dispatch.
				Ok(<Runtime as darwinia_evm::Config>::RingBalanceAdapter::ensure_can_withdraw(
					relayer_account,
					fee,
					WithdrawReasons::all(),
				)
				.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Payment))?)
			},
			_ => Ok(()),
		}
	}

	fn call_validate(
		relayer_account: &bp_darwinia_core::AccountId,
		origin: &Origin,
		call: &Call,
	) -> Result<(), TransactionValidityError> {
		match call {
			// Note: Only support Ethereum::message_transact(LegacyTransaction)
			Call::Ethereum(darwinia_ethereum::Call::message_transact { transaction: tx }) => {
				match origin.caller() {
					OriginCaller::Ethereum(RawOrigin::EthereumTransaction(id)) => match tx {
						Transaction::Legacy(t) => {
							let gas_price =
								<Runtime as darwinia_evm::Config>::FeeCalculator::min_gas_price();
							let fee = t.gas_limit.saturating_mul(gas_price);
							let derived_substrate_address =
								<Runtime as darwinia_evm::Config>::IntoAccountId::derive_substrate_address(id);

							// The balance validation already has been done in the
							// `check_receiving_before_dispatch`.
							<Runtime as darwinia_evm::Config>::RingBalanceAdapter::evm_transfer(
								relayer_account,
								&derived_substrate_address,
								fee,
							)
							.map_err(|_| {
								TransactionValidityError::Invalid(InvalidTransaction::Custom(3))
							})
						},
						_ => Err(TransactionValidityError::Invalid(InvalidTransaction::Custom(1))),
					},
					_ => Err(TransactionValidityError::Invalid(InvalidTransaction::Custom(0))),
				}
			},
			_ => Ok(()),
		}
	}
}

pub struct IntoDispatchOrigin;
impl IntoDispatchOriginT<bp_darwinia_core::AccountId, Call, Origin> for IntoDispatchOrigin {
	fn into_dispatch_origin(id: &bp_darwinia_core::AccountId, call: &Call) -> Origin {
		match call {
			Call::Ethereum(darwinia_ethereum::Call::message_transact { .. }) => {
				let derive_eth_address = id.derive_ethereum_address();

				darwinia_ethereum::RawOrigin::EthereumTransaction(derive_eth_address).into()
			},
			_ => frame_system::RawOrigin::Signed(id.clone()).into(),
		}
	}
}

impl Config<WithPangolinDispatch> for Runtime {
	type AccountIdConverter = bp_darwinia_core::AccountIdConverter;
	type BridgeMessageId = (LaneId, MessageNonce);
	type Call = Call;
	type CallValidator = CallValidator;
	type EncodedCall = bm_pangolin::FromPangolinEncodedCall;
	type Event = Event;
	type IntoDispatchOrigin = IntoDispatchOrigin;
	type SourceChainAccountId = bp_darwinia_core::AccountId;
	type TargetChainAccountPublic = bp_darwinia_core::AccountPublic;
	type TargetChainSignature = bp_darwinia_core::Signature;
}
