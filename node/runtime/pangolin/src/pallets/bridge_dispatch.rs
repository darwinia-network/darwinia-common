pub use pallet_bridge_dispatch::{
	Instance1 as WithPangoroDispatch, Instance2 as WithPangolinParachainDispatch,
};

// --- paritytech ---
use frame_support::traits::OriginTrait;
use sp_runtime::transaction_validity::{InvalidTransaction, TransactionValidityError};
// --- darwinia-network ---
use crate::*;
use bp_message_dispatch::{CallValidate, IntoDispatchOrigin as IntoDispatchOriginT};
use bp_messages::{LaneId, MessageNonce};
use darwinia_ethereum::{RawOrigin, Transaction};
use darwinia_evm::AccountBasic;
use darwinia_support::evm::{DeriveEthereumAddress, DeriveSubstrateAddress};
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

						let max_usable_balance_from_relayer = MaxUsableBalanceFromRelayer::get();
						// TODO
						// if !evm_ensure_can_withdraw(relayer_account, Transfer,
						// cmp::min(max_usable_balance_from_relayer * 10 ** 9, fee)) return Err

						Ok(())
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
			// Note: Only supprt Ethereum::message_transact(LegacyTransaction)
			Call::Ethereum(darwinia_ethereum::Call::message_transact { transaction: tx }) => {
				match origin.caller() {
					OriginCaller::Ethereum(RawOrigin::EthereumTransaction(id)) =>
						match tx {
							Transaction::Legacy(t) => {
								// Only non-payable call supported.
								if t.value != U256::zero() {
									return Err(TransactionValidityError::Invalid(
										InvalidTransaction::Payment,
									));
								}
								let gas_price =
									<Runtime as darwinia_evm::Config>::FeeCalculator::min_gas_price(
									);
								let fee = t.gas_limit.saturating_mul(gas_price);

								let derived_substrate_address = <Runtime as darwinia_evm::Config>::IntoAccountId::derive_substrate_address(*id);
								if <Runtime as darwinia_evm::Config>::RingAccountBasic::account_balance(relayer_account) >= fee {
								// Ensure the derived ethereum address has enough balance to pay for the transaction
								let _ = <Runtime as darwinia_evm::Config>::RingAccountBasic::transfer(
									&relayer_account,
									&derived_substrate_address,
									fee
								);
							}

								Ok(())
							},
							_ => Err(TransactionValidityError::Invalid(
								InvalidTransaction::Custom(1u8),
							)),
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
	type CallValidator = CallValidator;
	type EncodedCall = bm_pangolin_parachain::FromPangolinParachainEncodedCall;
	type Event = Event;
	type IntoDispatchOrigin = IntoDispatchOrigin;
	type SourceChainAccountId = bp_pangolin_parachain::AccountId;
	type TargetChainAccountPublic = bp_pangolin::AccountPublic;
	type TargetChainSignature = bp_pangolin::Signature;
}
