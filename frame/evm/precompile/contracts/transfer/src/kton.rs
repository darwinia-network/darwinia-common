use crate::util;
use crate::AccountId;
use codec::Decode;
use core::str::FromStr;
use darwinia_evm::AddressMapping;
use darwinia_evm::{Account, AccountBasic, Config, Module, Runner};
use darwinia_support::evm::POW_9;
use dvm_ethereum::{
	account_basic::{KtonRemainBalance, RemainBalanceOp},
	KtonBalance,
};
use ethabi::{Function, Param, ParamType, Token};
use evm::{Context, ExitError, ExitReason, ExitSucceed};
use frame_support::{ensure, traits::Currency};
use sha3::Digest;
use sp_core::{H160, U256};
use sp_runtime::{traits::UniqueSaturatedInto, SaturatedConversion};
use sp_std::{borrow::ToOwned, marker::PhantomData, prelude::*, vec::Vec};

const TRANSFER_AND_CALL_ACTION: &[u8] = b"transfer_and_call(address,uint256)";
const WITHDRAW_ACTION: &[u8] = b"withdraw(bytes32,uint256)";
const KTON_PRECOMPILE: &str = "0000000000000000000000000000000000000016";

/// Kton Precompile Contract is used to support the exchange of KTON native asset between darwinia and dvm contract
///
/// The contract address: 0000000000000000000000000000000000000016
pub struct Kton<T: Config> {
	_maker: PhantomData<T>,
}

pub enum KtonAction<T: frame_system::Config> {
	/// Transfer from substrate account to wkton contract
	TransferAndCall(TACallData),
	/// Withdraw from wkton contract to substrate account
	Withdraw(WithdrawData<T>),
}

impl<T: frame_system::Config + dvm_ethereum::Config> KtonAction<T> {
	pub fn execute(
		input: &[u8],
		target_limit: Option<u64>,
		context: &Context,
	) -> core::result::Result<(ExitSucceed, Vec<u8>, u64), ExitError> {
		let helper = U256::from(POW_9);
		let action = which_action::<T>(&input)?;

		match action {
			KtonAction::TransferAndCall(call_data) => {
				// Ensure wkton is a contract
				ensure!(
					!Module::<T>::is_contract_code_empty(&call_data.wkton_address),
					ExitError::Other("Wkton must be a contract!".into())
				);
				// Ensure context's apparent_value is zero, since the transfer value is encoded in input field
				ensure!(
					context.apparent_value == U256::zero(),
					ExitError::Other("The value in tx must be zero!".into())
				);
				// Ensure caller's balance is enough
				ensure!(
					T::KtonAccountBasic::account_basic(&context.caller).balance >= call_data.value,
					ExitError::OutOfFund
				);

				// Transfer kton from sender to KTON wrapped contract
				T::KtonAccountBasic::transfer(
					&context.caller,
					&call_data.wkton_address,
					call_data.value,
				)?;
				// Call WKTON wrapped contract deposit
				let precompile_address = H160::from_str(KTON_PRECOMPILE).unwrap_or_default();
				let raw_input = make_call_data(context.caller, call_data.value)?;
				if let Ok(call_res) = T::Runner::call(
					precompile_address,
					call_data.wkton_address,
					raw_input.to_vec(),
					U256::zero(),
					target_limit.unwrap_or_default(),
					None,
					None,
					T::config(),
				) {
					match call_res.exit_reason {
						ExitReason::Succeed(_) => {
							log::debug!("Transfer and call execute success.");
						}
						_ => return Err(ExitError::Other("Call in Kton precompile failed".into())),
					}
				}

				Ok((ExitSucceed::Returned, vec![], 20000))
			}
			KtonAction::Withdraw(wd) => {
				// Ensure wkton is a contract
				ensure!(
					!Module::<T>::is_contract_code_empty(&context.caller),
					ExitError::Other("The caller must be wkton contract!".into())
				);
				// Ensure context's apparent_value is zero
				ensure!(
					context.apparent_value == U256::zero(),
					ExitError::Other("The value in tx must be zero!".into())
				);
				// Ensure caller's balance is enough
				let caller_kton = T::KtonAccountBasic::account_basic(&context.caller);
				ensure!(caller_kton.balance >= wd.kton_value, ExitError::OutOfFund);

				// Transfer
				let new_wkton_balance = caller_kton.balance.saturating_sub(wd.kton_value);
				T::KtonAccountBasic::mutate_account_basic(
					&context.caller,
					Account {
						nonce: caller_kton.nonce,
						balance: new_wkton_balance,
					},
				);
				let (currency_value, remain_balance) = wd.kton_value.div_mod(helper);
				<T as darwinia_evm::Config>::KtonCurrency::deposit_creating(
					&wd.to_account_id,
					currency_value.low_u128().unique_saturated_into(),
				);
				<KtonRemainBalance as RemainBalanceOp<T, KtonBalance<T>>>::inc_remaining_balance(
					&wd.to_account_id,
					remain_balance.low_u128().saturated_into(),
				);

				Ok((ExitSucceed::Returned, vec![], 20000))
			}
		}
	}
}

/// which action depends on the function selector
pub fn which_action<T: frame_system::Config>(
	input_data: &[u8],
) -> Result<KtonAction<T>, ExitError> {
	let transfer_and_call_action = &sha3::Keccak256::digest(&TRANSFER_AND_CALL_ACTION)[0..4];
	let withdraw_action = &sha3::Keccak256::digest(&WITHDRAW_ACTION)[0..4];
	if &input_data[0..4] == transfer_and_call_action {
		let decoded_data = TACallData::decode(&input_data[4..])?;
		return Ok(KtonAction::TransferAndCall(decoded_data));
	} else if &input_data[0..4] == withdraw_action {
		let decoded_data = WithdrawData::decode(&input_data[4..])?;
		return Ok(KtonAction::Withdraw(decoded_data));
	}
	Err(ExitError::Other("Invalid Action！".into()))
}

#[derive(Debug, PartialEq, Eq)]
pub struct TACallData {
	wkton_address: H160,
	value: U256,
}

impl TACallData {
	pub fn decode(data: &[u8]) -> Result<Self, ExitError> {
		let tokens = ethabi::decode(&[ParamType::Address, ParamType::Uint(256)], &data)
			.map_err(|_| ExitError::Other("ethabi decoded error".into()))?;
		match (tokens[0].clone(), tokens[1].clone()) {
			(Token::Address(eth_wkton_address), Token::Uint(eth_value)) => Ok(TACallData {
				wkton_address: util::e2s_address(eth_wkton_address),
				value: util::e2s_u256(eth_value),
			}),
			_ => Err(ExitError::Other("Invlid call data".into())),
		}
	}
}

#[derive(Debug, PartialEq, Eq)]
pub struct WithdrawData<T: frame_system::Config> {
	pub to_account_id: AccountId<T>,
	pub kton_value: U256,
}

impl<T: frame_system::Config> WithdrawData<T> {
	pub fn decode(data: &[u8]) -> Result<Self, ExitError> {
		let tokens = ethabi::decode(&[ParamType::FixedBytes(32), ParamType::Uint(256)], &data)
			.map_err(|_| ExitError::Other("ethabi decoded error".into()))?;
		match (tokens[0].clone(), tokens[1].clone()) {
			(Token::FixedBytes(address), Token::Uint(eth_value)) => Ok(WithdrawData {
				to_account_id: <T as frame_system::Config>::AccountId::decode(
					&mut address.as_ref(),
				)
				.map_err(|_| ExitError::Other("Invalid destination address".into()))?,
				kton_value: util::e2s_u256(eth_value),
			}),
			_ => Err(ExitError::Other("Invlid withdraw input data".into())),
		}
	}
}

/// which action depends on the function selector
pub fn which_kton_action<T: frame_system::Config>(
	input_data: &[u8],
) -> Result<KtonAction<T>, ExitError> {
	let transfer_and_call_action = &sha3::Keccak256::digest(&TRANSFER_AND_CALL_ACTION)[0..4];
	let withdraw_action = &sha3::Keccak256::digest(&WITHDRAW_ACTION)[0..4];
	if &input_data[0..4] == transfer_and_call_action {
		let decoded_data = TACallData::decode(&input_data[4..])?;
		return Ok(KtonAction::TransferAndCall(decoded_data));
	} else if &input_data[0..4] == withdraw_action {
		let decoded_data = WithdrawData::decode(&input_data[4..])?;
		return Ok(KtonAction::Withdraw(decoded_data));
	}
	Err(ExitError::Other("Invalid Action！".into()))
}

fn make_call_data(
	sp_address: sp_core::H160,
	sp_value: sp_core::U256,
) -> Result<Vec<u8>, ExitError> {
	let eth_address = util::s2e_address(sp_address);
	let eth_value = util::s2e_u256(sp_value);
	let func = Function {
		name: "deposit".to_owned(),
		inputs: vec![
			Param {
				name: "address".to_owned(),
				kind: ParamType::Address,
			},
			Param {
				name: "value".to_owned(),
				kind: ParamType::Uint(256),
			},
		],
		outputs: vec![],
		constant: false,
	};
	func.encode_input(&[Token::Address(eth_address), Token::Uint(eth_value)])
		.map_err(|_| ExitError::Other("Make call data error happened".into()))
}
