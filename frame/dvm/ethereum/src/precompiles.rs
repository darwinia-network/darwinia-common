
use pallet_evm::{Precompile, ExitError, ExitSucceed, AddressMapping};
use sp_core::{H160, Hasher};
use sp_runtime::{
    traits::{BlakeTwo256, IdentifyAccount, Verify},
    MultiSignature,
};
use frame_support::traits::ExistenceRequirement;
use sp_std::marker::PhantomData;
pub use codec::Decode;

use crate::*;
use sp_runtime::AccountId32;
pub type Signature = MultiSignature;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
use sp_runtime::traits::UniqueSaturatedFrom;

pub struct ConcatAddressMapping;

impl AddressMapping<AccountId32> for ConcatAddressMapping {
	fn into_account_id(address: H160) -> AccountId32 {
		let mut data = [0u8; 32];
		data[0..4].copy_from_slice(b"dvm:");
		data[11..31].copy_from_slice(&address[..]);
		let checksum: u8 = data[1..31].iter().fold(data[0], |mut sum, &byte| {
			sum = sum ^ byte;
			sum
		});
		data[31] = checksum;
		AccountId32::from(data)
	}
}

fn ensure_linear_cost(
	target_gas: Option<usize>,
	len: usize,
	base: usize,
	word: usize,
) -> Result<usize, ExitError> {
	let cost = base
		.checked_add(
			word.checked_mul(len.saturating_add(31) / 32)
				.ok_or(ExitError::OutOfGas)?,
		)
		.ok_or(ExitError::OutOfGas)?;

	if let Some(target_gas) = target_gas {
		if cost > target_gas {
			return Err(ExitError::OutOfGas);
		}
	}

	Ok(cost)
}

/// Precompile for withdrawing from evm address
pub struct NativeTransfer<T>(PhantomData<T>);

impl<T: Trait + frame_system::Trait> Precompile for NativeTransfer<T> {
	fn execute(
		input: &[u8],
		target_gas: Option<usize>,
	) -> core::result::Result<(ExitSucceed, Vec<u8>, usize), ExitError> {
		if input.len() != 0x80 {
			Err(ExitError::Other("InputDataLenErr"))
		} else {
			let cost = ensure_linear_cost(target_gas, input.len(), 60, 12)?;

			let from = <T as Trait>::AddressMapping::into_account_id(H160::from_slice(&input[44..64]));

			let mut to_data = [0u8; 32];
			to_data.copy_from_slice(&input[64..96]);
			let to = AccountId32::from(to_data);
			let to = <T as frame_system::Trait>::AccountId::decode(&mut to.as_ref()).unwrap();

			let mut value_data = [0u8; 16];
			value_data.copy_from_slice(&input[112..128]);
			let value = u128::from_be_bytes(value_data);
			let value  = <<T as Trait>::RingCurrency as Currency<T::AccountId>>::Balance::unique_saturated_from(value);

			let result = <T as Trait>::RingCurrency::transfer(
				&from,
				&to,
				value,
				ExistenceRequirement::KeepAlive,
			);
			match result {
				Ok(()) => Ok((ExitSucceed::Returned, vec![], cost)),
				Err(error) => match error {
					sp_runtime::DispatchError::BadOrigin => Err(ExitError::Other("BadOrigin")),
					sp_runtime::DispatchError::CannotLookup => {
						Err(ExitError::Other("CannotLookup"))
					}
					sp_runtime::DispatchError::Other(message) => Err(ExitError::Other(message)),
					sp_runtime::DispatchError::Module { message, .. } => {
						Err(ExitError::Other(message.unwrap()))
					}
				},
			}
		}
	}
}
