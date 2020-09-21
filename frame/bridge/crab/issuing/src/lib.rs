//! # Crab Issuing Module

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "genesis-loader")]
pub mod genesis_loader {
	// --- crate ---
	use serde::{Deserialize, Serialize};

	#[derive(Debug, Default, Serialize, Deserialize)]
	pub struct Account {
		pub address: String,
		pub mapped_ring: u128,
	}
	darwinia_support::impl_genesis! {
		struct MappedRingLoader {
			mapped_rings: Vec<Account>
		}
	}

	#[test]
	fn genesis_loader_should_work() {
		let mapped_ring_loader = MappedRingLoader::from_str(
			r#"{
				"mapped_rings": [
					{
						"address": "0xb4f7f03bebc56ebe96bc52ea5ed3159d45a0ce3a8d7f082983c33ef133274747",
						"mapped_ring": 2000000000000000000
					}
				]
			}"#,
		);

		eprintln!("{:#?}", mapped_ring_loader);
	}
}
#[cfg(feature = "genesis-loader")]
pub use genesis_loader::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

mod types {
	// --- darwinia ---
	use crate::*;

	pub type MappedRing = u128;

	pub type AccountId<T> = <T as frame_system::Trait>::AccountId;

	pub type RingBalance<T> = <RingCurrency<T> as Currency<AccountId<T>>>::Balance;

	type RingCurrency<T> = <T as Trait>::RingCurrency;
}

// --- substrate ---
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage, ensure,
	traits::{Currency, ExistenceRequirement, Get},
};
use frame_system::{ensure_root, ensure_signed};
use sp_runtime::{traits::AccountIdConversion, ModuleId, SaturatedConversion};
// --- darwinia ---
use types::*;

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

	type ModuleId: Get<ModuleId>;

	type RingCurrency: Currency<AccountId<Self>>;

	type WeightInfo: WeightInfo;
}

pub trait WeightInfo {}
impl WeightInfo for () {}

decl_event! {
	pub enum Event<T>
	where
		AccountId = AccountId<T>,
		RingBalance = RingBalance<T>,
	{
		/// Someone swapped some *CRING*. [who, swapped *CRING*, burned Mapped *RING*]
		SwapAndBurnToGenesis(AccountId, RingBalance, MappedRing),
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
			/// Genesis Swap - CLOSED
			GenesisSwapC,
			/// Swap Amount - TOO LOW
			SwapAmountTL,
			/// Backed *RING* - INSUFFICIENT
			BackedRingIS,
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as DarwiniaCrabIssuing {
		pub GenesisSwapOpen get(fn genesis_swap_open): bool = true;

		pub TotalMappedRing get(fn total_mapped_ring) config(): MappedRing;
	}

	add_extra_genesis {
		build(|config| {
			let _ = T::RingCurrency::make_free_balance_be(
				&<Module<T>>::account_id(),
				T::RingCurrency::minimum_balance(),
			);

			TotalMappedRing::put(config.total_mapped_ring);
		});
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call
	where
		origin: T::Origin
	{
		type Error = Error<T>;

		fn deposit_event() = default;

		#[weight = T::DbWeight::get().reads_writes(3, 1) + 100_000_000]
		pub fn swap_and_burn_to_genesis(origin, amount: RingBalance<T>) {
			let who = ensure_signed(origin)?;

			ensure!(Self::genesis_swap_open(), <Error<T>>::GenesisSwapC);

			let burned = amount.saturated_into() / 100;

			ensure!(burned > 0, <Error<T>>::SwapAmountTL);

			let backed = Self::total_mapped_ring();

			ensure!(backed >= burned, <Error<T>>::BackedRingIS);

			T::RingCurrency::transfer(&who, &Self::account_id(), amount, ExistenceRequirement::AllowDeath)?;
			TotalMappedRing::put(backed - burned);

			Self::deposit_event(RawEvent::SwapAndBurnToGenesis(who, amount, burned));
		}

		#[weight = 100_000_000]
		pub fn set_genesis_swap_status(origin, status: bool) {
			ensure_root(origin)?;

			GenesisSwapOpen::put(status);
		}
	}
}

impl<T: Trait> Module<T> {
	pub fn account_id() -> T::AccountId {
		T::ModuleId::get().into_account()
	}
}
