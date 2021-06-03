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

pub mod kton;
pub mod ring;
pub mod util;

// --- substrate ---
use sp_std::{marker::PhantomData, vec::Vec};
// --- darwinia ---
use darwinia_support::evm::SELECTOR;
use dp_evm::Precompile;
use kton::Kton;
use ring::RingBack;
// --- crate ---
use evm::{Context, ExitError, ExitSucceed};

pub type AccountId<T> = <T as frame_system::Config>::AccountId;

/// Transfer Precompile Contract, used to support the exchange of KTON and RING transfer.
pub enum Transfer<T> {
	/// Transfer RING back from DVM to Darwinia
	RingTransfer,
	/// Transfer KTON between Darwinia and DVM contract
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
			Ok(Transfer::RingTransfer) => RingBack::<T>::transfer(&input, target_gas, context),
			Ok(Transfer::KtonTransfer) => Kton::<T>::transfer(&input, target_gas, context),
			_ => Err(ExitError::Other("Invalid action".into())),
		}
	}
}

/// There are two types of transfers: RING transfer and KTON transfer
///
/// The RingBack has only one action, while KtonTransfer has two: `transfer and call`, `withdraw`.
fn which_transfer<T: dvm_ethereum::Config>(data: &[u8]) -> Result<Transfer<T>, ExitError> {
	if data.len() < SELECTOR {
		return Err(ExitError::Other("Invalid input data！".into()));
	}
	if kton::is_kton_transfer(data) {
		return Ok(Transfer::KtonTransfer);
	}
	Ok(Transfer::RingTransfer)
}
