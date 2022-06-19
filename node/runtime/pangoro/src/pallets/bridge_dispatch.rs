pub use pallet_bridge_dispatch::Instance1 as WithPangolinDispatch;

// --- paritytech ---
use frame_support::{
	ensure,
	traits::{Currency, OriginTrait, WithdrawReasons},
};
use sp_runtime::{
	traits::UniqueSaturatedInto,
	transaction_validity::{InvalidTransaction, TransactionValidityError},
};
use sp_std::cmp;
// --- darwinia-network ---
use crate::*;
use bp_message_dispatch::{CallValidate, IntoDispatchOrigin as IntoDispatchOriginT};
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

fn evm_ensure_can_withdraw(
	who: &bp_pangoro::AccountId,
	amount: U256,
	reasons: WithdrawReasons,
) -> Result<(), TransactionValidityError> {
	// Ensure the account's evm account has enough balance to withdraw.
	let old_evm_balance = <Runtime as darwinia_evm::Config>::RingAccountBasic::account_balance(who);
	let (_old_sub, old_remaining) = old_evm_balance.div_mod(U256::from(POW_9));
	ensure!(
		old_evm_balance > amount,
		TransactionValidityError::Invalid(InvalidTransaction::Payment)
	);

	let (mut amount_sub, amount_remaining) = amount.div_mod(U256::from(POW_9));
	if old_remaining < amount_remaining {
		amount_sub = amount_sub.saturating_add(U256::from(1));
	}

	let new_evm_balance = old_evm_balance.saturating_sub(amount);
	let (new_sub, _new_remaining) = new_evm_balance.div_mod(U256::from(POW_9));

	// Ensure the account underlying substrate account has no liquidity restrictions.
	ensure!(
		Ring::ensure_can_withdraw(
			who,
			amount_sub.low_u128().unique_saturated_into(),
			reasons,
			new_sub.low_u128().unique_saturated_into(),
		)
		.is_ok(),
		TransactionValidityError::Invalid(InvalidTransaction::Payment)
	);

	Ok(())
}

pub struct CallValidator;
impl CallValidate<bp_pangoro::AccountId, Origin, Call> for CallValidator {
	fn check_receiving_before_dispatch(
		relayer_account: &bp_pangoro::AccountId,
		call: &Call,
	) -> Result<(), &'static str> {
		match call {
			Call::Ethereum(darwinia_ethereum::Call::message_transact { transaction: tx }) =>
				match tx {
					Transaction::Legacy(t) => {
						// Use fixed gas price now.
						let gas_price =
							<Runtime as darwinia_evm::Config>::FeeCalculator::min_gas_price();
						let fee = t.gas_limit.saturating_mul(gas_price);

						// Ensure the relayer's account has enough balance to withdraw.
						ensure!(
							evm_ensure_can_withdraw(
								relayer_account,
								cmp::min(
									fee,
									decimal_convert(MaxUsableBalanceFromRelayer::get(), None)
								),
								WithdrawReasons::TRANSFER
							)
							.is_ok(),
							TransactionValidityError::Invalid(InvalidTransaction::Payment)
						);
						Ok(())
					},
					_ => Ok(()),
				},
			_ => Ok(()),
		}
	}

	fn call_validate(
		relayer_account: &bp_pangoro::AccountId,
		origin: &Origin,
		call: &Call,
	) -> Result<(), TransactionValidityError> {
		match call {
			// Note: Only supprt Ethereum::message_transact(LegacyTransaction)
			Call::Ethereum(darwinia_ethereum::Call::message_transact { transaction: tx }) => {
				match origin.caller() {
					OriginCaller::Ethereum(RawOrigin::EthereumTransaction(id)) => match tx {
						Transaction::Legacy(t) => {
							// Only non-payable call supported.
							if t.value != U256::zero() {
								return Err(TransactionValidityError::Invalid(
									InvalidTransaction::Payment,
								));
							}

							// Use fixed gas price now.
							let gas_price =
								<Runtime as darwinia_evm::Config>::FeeCalculator::min_gas_price();
							let fee = t.gas_limit.saturating_mul(gas_price);

							if evm_ensure_can_withdraw(
								relayer_account,
								cmp::min(
									fee,
									decimal_convert(MaxUsableBalanceFromRelayer::get(), None),
								),
								WithdrawReasons::TRANSFER,
							)
							.is_ok()
							{
								let derived_substrate_address =
									<Runtime as darwinia_evm::Config>::IntoAccountId::derive_substrate_address(*id);
								// Ensure the derived ethereum address has enough balance to pay for
								// the transaction
								let _ =
									<Runtime as darwinia_evm::Config>::RingAccountBasic::transfer(
										&relayer_account,
										&derived_substrate_address,
										fee,
									);
							}

							Ok(())
						},
						_ =>
							Err(TransactionValidityError::Invalid(InvalidTransaction::Custom(1u8))),
					},
					_ => Err(TransactionValidityError::Invalid(InvalidTransaction::Custom(0u8))),
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
