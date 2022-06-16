pub use pallet_bridge_dispatch::Instance1 as WithPangolinDispatch;

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

pub struct CallValidator;
impl CallValidate<bp_pangoro::AccountId, Origin, Call> for CallValidator {
	fn pre_dispatch(
		relayer_account: &bp_pangoro::AccountId,
		origin: &Origin,
		call: &Call,
	) -> Result<(), TransactionValidityError> {
		match call {
			// Note: Only supprt Ethereum::transact(LegacyTransaction)
			Call::Ethereum(darwinia_ethereum::Call::transact { transaction: tx }) => {
				match origin.caller() {
					OriginCaller::Ethereum(RawOrigin::EthereumTransaction(id)) => {
						match tx {
							Transaction::Legacy(t) => {
								// Only non-payable call supported.
								if t.value != U256::zero() {
									return Err(TransactionValidityError::Invalid(
										InvalidTransaction::Payment,
									));
								}
								let fee = t.gas_limit.saturating_mul(t.gas_limit);

								let derived_substrate_address = <Runtime as darwinia_evm::Config>::IntoAccountId::derive_substrate_address(*id);
								if <Runtime as darwinia_evm::Config>::RingAccountBasic::account_balance(relayer_account) >= fee {
										// Ensure the derived ethereum address has enough balance to pay for the transaction
										let _ = <Runtime as darwinia_evm::Config>::RingAccountBasic::transfer(
											&relayer_account,
											&derived_substrate_address,
											fee
										);
								}

								Ethereum::validate_transaction_in_block(*id, tx)
							},
							_ => Err(TransactionValidityError::Invalid(
								InvalidTransaction::Custom(1u8),
							)),
						}
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
