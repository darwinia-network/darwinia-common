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
	pub type AccountId<T> = <T as frame_system::Trait>::AccountId;
}

// --- substrate ---
use frame_support::{decl_error, decl_event, decl_module, decl_storage};
// --- darwinia ---
use types::*;

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

	type WeightInfo: WeightInfo;
}

pub trait WeightInfo {}
impl WeightInfo for () {}

decl_event!(
	pub enum Event<T>
	where
		AccountId = AccountId<T>,
	{
		TODO(AccountId),
	}
);

decl_error! {
	pub enum Error for Module<T: Trait> {
		/// TODO
		TODO,
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as DarwiniaEcdsaAuthorities {
		pub Authorities
			get(fn authority)
			: map hasher(blake2_128_concat) AccountId<T> => u8;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call
	where
		origin: T::Origin
	{
		type Error = Error<T>;
	}
}
