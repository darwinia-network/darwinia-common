// This file is part of Darwinia.
//
// Copyright (C) 2018-2020 Darwinia Network
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
// along with Darwinia.  If not, see <https://www.gnu.org/licenses/>.

//! # Ecdsa Authorities Module

#![cfg_attr(not(feature = "std"), no_std)]

mod types {
	// --- darwinia ---
	use crate::*;

	pub type AccountId<T> = <T as frame_system::Trait>::AccountId;
	pub type BlockNumber<T> = <T as frame_system::Trait>::BlockNumber;
	pub type RingBalance<T, I> = <RingCurrency<T, I> as Currency<AccountId<T>>>::Balance;
	type RingCurrency<T, I> = <T as Trait<I>>::RingCurrency;

	pub type Signer<T, I> = <<T as Trait<I>>::BackableChain as Backable>::Signer;
}

// --- substrate ---
use frame_support::{
	decl_error, decl_module, decl_storage,
	traits::{Currency, Get},
};
use sp_runtime::DispatchResult;
// --- darwinia ---
use darwinia_relay_primitives::relay_authorities::*;
use darwinia_support::balance::lock::*;
use types::*;

pub trait Trait<I: Instance = DefaultInstance>: frame_system::Trait {
	type RingCurrency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

	type TermDuration: Get<Self::BlockNumber>;

	type BackableChain: Backable;

	type WeightInfo: WeightInfo;
}

pub trait WeightInfo {}
impl WeightInfo for () {}

decl_error! {
	pub enum Error for Module<T: Trait<I>, I: Instance> {
		/// TODO
		TODO,
	}
}

decl_storage! {
	trait Store for Module<T: Trait<I>, I: Instance = DefaultInstance> as DarwiniaRelayAuthorities {
		pub Authorities
			get(fn authority)
			: map hasher(blake2_128_concat) AccountId<T> => Signer<T, I>;
	}
}

decl_module! {
	pub struct Module<T: Trait<I>, I: Instance = DefaultInstance> for enum Call
	where
		origin: T::Origin
	{
		type Error = Error<T, I>;
	}
}

impl<T: Trait<I>, I: Instance> Module<T, I> {
	pub fn ensure_authority() -> DispatchResult {
		Ok(())
	}
}
