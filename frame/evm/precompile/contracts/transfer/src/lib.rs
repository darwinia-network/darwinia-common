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

use frame_support::traits::{Currency, ExistenceRequirement};
use kton::Kton;
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
use kton::KtonAction;
use withdraw::WithDraw;

pub type AccountId<T> = <T as frame_system::Config>::AccountId;

pub enum TransferAction<T: Config> {
	RingBack,
	KtonAction,
	_Impossible(PhantomData<T>),
}

impl<T: dvm_ethereum::Config + frame_system::Config> Precompile for TransferAction<T> {
	fn execute(
		input: &[u8],
		target_gas: Option<u64>,
		context: &Context,
	) -> core::result::Result<(ExitSucceed, Vec<u8>, u64), ExitError> {
		match which_action::<T>(&input) {
			Ok(TransferAction::RingBack) => {
				WithDraw::<T>::execute(&input, target_gas, context)?;
			}
			Ok(TransferAction::KtonAction) => {
				KtonAction::<T>::execute(&input, target_gas, context)?;
			}
			_ => {
				return Err(ExitError::Other("Invalid input data length".into()));
			}
		}
		Err(ExitError::Other("Invalid input data length".into()))
	}
}

fn which_action<T: frame_system::Config + dvm_ethereum::Config>(
	data: &[u8],
) -> Result<TransferAction<T>, ExitError> {
	if data.len() == 32 {
		return Ok(TransferAction::RingBack);
	} else if data.len() == 68 {
		return Ok(TransferAction::KtonAction);
	} else {
		return Err(ExitError::Other("Invalid input data length".into()));
	}
}
