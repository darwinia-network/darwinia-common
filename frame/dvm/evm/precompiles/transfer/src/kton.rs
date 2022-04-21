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
// --- paritytech ---
use fp_evm::{
	Context, ExitError, ExitReason, ExitSucceed, PrecompileFailure, PrecompileOutput,
	PrecompileResult,
};
use frame_support::ensure;
use sp_core::{H160, U256};
use sp_std::{borrow::ToOwned, prelude::*, vec::Vec};
// --- darwinia-network ---
use crate::util;
use darwinia_evm::{runner::Runner, AccountBasic, AccountId, Pallet};
use darwinia_evm_precompile_utils::{
	custom_precompile_err, selector, DvmInputParser, PrecompileHelper,
};
use darwinia_support::evm::{IntoAccountId, TRANSFER_ADDR};

#[selector]
#[derive(Eq, PartialEq)]
pub enum Action {
	TransferAndCall = "transfer_and_call(address,uint256)",
	Withdraw = "withdraw(bytes32,uint256)",
}

pub enum Kton<T: darwinia_ethereum::Config> {
	/// Transfer from substrate account to wkton contract
	TransferAndCall(CallData),
	/// Withdraw from wkton contract to substrate account
	Withdraw(WithdrawData<T>),
}

impl<T: darwinia_ethereum::Config> Kton<T> {
	pub fn transfer(
		input: &[u8],
		target_gas: Option<u64>,
		context: &Context,
		is_static: bool,
	) -> PrecompileResult {
		let dvm_parser = DvmInputParser::new(input)?;
		let action = Action::from_u32(dvm_parser.selector)?;

		let mut precompile_helper = PrecompileHelper::<T>::new(target_gas);
		// Check state modifiers
		precompile_helper.check_state_modifier(context, is_static, StateMutability::NonPayable)?;

		match action {
			Action::TransferAndCall => {
				// Storage: System Account (r:2 w:2)
				// Storage: Ethereum RemainingRingBalance (r:2 w:2)
				// Storage: EVM AccountCodes (r:1 w:0)
				// Storage: EVM AccountStorages (r:2 w:2)
				precompile_helper.record_gas(7, 6)?;

				let call_data = CallData::decode(&dvm_parser.input)?;
				let (caller, wkton, value) =
					(context.caller, call_data.wkton_address, call_data.value);
				// Ensure wkton is a contract
				ensure!(
					!<Pallet<T>>::is_contract_code_empty(&wkton),
					custom_precompile_err("Wkton must be a contract!")
				);

				let caller_account_id =
					<T as darwinia_evm::Config>::IntoAccountId::into_account_id(caller);
				let wkton_account_id =
					<T as darwinia_evm::Config>::IntoAccountId::into_account_id(wkton);
				// Transfer kton from sender to KTON wrapped contract
				T::KtonAccountBasic::transfer(&caller_account_id, &wkton_account_id, value)
					.map_err(|e| PrecompileFailure::Error { exit_status: e })?;
				// Call WKTON wrapped contract deposit
				let raw_input = make_call_data(caller, value)?;
				if let Ok(call_res) = T::Runner::call(
					array_bytes::hex_try_into(TRANSFER_ADDR)
						.map_err(|_| custom_precompile_err("Invalid transfer address"))?,
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
						_ => return Err(custom_precompile_err("Call in Kton precompile failed")),
					}
				}

				Ok(PrecompileOutput {
					exit_status: ExitSucceed::Returned,
					cost: 20000,
					output: Default::default(),
					logs: Default::default(),
				})
			}
			Action::Withdraw => {
				// Storage: System Account (r:2 w:2)
				// Storage: Ethereum RemainingRingBalance (r:2 w:2)
				// Storage: EVM AccountCodes (r:1 w:0)
				precompile_helper.record_gas(5, 4)?;

				let wd = WithdrawData::<T>::decode(&dvm_parser.input)?;
				let (source, to, value) = (context.caller, wd.to_account_id, wd.kton_value);
				// Ensure wkton is a contract
				ensure!(
					!<Pallet<T>>::is_contract_code_empty(&source),
					custom_precompile_err("The caller must be wkton contract")
				);

				let source = <T as darwinia_evm::Config>::IntoAccountId::into_account_id(source);
				T::KtonAccountBasic::transfer(&source, &to, value)
					.map_err(|e| PrecompileFailure::Error { exit_status: e })?;

				Ok(PrecompileOutput {
					exit_status: ExitSucceed::Returned,
					cost: precompile_helper.used_gas(),
					output: Default::default(),
					logs: Default::default(),
				})
			}
		}
	}
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
		.map_err(|_| custom_precompile_err("Make call data error happened"))
}

#[derive(Debug, PartialEq, Eq)]
pub struct CallData {
	wkton_address: H160,
	value: U256,
}

impl CallData {
	pub fn decode(data: &[u8]) -> Result<Self, PrecompileFailure> {
		let tokens = ethabi::decode(&[ParamType::Address, ParamType::Uint(256)], &data)
			.map_err(|_| custom_precompile_err("ethabi decoded error"))?;
		match (tokens[0].clone(), tokens[1].clone()) {
			(Token::Address(eth_wkton_address), Token::Uint(eth_value)) => Ok(CallData {
				wkton_address: util::e2s_address(eth_wkton_address),
				value: util::e2s_u256(eth_value),
			}),
			_ => Err(custom_precompile_err("Invlid call data")),
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
			.map_err(|_| custom_precompile_err("ethabi decoded error"))?;
		match (tokens[0].clone(), tokens[1].clone()) {
			(Token::FixedBytes(address), Token::Uint(eth_value)) => Ok(WithdrawData {
				to_account_id: <T as frame_system::Config>::AccountId::decode(
					&mut address.as_ref(),
				)
				.map_err(|_| custom_precompile_err("Invalid destination address"))?,
				kton_value: util::e2s_u256(eth_value),
			}),
			_ => Err(custom_precompile_err("Invalid withdraw input data")),
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
}
