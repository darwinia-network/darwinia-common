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

//! # Dynamic Fee pallet
//!
//! The Dynamic Fee pallet use to adjust the gas price dynamically on chain.

#![cfg_attr(not(feature = "std"), no_std)]

#[frame_support::pallet]
pub mod pallet {
	// --- core ---
	use core::cmp;
	// --- paritytech ---
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_core::U256;
	use sp_inherents::IsFatalError;
	#[cfg(feature = "std")]
	use sp_inherents::{Error, InherentDataProvider as InherentDataProviderT};
	// --- darwinia-network ---
	use darwinia_evm::FeeCalculator;

	pub(super) type InherentType = U256;

	pub(super) const INHERENT_IDENTIFIER: InherentIdentifier = *b"dynfee0_";

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Min Gas Price adjust divisor
		type MinGasPriceBoundDivisor: Get<U256>;
	}

	#[pallet::storage]
	#[pallet::getter(fn min_gas_price)]
	pub(super) type MinGasPrice<T> = StorageValue<_, U256, ValueQuery, DefaultForMinGasPrice>;
	#[pallet::type_value]
	pub(super) fn DefaultForMinGasPrice() -> U256 {
		1_000_000_000_u128.into()
	}

	#[pallet::storage]
	pub(super) type TargetMinGasPrice<T> = StorageValue<_, U256, OptionQuery>;

	#[pallet::pallet]
	pub struct Pallet<T>(_);
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			<TargetMinGasPrice<T>>::kill();

			T::DbWeight::get().writes(1)
		}

		fn on_finalize(_: BlockNumberFor<T>) {
			if let Some(target) = <TargetMinGasPrice<T>>::get() {
				let bound =
					<MinGasPrice<T>>::get() / T::MinGasPriceBoundDivisor::get() + U256::one();
				let upper_limit = <MinGasPrice<T>>::get().saturating_add(bound);
				let lower_limit = <MinGasPrice<T>>::get().saturating_sub(bound);

				<MinGasPrice<T>>::set(cmp::min(upper_limit, cmp::max(lower_limit, target)));
			}
		}
	}
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set the Target gas price
		#[pallet::weight(T::DbWeight::get().writes(1))]
		fn note_min_gas_price_target(
			origin: OriginFor<T>,
			target: U256,
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;

			<TargetMinGasPrice<T>>::set(Some(target));

			Ok(().into())
		}
	}
	impl<T: Config> FeeCalculator for Pallet<T> {
		fn min_gas_price() -> U256 {
			<MinGasPrice<T>>::get()
		}
	}

	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T> {
		type Call = Call<T>;
		type Error = InherentError;

		const INHERENT_IDENTIFIER: InherentIdentifier = INHERENT_IDENTIFIER;

		fn create_inherent(data: &InherentData) -> Option<Self::Call> {
			let target = data.get_data::<InherentType>(&INHERENT_IDENTIFIER).ok()??;
			Some(Call::note_min_gas_price_target(target))
		}

		fn check_inherent(_call: &Self::Call, _data: &InherentData) -> Result<(), Self::Error> {
			Ok(())
		}

		fn is_inherent(_: &Self::Call) -> bool {
			true
		}
	}

	/// Errors that can occur while checking the price inherent
	#[derive(Encode, Decode, RuntimeDebug)]
	pub enum InherentError {}
	impl IsFatalError for InherentError {
		fn is_fatal_error(&self) -> bool {
			match *self {}
		}
	}

	/// Provide price inherent
	#[cfg(feature = "std")]
	pub struct InherentDataProvider(pub InherentType);
	#[cfg(feature = "std")]
	impl InherentDataProvider {
		pub fn from_target_gas_price(price: InherentType) -> Self {
			Self(price)
		}
	}
	#[cfg(feature = "std")]
	#[async_trait::async_trait]
	impl InherentDataProviderT for InherentDataProvider {
		fn provide_inherent_data(&self, inherent_data: &mut InherentData) -> Result<(), Error> {
			inherent_data.put_data(INHERENT_IDENTIFIER, &self.0)
		}

		async fn try_handle_error(
			&self,
			identifier: &InherentIdentifier,
			error: &[u8],
		) -> Option<Result<(), Error>> {
			if *identifier != INHERENT_IDENTIFIER {
				return None;
			}

			let error = InherentError::decode(&mut &error[..]).ok()?;
			Some(Err(Error::Application(Box::from(format!("{:?}", error)))))
		}
	}
}
pub use pallet::*;
