//! # Ecdsa Authorities Module

#![cfg_attr(not(feature = "std"), no_std)]

// --- substrate ---
use frame_support::{decl_error, decl_event, decl_module, decl_storage};

pub trait Trait: frame_system::Trait {}

pub trait WeightInfo {}
impl WeightInfo for () {}

decl_event!(
	pub enum Event<T>
	where
		<T as frame_system::Trait>::AccountId,
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
	trait Store for Module<T: Trait> as DarwiniaEcdsaAuthorities {}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call
	where
		origin: T::Origin
	{
		type Error = Error<T>;
	}
}
