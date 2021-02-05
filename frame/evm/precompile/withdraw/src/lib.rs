#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::traits::{Currency, ExistenceRequirement};
use sp_core::U256;
use sp_runtime::traits::UniqueSaturatedInto;
use sp_std::marker::PhantomData;
use sp_std::prelude::*;
use sp_std::vec::Vec;

use codec::Decode;
use darwinia_evm::{AddressMapping, Trait};
use darwinia_evm_primitives::Precompile;
use evm::{Context, ExitError, ExitSucceed};

type AccountId<T> = <T as frame_system::Trait>::AccountId;

// WithDraw Precompile Contract, used to withdraw balance from evm account to darwinia account
// address: 0000000000000000000000000000000000000005
pub struct WithDraw<T: Trait> {
	_maker: PhantomData<T>,
}

impl<T: Trait> Precompile for WithDraw<T> {
	fn execute(
		input: &[u8],
		_: Option<usize>,
		context: &Context,
	) -> core::result::Result<(ExitSucceed, Vec<u8>, usize), ExitError> {
		// Decode input data
		let input = InputData::<T>::decode(&input)?;

		let helper = U256::from(10)
			.checked_pow(U256::from(9))
			.unwrap_or(U256::MAX);
		let value = input.value.saturating_mul(helper);
		let from_address = T::AddressMapping::into_account_id(context.caller);

		let result = T::Currency::transfer(
			&from_address,
			&input.dest,
			value.low_u128().unique_saturated_into(),
			ExistenceRequirement::AllowDeath,
		);

		match result {
			Ok(()) => Ok((ExitSucceed::Returned, vec![], 10000)),
			Err(error) => match error {
				sp_runtime::DispatchError::BadOrigin => Err(ExitError::Other("BadOrigin".into())),
				sp_runtime::DispatchError::CannotLookup => {
					Err(ExitError::Other("CannotLookup".into()))
				}
				sp_runtime::DispatchError::Other(message) => Err(ExitError::Other(message.into())),
				sp_runtime::DispatchError::Module { message, .. } => {
					Err(ExitError::Other(message.unwrap_or("Module Error").into()))
				}
			},
		}
	}
}

#[derive(Debug, PartialEq, Eq)]
pub struct InputData<T: frame_system::Trait> {
	pub dest: AccountId<T>,
	pub value: U256,
}

impl<T: frame_system::Trait> InputData<T> {
	pub fn decode(data: &[u8]) -> Result<Self, ExitError> {
		if data.len() == 96 {
			let mut dest_bytes = [0u8; 32];
			dest_bytes.copy_from_slice(&data[32..64]);

			let mut value_bytes = [0u8; 32];
			value_bytes.copy_from_slice(&data[64..96]);

			return Ok(InputData {
				dest: <T as frame_system::Trait>::AccountId::decode(&mut dest_bytes.as_ref())
					.map_err(|_| ExitError::Other("Invalid destination address".into()))?,
				value: U256::from_little_endian(&value_bytes),
			});
		}
		Err(ExitError::Other("Invalid input data length".into()))
	}
}
