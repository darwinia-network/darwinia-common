//! # Crab Backing Module

#![cfg_attr(not(feature = "std"), no_std)]

mod types {
	// --- darwinia ---
	use crate::*;

	pub type AccountId<T> = <T as frame_system::Trait>::AccountId;

	pub type RingBalance<T> = <RingCurrency<T> as Currency<AccountId<T>>>::Balance;

	type RingCurrency<T> = <T as Trait>::RingCurrency;
}

// --- substrate ---
use frame_support::{
	decl_module, decl_storage,
	traits::{Currency, Get},
};
use sp_runtime::{traits::AccountIdConversion, ModuleId};
// --- darwinia ---
use types::*;

pub trait Trait: frame_system::Trait {
	type ModuleId: Get<ModuleId>;

	type RingCurrency: Currency<AccountId<Self>>;

	type WeightInfo: WeightInfo;
}

pub trait WeightInfo {}
impl WeightInfo for () {}

decl_storage! {
	trait Store for Module<T: Trait> as DarwiniaCrabBacking {}

	add_extra_genesis {
		config(backed_ring): RingBalance<T>;
		build(|config| {
			let _ = T::RingCurrency::make_free_balance_be(
				&<Module<T>>::account_id(),
				T::RingCurrency::minimum_balance() + config.backed_ring
			);
		});
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call
	where
		origin: T::Origin
	{}
}

impl<T: Trait> Module<T> {
	pub fn account_id() -> T::AccountId {
		T::ModuleId::get().into_account()
	}
}
