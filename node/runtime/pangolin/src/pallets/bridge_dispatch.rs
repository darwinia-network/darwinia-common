pub use pallet_bridge_dispatch::{
	Instance1 as WithPangoroDispatch, Instance2 as WithPangolinParachainDispatch,
};

// --- paritytech ---
use frame_support::{
	ensure,
	traits::{Currency, OriginTrait, WithdrawReasons},
};
use sp_runtime::{
	traits::UniqueSaturatedInto,
	transaction_validity::{InvalidTransaction, TransactionValidityError},
};
// --- darwinia-network ---
use crate::*;
use bp_message_dispatch::{CallValidate, Everything, IntoDispatchOrigin as IntoDispatchOriginT};
use bp_messages::{LaneId, MessageNonce};
use darwinia_ethereum::{RawOrigin, Transaction};
use darwinia_evm::AccountBasic;
use darwinia_support::evm::{
	decimal_convert, DeriveEthereumAddress, DeriveSubstrateAddress, POW_9,
};
use pallet_bridge_dispatch::Config;

frame_support::parameter_types! {
	pub const MaxUsableBalanceFromRelayer: Balance = 100 * COIN;
}

pub struct CallValidator;
impl CallValidate<bp_pangolin::AccountId, Origin, Call> for CallValidator {
	fn check_receiving_before_dispatch(
		relayer_account: &bp_pangolin::AccountId,
		call: &Call,
	) -> Result<(), &'static str> {
		match call {
			Call::Ethereum(darwinia_ethereum::Call::message_transact { transaction: tx }) =>
				match tx {
					Transaction::Legacy(t) => {
						let gas_price =
							<Runtime as darwinia_evm::Config>::FeeCalculator::min_gas_price();
						let fee = t.gas_limit.saturating_mul(gas_price);

						// Ensure the relayer's account has enough balance to withdraw. If not,
						// reject the call.
						Ok(evm_ensure_can_withdraw(
							relayer_account,
							fee.min(decimal_convert(MaxUsableBalanceFromRelayer::get(), None)),
							WithdrawReasons::TRANSFER,
						)?)
					},
					_ => Ok(()),
				},
			_ => Ok(()),
		}
	}

	fn call_validate(
		relayer_account: &bp_pangolin::AccountId,
		origin: &Origin,
		call: &Call,
	) -> Result<(), TransactionValidityError> {
		match call {
			// Note: Only support Ethereum::message_transact(LegacyTransaction)
			Call::Ethereum(darwinia_ethereum::Call::message_transact { transaction: tx }) => {
				match origin.caller() {
					OriginCaller::Ethereum(RawOrigin::EthereumTransaction(id)) => match tx {
						Transaction::Legacy(t) => {
							// Only non-payable call supported.
							ensure!(
								t.value.is_zero(),
								TransactionValidityError::Invalid(InvalidTransaction::Payment,)
							);

							let gas_price =
								<Runtime as darwinia_evm::Config>::FeeCalculator::min_gas_price();
							let fee = t.gas_limit.saturating_mul(gas_price);

							// MaxUsableBalanceFromRelayer is the cap limitation for fee in case
							// gas_limit is too large for relayer
							ensure!(
								fee <= decimal_convert(MaxUsableBalanceFromRelayer::get(), None),
								TransactionValidityError::Invalid(InvalidTransaction::Custom(2))
							);

							// Already done `evm_ensure_can_withdraw` in
							// check_receiving_before_dispatch
							let derived_substrate_address =
								<Runtime as darwinia_evm::Config>::IntoAccountId::derive_substrate_address(*id);

							<Runtime as darwinia_evm::Config>::RingAccountBasic::evm_transfer(
								&relayer_account,
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
impl IntoDispatchOriginT<bp_pangoro::AccountId, Call, Origin> for IntoDispatchOrigin {
	fn into_dispatch_origin(id: &bp_pangoro::AccountId, call: &Call) -> Origin {
		match call {
			Call::Ethereum(darwinia_ethereum::Call::message_transact { .. }) => {
				let derive_eth_address = id.derive_ethereum_address();

				darwinia_ethereum::RawOrigin::EthereumTransaction(derive_eth_address).into()
			},
			_ => frame_system::RawOrigin::Signed(id.clone()).into(),
		}
	}
}

/// Ensure the account has enough balance to withdraw.
fn evm_ensure_can_withdraw(
	who: &bp_pangolin::AccountId,
	amount: U256,
	reasons: WithdrawReasons,
) -> Result<(), TransactionValidityError> {
	// Ensure the evm balance of the account large than the withdraw amount
	let old_evm_balance = <Runtime as darwinia_evm::Config>::RingAccountBasic::account_balance(who);
	let (_old_sub, old_remaining) = old_evm_balance.div_mod(U256::from(POW_9));
	ensure!(
		old_evm_balance > amount,
		TransactionValidityError::Invalid(InvalidTransaction::Payment)
	);

	// Because of precision difference, the amount also needs to convert.
	let (mut amount_sub, amount_remaining) = amount.div_mod(U256::from(POW_9));
	if old_remaining < amount_remaining {
		// Add 1, if the substrate balance part touched
		amount_sub = amount_sub.saturating_add(U256::from(1));
	}

	// Calculate the new substrate balance part
	let new_evm_balance = old_evm_balance.saturating_sub(amount);
	let (new_sub, _new_remaining) = new_evm_balance.div_mod(U256::from(POW_9));

	// Ensure the account underlying substrate account has no liquidity restrictions.
	Ring::ensure_can_withdraw(
		who,
		amount_sub.low_u128().unique_saturated_into(),
		reasons,
		new_sub.low_u128().unique_saturated_into(),
	)
	.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Payment))
}

impl Config<WithPangoroDispatch> for Runtime {
	type AccountIdConverter = bp_pangolin::AccountIdConverter;
	type BridgeMessageId = (LaneId, MessageNonce);
	type Call = Call;
	type CallValidator = CallValidator;
	type EncodedCall = bm_pangoro::FromPangoroEncodedCall;
	type Event = Event;
	type IntoDispatchOrigin = IntoDispatchOrigin;
	type SourceChainAccountId = bp_pangoro::AccountId;
	type TargetChainAccountPublic = bp_pangolin::AccountPublic;
	type TargetChainSignature = bp_pangolin::Signature;
}
impl Config<WithPangolinParachainDispatch> for Runtime {
	type AccountIdConverter = bp_pangolin::AccountIdConverter;
	type BridgeMessageId = (LaneId, MessageNonce);
	type Call = Call;
	type CallValidator = Everything;
	type EncodedCall = bm_pangolin_parachain::FromPangolinParachainEncodedCall;
	type Event = Event;
	type IntoDispatchOrigin = IntoDispatchOrigin;
	type SourceChainAccountId = bp_pangolin_parachain::AccountId;
	type TargetChainAccountPublic = bp_pangolin::AccountPublic;
	type TargetChainSignature = bp_pangolin::Signature;
}
