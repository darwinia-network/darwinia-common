// This file is part of Darwinia.
//
// Copyright (C) 2018-2021 Darwinia Network
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

// --- substrate ---
use frame_support::{ensure, traits::Currency};
use sp_core::U256;
use sp_runtime::{traits::UniqueSaturatedInto, SaturatedConversion};
use sp_std::{marker::PhantomData, prelude::*, vec::Vec};
// --- darwinia ---
use crate::AccountId;
use darwinia_evm::{AccountBasic, Config};
use darwinia_support::evm::{POW_9, TRANSFER_ADDR};
use dvm_ethereum::{
	account_basic::{RemainBalanceOp, RingRemainBalance},
	RingBalance,
};
// --- crates ---
use codec::Decode;
use evm::{Context, ExitError, ExitSucceed};

pub struct RingBack<T: Config> {
	_maker: PhantomData<T>,
}

impl<T: dvm_ethereum::Config> RingBack<T> {
	/// The Withdraw process is divided into two part:
	/// 1. parse the withdrawal address from the input parameter and get the contract address and value from the context
	/// 2. transfer from the contract address to withdrawal address
	///
	/// Input data: 32-bit substrate withdrawal public key
	pub fn transfer(
		input: &[u8],
		_: Option<u64>,
		context: &Context,
	) -> core::result::Result<(ExitSucceed, Vec<u8>, u64), ExitError> {
		// Decode input data
		let helper = U256::from(POW_9);
		let input = InputData::<T>::decode(&input)?;
		let (source, value) = (context.address, context.apparent_value);
		let source_account = T::RingAccountBasic::account_basic(&source);

		// Ensure the context address should be precompile address
		ensure!(
			source == array_bytes::hex2array_unchecked!(TRANSFER_ADDR, 20).into(),
			ExitError::Other("Invalid context address".into())
		);
		// Ensure the context address balance is enough
		ensure!(source_account.balance >= value, ExitError::OutOfFund);

		// Transfer
		let new_source_balance = source_account.balance.saturating_sub(value);
		T::RingAccountBasic::mutate_account_basic(&source, new_source_balance);
		let (currency_value, remain_balance) = context.apparent_value.div_mod(helper);
		<T as darwinia_evm::Config>::RingCurrency::deposit_creating(
			&input.dest,
			currency_value.low_u128().unique_saturated_into(),
		);
		<RingRemainBalance as RemainBalanceOp<T, RingBalance<T>>>::inc_remaining_balance(
			&input.dest,
			remain_balance.low_u128().saturated_into(),
		);

		Ok((ExitSucceed::Returned, vec![], 10000))
	}
}

#[derive(Debug, PartialEq, Eq)]
pub struct InputData<T: frame_system::Config> {
	pub dest: AccountId<T>,
}

impl<T: frame_system::Config> InputData<T> {
	pub fn decode(data: &[u8]) -> Result<Self, ExitError> {
		if data.len() == 32 {
			let mut dest_bytes = [0u8; 32];
			dest_bytes.copy_from_slice(&data[0..32]);

			return Ok(InputData {
				dest: <T as frame_system::Config>::AccountId::decode(&mut dest_bytes.as_ref())
					.map_err(|_| ExitError::Other("Invalid destination address".into()))?,
			});
		}
		Err(ExitError::Other("Invalid input data length".into()))
	}
}
