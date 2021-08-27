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

//! # Fee Market Module

#![cfg_attr(not(feature = "std"), no_std)]

pub mod weights;
use crate::weights::WeightInfo;

#[cfg(test)]
mod tests;

use frame_support::{
	pallet_prelude::*,
	traits::{Currency, Get},
	PalletId,
};
use frame_system::pallet_prelude::*;
pub use pallet::*;

pub type AccountId<T> = <T as frame_system::Config>::AccountId;
pub type RingBalance<T> = <RingCurrency<T> as Currency<AccountId<T>>>::Balance;
type RingCurrency<T> = <T as Config>::RingCurrency;

#[frame_support::pallet]
pub mod pallet {
	use crate::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		// --- substrate ---
		type WeightInfo: WeightInfo;
		// --- darwinia ---
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		type RingCurrency: Currency<AccountId<Self>>;
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig {}
	#[cfg(feature = "std")]
	impl Default for GenesisConfig {
		fn default() -> Self {
			Self {}
		}
	}
	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {}
	}
	#[pallet::pallet]
	pub struct Pallet<T>(_);
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}
	#[pallet::call]
	impl<T: Config> Pallet<T> {}
}
