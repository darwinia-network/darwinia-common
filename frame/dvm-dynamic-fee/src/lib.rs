// SPDX-License-Identifier: Apache-2.0
// This file is part of Frontier.
//
// Copyright (c) 2021 Parity Technologies (UK) Ltd.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
	decl_event, decl_module, decl_storage,
	inherent::{InherentData, InherentIdentifier, IsFatalError, ProvideInherent},
	traits::Get,
	weights::Weight,
};
use frame_system::ensure_none;
use sp_core::U256;
#[cfg(feature = "std")]
use sp_inherents::Error;
use sp_runtime::RuntimeDebug;
use sp_std::{
	cmp::{max, min},
	result,
};

pub trait Config: frame_system::Config {
	/// The overarching event type.
	type Event: From<Event> + Into<<Self as frame_system::Config>::Event>;
	/// Bound divisor for min gas price.
	type MinGasPriceBoundDivisor: Get<U256>;
}

decl_storage! {
	trait Store for Module<T: Config> as DynamicFee {
		MinGasPrice get(fn min_gas_price) config(): U256 = U256::from(1_000_000_000u128);
		TargetMinGasPrice: Option<U256>;
	}
}

decl_event!(
	pub enum Event {
		TargetMinGasPriceSet(U256),
	}
);

decl_module! {
	pub struct Module<T: Config> for enum Call where origin: T::Origin {
		fn deposit_event() = default;

		fn on_initialize(_block_number: T::BlockNumber) -> Weight {
			TargetMinGasPrice::kill();
			T::DbWeight::get().writes(1)
		}

		fn on_finalize(_block_number: T::BlockNumber) {
			if let Some(target) = TargetMinGasPrice::get() {
				let bound = MinGasPrice::get() / T::MinGasPriceBoundDivisor::get() + U256::one();

				let upper_limit = MinGasPrice::get().saturating_add(bound);
				let lower_limit = MinGasPrice::get().saturating_sub(bound);

				MinGasPrice::set(min(upper_limit, max(lower_limit, target)));
			}
		}

		#[weight = T::DbWeight::get().writes(1)]
		fn note_min_gas_price_target(
			origin,
			target: U256,
		) {
			ensure_none(origin)?;
			TargetMinGasPrice::set(Some(target));
		}
	}
}

impl<T: Config> darwinia_evm::FeeCalculator for Module<T> {
	fn min_gas_price() -> U256 {
		MinGasPrice::get()
	}
}

#[derive(Encode, Decode, RuntimeDebug)]
pub enum InherentError {}

impl IsFatalError for InherentError {
	fn is_fatal_error(&self) -> bool {
		match *self {}
	}
}

pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"dynfee0_";

pub type InherentType = U256;

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
impl sp_inherents::InherentDataProvider for InherentDataProvider {
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

impl<T: Config> ProvideInherent for Module<T> {
	type Call = Call<T>;
	type Error = InherentError;
	const INHERENT_IDENTIFIER: InherentIdentifier = INHERENT_IDENTIFIER;

	fn create_inherent(data: &InherentData) -> Option<Self::Call> {
		let target = data.get_data::<InherentType>(&INHERENT_IDENTIFIER).ok()??;

		Some(Call::note_min_gas_price_target(target))
	}

	fn check_inherent(_call: &Self::Call, _data: &InherentData) -> result::Result<(), Self::Error> {
		Ok(())
	}

	fn is_inherent(_: &Self::Call) -> bool {
		true
	}
}
