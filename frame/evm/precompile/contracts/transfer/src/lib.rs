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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Darwinia. If not, see <https://www.gnu.org/licenses/>.

#![cfg_attr(not(feature = "std"), no_std)]

mod kton;
pub mod util;
mod withdraw;

use sha3::Digest;
use sp_core::{H160, U256};
use sp_runtime::traits::UniqueSaturatedInto;
use sp_std::marker::PhantomData;
use sp_std::prelude::*;
use sp_std::vec::Vec;

use codec::Decode;
use darwinia_evm::{AddressMapping, Config};
use darwinia_support::evm::POW_9;
use dp_evm::Precompile;
use ethabi::{Function, Param, ParamType, Token};
use evm::{Context, ExitError, ExitSucceed};

use kton::Kton;
use withdraw::RingBack;

pub type AccountId<T> = <T as frame_system::Config>::AccountId;

/// Transfer Precompile Contract, used to support the exchange of KTON and RING tranfer.
///
/// The contract address: 0000000000000000000000000000000000000015
pub enum Transfer<T> {
	/// Transfer RING bach from dvm to darwinia
	RingBack,
	/// Transfer KTON between darwinia and dvm contract
	KtonTransfer,
	_Impossible(PhantomData<T>),
}

impl<T: dvm_ethereum::Config> Precompile for Transfer<T> {
	fn execute(
		input: &[u8],
		target_gas: Option<u64>,
		context: &Context,
	) -> core::result::Result<(ExitSucceed, Vec<u8>, u64), ExitError> {
		match which_action::<T>(&input) {
			Ok(Transfer::RingBack) => {
				RingBack::<T>::transfer(&input, target_gas, context)?;
			}
			Ok(Transfer::KtonTransfer) => {
				Kton::<T>::execute(&input, target_gas, context)?;
			}
			_ => {
				return Err(ExitError::Other("Invalid action".into()));
			}
		}
		Err(ExitError::Other("Invalid action".into()))
	}
}

fn which_action<T: dvm_ethereum::Config>(data: &[u8]) -> Result<Transfer<T>, ExitError> {
	// TODO: change the way of select action
	if data.len() == 32 {
		return Ok(Transfer::RingBack);
	} else if data.len() == 68 {
		return Ok(Transfer::KtonTransfer);
	} else {
		return Err(ExitError::Other("Invalid action".into()));
	}
}
