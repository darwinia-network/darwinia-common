pub use pallet_bridge_dispatch::Instance1 as WithPangolinDispatch;

use ethereum::LegacyTransaction;
// --- paritytech ---
use frame_support::traits::OriginTrait;
use sp_core::H160;
use sp_runtime::transaction_validity::{InvalidTransaction, TransactionValidityError};
// --- darwinia-network ---
use crate::*;
use bp_message_dispatch::{CallValidate, IntoDispatchOrigin as IntoDispatchOriginT};
use bp_messages::{LaneId, MessageNonce};
use darwinia_ethereum::{RawOrigin, Transaction};
use darwinia_evm::AccountBasic;
use darwinia_support::evm::{DeriveEthereumAddress, DeriveSubstrateAddress};
use pallet_bridge_dispatch::Config;

fn extract_tx_from_call(
	dispatch_origin: Option<&Origin>,
	call: &Call,
) -> Result<(Option<H160>, LegacyTransaction), bool> {
	let origin = if let Some(o) = dispatch_origin {
		match o.caller() {
			OriginCaller::Ethereum(RawOrigin::EthereumTransaction(eth_origin)) =>
				Some(eth_origin.clone()),
			_ => return Err(false),
		}
	} else {
		None
	};

	let tx = match call {
		Call::Ethereum(darwinia_ethereum::Call::transact { transaction: tx }) => match tx {
			Transaction::Legacy(t) => t.clone(),
			_ => return Err(false),
		},
		_ => return Err(false),
	};

	Ok((origin, tx))
}

pub struct CallValidator;
impl CallValidate<bp_pangoro::AccountId, Origin, Call> for CallValidator {
	fn check_receiving_before_dispatch(
		relayer_account: &bp_pangoro::AccountId,
		call: &Call,
	) -> Result<(), &'static str> {
		match extract_tx_from_call(None, call) {
			Ok((None, t)) => {
				let gas_price = <Runtime as darwinia_evm::Config>::FeeCalculator::min_gas_price();
				let fee = t.gas_limit.saturating_mul(gas_price);
				// TODO: check tx fee can not too high

				// TODO: check relayer balance usable balance is enough
				Ok(())
			},
			_ => Err("Error"),
		}
	}

	fn call_validate(
		relayer_account: &bp_pangoro::AccountId,
		origin: &Origin,
		call: &Call,
	) -> Result<(), TransactionValidityError> {
		match extract_tx_from_call(Some(origin), call) {
			Ok((Some(eth_origin), t)) => {
				// Only non-payable call supported.
				if t.value != U256::zero() {
					return Err(TransactionValidityError::Invalid(InvalidTransaction::Payment));
				}

				let gas_price = <Runtime as darwinia_evm::Config>::FeeCalculator::min_gas_price();
				let fee = t.gas_limit.saturating_mul(gas_price);
				let derived_substrate_address =
					<Runtime as darwinia_evm::Config>::IntoAccountId::derive_substrate_address(
						eth_origin,
					);
				if <Runtime as darwinia_evm::Config>::RingAccountBasic::account_balance(
					relayer_account,
				) >= fee
				{
					// Ensure the derived ethereum address has enough balance to pay for the
					// transaction
					let _ = <Runtime as darwinia_evm::Config>::RingAccountBasic::transfer(
						&relayer_account,
						&derived_substrate_address,
						fee,
					);
				}

				Ok(())
			},
			_ => Err(TransactionValidityError::Invalid(InvalidTransaction::Custom(0u8))),
		}
	}
}

pub struct IntoDispatchOrigin;
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
	type CallValidator = CallValidator;
	type EncodedCall = bm_pangolin::FromPangolinEncodedCall;
	type Event = Event;
	type IntoDispatchOrigin = IntoDispatchOrigin;
	type SourceChainAccountId = bp_pangolin::AccountId;
	type TargetChainAccountPublic = bp_pangoro::AccountPublic;
	type TargetChainSignature = bp_pangoro::Signature;
}
