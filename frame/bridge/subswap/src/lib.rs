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

//! # Sub Swap Module

#![cfg_attr(not(feature = "std"), no_std)]

pub mod weights;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
	pub mod types {
		// --- darwinia ---
		use crate::pallet::*;
		pub type AccountId<T> = <T as frame_system::Config>::AccountId;
	}
	pub use types::*;

	// --- crates.io ---
	use ethereum_primitives::{EthereumAddress, U256};
	// --- substrate ---
	use frame_support::{
		pallet_prelude::*,
		traits::{Currency, Get},
		transactional, PalletId,
	};
	use frame_system::pallet_prelude::*;
	// --- darwinia ---
	use crate::weights::WeightInfo;

	#[pallet::config]
	pub trait Config: frame_system::Config {}

	#[pallet::pallet]
	pub struct Pallet<T>(_);
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10000)]
		#[transactional]
		pub fn add_liquidity(
			origin: OriginFor<T>,
			token_a: EthereumAddress,
			token_b: EthereumAddress,
			amount_a_desired: U256,
			amount_b_desired: U256,
			amount_a_admin: U256,
			amount_b_admin: U256,
			address_to: EthereumAddress,
			deadline: U256,
		) -> DispatchResultWithPostInfo {
			Ok(().into())
		}

		#[pallet::weight(10000)]
		#[transactional]
		pub fn remove_liquidity(
			origin: OriginFor<T>,
			token_a: EthereumAddress,
			token_b: EthereumAddress,
			liquidity: U256,
			amount_a_admin: U256,
			amount_b_admin: U256,
			to: EthereumAddress,
			deadline: U256,
		) -> DispatchResultWithPostInfo {
			Ok(().into())
		}
	}
}
pub use pallet::*;

pub mod migration {
	#[cfg(feature = "try-runtime")]
	pub mod try_runtime {
		pub fn pre_migrate() -> Result<(), &'static str> {
			Ok(())
		}
	}
}
pub fn migrate() {}
