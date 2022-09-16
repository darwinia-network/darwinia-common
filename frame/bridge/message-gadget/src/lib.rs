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

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

// --- core ---
use core::marker::PhantomData;
// --- darwinia-network ---
use darwinia_evm::Runner;
// --- paritytech ---
use frame_support::{log, pallet_prelude::*, traits::Get};
use frame_system::pallet_prelude::*;
use sp_core::{H160, H256};
use sp_io::hashing;

#[frame_support::pallet]
pub mod pallet {
	// --- darwinia-network ---
	use crate::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {}

	#[pallet::storage]
	#[pallet::getter(fn commitment_contract)]
	pub type CommitmentContract<T> = StorageValue<_, H160, ValueQuery>;

	#[cfg_attr(feature = "std", derive(Default))]
	#[pallet::genesis_config]
	pub struct GenesisConfig {
		pub commitment_contract: H160,
	}
	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {
			<CommitmentContract<T>>::put(H160::default());
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		pub fn set_commitment_contract(
			origin: OriginFor<T>,
			commitment_contract: H160,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			<CommitmentContract<T>>::put(commitment_contract);

			Ok(().into())
		}
	}
}
pub use pallet::*;

const LOG_TARGET: &str = "runtime::message-gadget";

pub struct MessageRootGetter<T>(PhantomData<T>);
impl<T> Get<Option<H256>> for MessageRootGetter<T>
where
	T: Config + darwinia_evm::Config,
{
	fn get() -> Option<H256> {
		if let Ok(info) = <T as darwinia_evm::Config>::Runner::call(
			H160::default(),
			<CommitmentContract<T>>::get(),
			hashing::keccak_256(b"commitment()")[..4].to_vec(),
			0.into(),
			1_000_000_000,
			None,
			None,
			None,
			vec![],
			false,
			<T as darwinia_evm::Config>::config(),
		) {
			let raw_message_root = info.value;
			if raw_message_root.len() != 32 {
				log::warn!(
					target: LOG_TARGET,
					"Invalid raw message root: {:?}, return.",
					raw_message_root
				);

				return None;
			}
			return Some(H256::from_slice(&raw_message_root));
		}

		None
	}
}
