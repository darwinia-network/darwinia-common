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
mod ring;
pub mod util;

// --- substrate ---
use sp_std::vec::Vec;
use sp_std::{marker::PhantomData, prelude::*};
// --- darwinia ---
use dp_evm::Precompile;
use kton::Kton;
use ring::RingBack;
// --- crate ---
use evm::{Context, ExitError, ExitSucceed};

pub type AccountId<T> = <T as frame_system::Config>::AccountId;

/// Transfer Precompile Contract, used to support the exchange of KTON and RING tranfer.
///
/// The contract address: 0000000000000000000000000000000000000015
#[derive(PartialEq, Eq, Debug)]
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
		match which_transfer::<T>(&input) {
			Transfer::RingBack => RingBack::<T>::transfer(&input, target_gas, context),
			Transfer::KtonTransfer => Kton::<T>::transfer(&input, target_gas, context),
			_ => Err(ExitError::Other("Invalid action".into())),
		}
	}
}

fn which_transfer<T: dvm_ethereum::Config>(data: &[u8]) -> Transfer<T> {
	if !kton::is_kton_transfer(data) {
		Transfer::RingBack
	} else {
		Transfer::KtonTransfer
	}
}
