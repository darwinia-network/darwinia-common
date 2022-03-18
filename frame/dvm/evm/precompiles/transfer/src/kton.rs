// This file is part of Darwinia.
//
// Copyright (C) 2018-2022 Darwinia Network
// SPDX-License-Identifier: GPL-3.0
//
// Darwinia is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Darwinia is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

// --- crates.io ---
use codec::Decode;
use ethabi::{Function, Param, ParamType, StateMutability, Token};
use sha3::Digest;
// --- paritytech ---
use fp_evm::{
	Context, ExitError, ExitReason, ExitSucceed, PrecompileFailure, PrecompileOutput,
	PrecompileResult,
};
use frame_support::ensure;
use sp_core::{H160, U256};
use sp_std::{borrow::ToOwned, prelude::*, vec::Vec};
// --- darwinia-network ---
use crate::{util, AccountId};
use darwinia_evm::{runner::Runner, AccountBasic, Pallet};
use darwinia_support::evm::{SELECTOR, TRANSFER_ADDR};

const TRANSFER_AND_CALL_ACTION: &[u8] = b"transfer_and_call(address,uint256)";
const WITHDRAW_ACTION: &[u8] = b"withdraw(bytes32,uint256)";

pub enum Kton<T: darwinia_ethereum::Config> {
	/// Transfer from substrate account to wkton contract
	TransferAndCall(CallData),
	/// Withdraw from wkton contract to substrate account
	Withdraw(WithdrawData<T>),
}

impl<T: darwinia_ethereum::Config> Kton<T> {
	pub fn transfer(input: &[u8], target_gas: Option<u64>, context: &Context) -> PrecompileResult {
		let action = which_action::<T>(&input)?;

		match action {
			Kton::TransferAndCall(call_data) => {
				let (caller, wkton, value) =
					(context.caller, call_data.wkton_address, call_data.value);
				// Ensure wkton is a contract
				ensure!(
					!<Pallet<T>>::is_contract_code_empty(&wkton),
					PrecompileFailure::Error {
						exit_status: ExitError::Other("Wkton must be a contract!".into()),
					}
				);
				// Ensure context's apparent_value is zero, since the transfer value is encoded in input field
				ensure!(
					context.apparent_value == U256::zero(),
					PrecompileFailure::Error {
						exit_status: ExitError::Other("The value in tx must be zero!".into()),
					}
				);
				// Ensure caller's balance is enough
				ensure!(
					T::KtonAccountBasic::account_basic(&caller).balance >= value,
					PrecompileFailure::Error {
						exit_status: ExitError::OutOfFund,
					}
				);

				// Transfer kton from sender to KTON wrapped contract
				T::KtonAccountBasic::transfer(&caller, &wkton, value)
					.map_err(|e| PrecompileFailure::Error { exit_status: e })?;
				// Call WKTON wrapped contract deposit
				let raw_input = make_call_data(caller, value)?;
				if let Ok(call_res) = T::Runner::call(
					array_bytes::hex_try_into(TRANSFER_ADDR).map_err(|_| {
						PrecompileFailure::Error {
							exit_status: ExitError::Other("Invalid transfer address".into()),
						}
					})?,
					wkton,
					raw_input.to_vec(),
					U256::zero(),
					target_gas.unwrap_or_default(),
					None,
					None,
					None,
					Vec::new(),
					T::config(),
				) {
					match call_res.exit_reason {
						ExitReason::Succeed(_) => {
							log::debug!("Transfer and call execute success.");
						}
						_ => {
							return Err(PrecompileFailure::Error {
								exit_status: ExitError::Other(
									"Call in Kton precompile failed".into(),
								),
							})
						}
					}
				}

				<darwinia_ethereum::Pallet<T>>::deposit_event(
					darwinia_ethereum::Event::TransferToWKton(caller, value),
				);
				Ok(PrecompileOutput {
					exit_status: ExitSucceed::Returned,
					cost: 20000,
					output: Default::default(),
					logs: Default::default(),
				})
			}
			Kton::Withdraw(wd) => {
				let (source, to, value) = (context.caller, wd.to_account_id, wd.kton_value);
				// Ensure wkton is a contract
				ensure!(
					!<Pallet<T>>::is_contract_code_empty(&source),
					PrecompileFailure::Error {
						exit_status: ExitError::Other("The caller must be wkton contract!".into()),
					}
				);
				// Ensure context's apparent_value is zero
				ensure!(
					context.apparent_value == U256::zero(),
					PrecompileFailure::Error {
						exit_status: ExitError::Other("The value in tx must be zero!".into()),
					}
				);
				// Ensure source's balance is enough
				let source_kton = T::KtonAccountBasic::account_basic(&source);
				ensure!(
					source_kton.balance >= value,
					PrecompileFailure::Error {
						exit_status: ExitError::OutOfFund,
					}
				);

				// Transfer
				let new_source_kton_balance = source_kton.balance.saturating_sub(value);
				T::KtonAccountBasic::mutate_account_basic_balance(&source, new_source_kton_balance);

				let target_kton = T::KtonAccountBasic::account_balance(&to);
				let new_target_kton_balance = target_kton.saturating_add(value);
				T::KtonAccountBasic::mutate_account_balance(&to, new_target_kton_balance);

				<darwinia_ethereum::Pallet<T>>::deposit_event(
					darwinia_ethereum::Event::WithdrawFromWKton(to, value),
				);
				Ok(PrecompileOutput {
					exit_status: ExitSucceed::Returned,
					cost: 20000,
					output: Default::default(),
					logs: Default::default(),
				})
			}
		}
	}
}

/// which action depends on the function selector
pub fn which_action<T: darwinia_ethereum::Config>(
	input_data: &[u8],
) -> Result<Kton<T>, PrecompileFailure> {
	let transfer_and_call_action = &sha3::Keccak256::digest(&TRANSFER_AND_CALL_ACTION)[0..SELECTOR];
	let withdraw_action = &sha3::Keccak256::digest(&WITHDRAW_ACTION)[0..SELECTOR];
	if &input_data[0..SELECTOR] == transfer_and_call_action {
		let decoded_data = CallData::decode(&input_data[SELECTOR..])?;
		return Ok(Kton::TransferAndCall(decoded_data));
	} else if &input_data[0..SELECTOR] == withdraw_action {
		let decoded_data = WithdrawData::decode(&input_data[SELECTOR..])?;
		return Ok(Kton::Withdraw(decoded_data));
	}
	Err(PrecompileFailure::Error {
		exit_status: ExitError::Other("Invalid Action！".into()),
	})
}
pub fn is_kton_transfer(data: &[u8]) -> bool {
	let transfer_and_call_action = &sha3::Keccak256::digest(&TRANSFER_AND_CALL_ACTION)[0..SELECTOR];
	let withdraw_action = &sha3::Keccak256::digest(&WITHDRAW_ACTION)[0..SELECTOR];
	&data[0..SELECTOR] == transfer_and_call_action || &data[0..SELECTOR] == withdraw_action
}

fn make_call_data(
	sp_address: sp_core::H160,
	sp_value: sp_core::U256,
) -> Result<Vec<u8>, PrecompileFailure> {
	let eth_address = util::s2e_address(sp_address);
	let eth_value = util::s2e_u256(sp_value);
	#[allow(deprecated)]
	let func = Function {
		name: "deposit".to_owned(),
		inputs: vec![
			Param {
				name: "address".to_owned(),
				kind: ParamType::Address,
				internal_type: Some("address".into()),
			},
			Param {
				name: "value".to_owned(),
				kind: ParamType::Uint(256),
				internal_type: Some("uint256".into()),
			},
		],
		outputs: vec![],
		constant: false,
		state_mutability: StateMutability::NonPayable,
	};
	func.encode_input(&[Token::Address(eth_address), Token::Uint(eth_value)])
		.map_err(|_| PrecompileFailure::Error {
			exit_status: ExitError::Other("Make call data error happened".into()),
		})
}

#[derive(Debug, PartialEq, Eq)]
pub struct CallData {
	wkton_address: H160,
	value: U256,
}

impl CallData {
	pub fn decode(data: &[u8]) -> Result<Self, PrecompileFailure> {
		let tokens =
			ethabi::decode(&[ParamType::Address, ParamType::Uint(256)], &data).map_err(|_| {
				PrecompileFailure::Error {
					exit_status: ExitError::Other("ethabi decoded error".into()),
				}
			})?;
		match (tokens[0].clone(), tokens[1].clone()) {
			(Token::Address(eth_wkton_address), Token::Uint(eth_value)) => Ok(CallData {
				wkton_address: util::e2s_address(eth_wkton_address),
				value: util::e2s_u256(eth_value),
			}),
			_ => Err(PrecompileFailure::Error {
				exit_status: ExitError::Other("Invlid call data".into()),
			}),
		}
	}
}

#[derive(Debug, PartialEq, Eq)]
pub struct WithdrawData<T: frame_system::Config> {
	pub to_account_id: AccountId<T>,
	pub kton_value: U256,
}

impl<T: frame_system::Config> WithdrawData<T> {
	pub fn decode(data: &[u8]) -> Result<Self, PrecompileFailure> {
		let tokens = ethabi::decode(&[ParamType::FixedBytes(32), ParamType::Uint(256)], &data)
			.map_err(|_| PrecompileFailure::Error {
				exit_status: ExitError::Other("ethabi decoded error".into()),
			})?;
		match (tokens[0].clone(), tokens[1].clone()) {
			(Token::FixedBytes(address), Token::Uint(eth_value)) => Ok(WithdrawData {
				to_account_id: <T as frame_system::Config>::AccountId::decode(
					&mut address.as_ref(),
				)
				.map_err(|_| PrecompileFailure::Error {
					exit_status: ExitError::Other("Invalid destination address".into()),
				})?,
				kton_value: util::e2s_u256(eth_value),
			}),
			_ => Err(PrecompileFailure::Error {
				exit_status: ExitError::Other("Invalid withdraw input data".into()),
			}),
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
		let transfer_and_call_action =
			&sha3::Keccak256::digest(&TRANSFER_AND_CALL_ACTION)[0..SELECTOR];
		let withdraw_action = &sha3::Keccak256::digest(&WITHDRAW_ACTION)[0..SELECTOR];

		let data = vec![0; 32];
		assert!(!is_kton_transfer(&data));
		let data1 = transfer_and_call_action;
		assert!(is_kton_transfer(&data1));
		let data2 = withdraw_action;
		assert!(is_kton_transfer(&data2));
	}
}
