// --- substrate ---
use sp_core::{H160, U256};
use sp_runtime::{traits::UniqueSaturatedInto, SaturatedConversion};
use sp_std::{borrow::ToOwned, prelude::*, vec::Vec};
// --- darwinia ---
use crate::util;
use crate::AccountId;
use darwinia_evm::{Account, AccountBasic, Module, Runner};
use darwinia_support::evm::POW_9;
use dvm_ethereum::{
	account_basic::{KtonRemainBalance, RemainBalanceOp},
	KtonBalance,
};
// --- crates ---
use codec::Decode;
use core::str::FromStr;
use ethabi::{Function, Param, ParamType, Token};
use evm::{Context, ExitError, ExitReason, ExitSucceed};
use frame_support::{ensure, traits::Currency};
use sha3::Digest;

const TRANSFER_AND_CALL_ACTION: &[u8] = b"transfer_and_call(address,uint256)";
const WITHDRAW_ACTION: &[u8] = b"withdraw(bytes32,uint256)";
// const KTON_PRECOMPILE: &str = "0000000000000000000000000000000000000016";
const KTON_PRECOMPILE: &str = "0000000000000000000000000000000000000015";

pub enum Kton<T: frame_system::Config> {
	/// Transfer from substrate account to wkton contract
	TransferAndCall(CallData),
	/// Withdraw from wkton contract to substrate account
	Withdraw(WithdrawData<T>),
}

impl<T: frame_system::Config + dvm_ethereum::Config> Kton<T> {
	pub fn transfer(
		input: &[u8],
		target_gas: Option<u64>,
		context: &Context,
	) -> core::result::Result<(ExitSucceed, Vec<u8>, u64), ExitError> {
		let helper = U256::from(POW_9);
		let action = which_action::<T>(&input)?;

		match action {
			Kton::TransferAndCall(call_data) => {
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
					target_gas.unwrap_or_default(),
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
			Kton::Withdraw(wd) => {
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
pub fn which_action<T: frame_system::Config>(input_data: &[u8]) -> Result<Kton<T>, ExitError> {
	let transfer_and_call_action = &sha3::Keccak256::digest(&TRANSFER_AND_CALL_ACTION)[0..4];
	let withdraw_action = &sha3::Keccak256::digest(&WITHDRAW_ACTION)[0..4];
	if &input_data[0..4] == transfer_and_call_action {
		let decoded_data = CallData::decode(&input_data[4..])?;
		return Ok(Kton::TransferAndCall(decoded_data));
	} else if &input_data[0..4] == withdraw_action {
		let decoded_data = WithdrawData::decode(&input_data[4..])?;
		return Ok(Kton::Withdraw(decoded_data));
	}
	Err(ExitError::Other("Invalid Action！".into()))
}
pub fn is_kton_transfer(data: &[u8]) -> bool {
	let transfer_and_call_action = &sha3::Keccak256::digest(&TRANSFER_AND_CALL_ACTION)[0..4];
	let withdraw_action = &sha3::Keccak256::digest(&WITHDRAW_ACTION)[0..4];
	&data[0..4] == transfer_and_call_action || &data[0..4] == withdraw_action
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

#[derive(Debug, PartialEq, Eq)]
pub struct CallData {
	wkton_address: H160,
	value: U256,
}

impl CallData {
	pub fn decode(data: &[u8]) -> Result<Self, ExitError> {
		let tokens = ethabi::decode(&[ParamType::Address, ParamType::Uint(256)], &data)
			.map_err(|_| ExitError::Other("ethabi decoded error".into()))?;
		match (tokens[0].clone(), tokens[1].clone()) {
			(Token::Address(eth_wkton_address), Token::Uint(eth_value)) => Ok(CallData {
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

#[cfg(test)]
mod tests {
	use super::*;
	use std::str::FromStr;

	#[test]
	fn test_make_input() {
		let mock_address =
			sp_core::H160::from_str("Aa01a1bEF0557fa9625581a293F3AA7770192632").unwrap();
		let mock_value_1 = sp_core::U256::from(30);
		let expected_str = "0x47e7ef24000000000000000000000000aa01a1bef0557fa9625581a293f3aa7770192632000000000000000000000000000000000000000000000000000000000000001e";

		let encoded_str =
			array_bytes::bytes2hex("0x", make_call_data(mock_address, mock_value_1).unwrap());
		assert_eq!(encoded_str, expected_str);

		let mock_value_2 = sp_core::U256::from(25);
		let encoded_str =
			array_bytes::bytes2hex("0x", make_call_data(mock_address, mock_value_2).unwrap());
		assert_ne!(encoded_str, expected_str);
	}

	#[test]
	fn test_is_kton_transfer() {
		let transfer_and_call_action = &sha3::Keccak256::digest(&TRANSFER_AND_CALL_ACTION)[0..4];
		let withdraw_action = &sha3::Keccak256::digest(&WITHDRAW_ACTION)[0..4];

		let data = vec![0; 32];
		assert!(!is_kton_transfer(&data));
		let data1 = transfer_and_call_action;
		assert!(is_kton_transfer(&data1));
		let data2 = withdraw_action;
		assert!(is_kton_transfer(&data2));
	}
}
