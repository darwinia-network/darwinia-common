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
	// --- crates.io ---
	use scale_info::TypeInfo;
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
		/// Min Gas Price adjust divisor.
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
			if let Some(target) = <TargetMinGasPrice<T>>::take() {
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
		/// Set the target gas price.
		#[pallet::weight((T::DbWeight::get().writes(1), DispatchClass::Mandatory))]
		pub fn note_min_gas_price_target(
			origin: OriginFor<T>,
			target: U256,
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;
			// When a block author create a fake block with multiple noting, then other validators will reject that block because of failed import block verification.
			assert!(
				<TargetMinGasPrice<T>>::get().is_none(),
				"TargetMinGasPrice must be updated only once in the block"
			);

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
			Some(Call::note_min_gas_price_target { target })
		}

		fn check_inherent(_call: &Self::Call, _data: &InherentData) -> Result<(), Self::Error> {
			Ok(())
		}

		fn is_inherent(_: &Self::Call) -> bool {
			true
		}
	}

	/// Errors that can occur while checking the price inherent.
	#[derive(Encode, Decode, RuntimeDebug, TypeInfo)]
	pub enum InherentError {}
	impl IsFatalError for InherentError {
		fn is_fatal_error(&self) -> bool {
			match *self {}
		}
	}

	/// Provide price inherent.
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

#[cfg(test)]
mod tests {
	use super::*;
	use crate as dvm_dynamic_fee;

	use frame_support::{
		assert_ok, parameter_types,
		traits::{Everything, OnFinalize, OnInitialize},
	};
	use sp_core::{H256, U256};
	use sp_io::TestExternalities;
	use sp_runtime::{
		testing::Header,
		traits::{BlakeTwo256, IdentityLookup},
	};

	pub fn new_test_ext() -> TestExternalities {
		let t = frame_system::GenesisConfig::default()
			.build_storage::<Test>()
			.unwrap();
		TestExternalities::new(t)
	}

	type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
	type Block = frame_system::mocking::MockBlock<Test>;

	parameter_types! {
		pub const BlockHashCount: u64 = 250;
		pub BlockWeights: frame_system::limits::BlockWeights =
			frame_system::limits::BlockWeights::simple_max(1024);
	}
	impl frame_system::Config for Test {
		type BaseCallFilter = Everything;
		type BlockWeights = ();
		type BlockLength = ();
		type DbWeight = ();
		type Origin = Origin;
		type Index = u64;
		type BlockNumber = u64;
		type Call = Call;
		type Hash = H256;
		type Hashing = BlakeTwo256;
		type AccountId = u64;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Header = Header;
		type Event = Event;
		type BlockHashCount = BlockHashCount;
		type Version = ();
		type PalletInfo = PalletInfo;
		type AccountData = ();
		type OnNewAccount = ();
		type OnKilledAccount = ();
		type SystemWeightInfo = ();
		type SS58Prefix = ();
		type OnSetCode = ();
	}

	frame_support::parameter_types! {
		pub const MinimumPeriod: u64 = 1000;
	}
	impl pallet_timestamp::Config for Test {
		type Moment = u64;
		type OnTimestampSet = ();
		type MinimumPeriod = MinimumPeriod;
		type WeightInfo = ();
	}

	frame_support::parameter_types! {
		pub BoundDivision: U256 = 1024.into();
	}
	impl Config for Test {
		type MinGasPriceBoundDivisor = BoundDivision;
	}

	frame_support::construct_runtime!(
		pub enum Test where
			Block = Block,
			NodeBlock = Block,
			UncheckedExtrinsic = UncheckedExtrinsic,
		{
			System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
			Timestamp: pallet_timestamp::{Pallet, Call, Storage},
			DynamicFee: dvm_dynamic_fee::{Pallet, Call, Storage, Inherent},
		}
	);

	fn run_to_block(n: u64) {
		while System::block_number() < n {
			DynamicFee::on_finalize(System::block_number());
			System::set_block_number(System::block_number() + 1);
			DynamicFee::on_initialize(System::block_number());
		}
	}

	#[test]
	#[should_panic(expected = "TargetMinGasPrice must be updated only once in the block")]
	fn double_set_in_a_block_failed() {
		new_test_ext().execute_with(|| {
			run_to_block(3);
			assert_ok!(DynamicFee::note_min_gas_price_target(
				Origin::none(),
				U256::zero()
			));
			let _ = DynamicFee::note_min_gas_price_target(Origin::none(), U256::zero());
			run_to_block(4);
			assert_ok!(DynamicFee::note_min_gas_price_target(
				Origin::none(),
				U256::zero()
			));
		});
	}
}
