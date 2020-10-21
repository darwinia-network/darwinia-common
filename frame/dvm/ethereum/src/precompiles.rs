use crate::Trait;
use frame_support::traits::{Currency, ExistenceRequirement};
use sp_core::H160;
use sp_runtime::{traits::UniqueSaturatedFrom, AccountId32};
use sp_std::marker::PhantomData;
use sp_std::prelude::*;

pub use codec::Decode;
use pallet_evm::{AddressMapping, ExitError, ExitSucceed, Precompile};

pub struct ConcatAddressMapping;

/// The ConcatAddressMapping used for transfer from evm 20-length to substrate 32-length address
/// The concat rule inclued three parts:
/// 1. AccountId Prefix: concat("dvm", "0x00000000000000"), length: 11 byetes
/// 2. Evm address: the original evm address, length: 20 bytes
/// 3. CheckSum:  byte_xor(AccountId Prefix + Evm address), length: 1 bytes
impl AddressMapping<AccountId32> for ConcatAddressMapping {
	fn into_account_id(address: H160) -> AccountId32 {
		let mut data = [0u8; 32];
		data[0..4].copy_from_slice(b"dvm:");
		data[11..31].copy_from_slice(&address[..]);
		let checksum: u8 = data[1..31].iter().fold(data[0], |sum, &byte| {
			sum ^ byte
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

impl<T: Trait> Precompile for NativeTransfer<T> {
	fn execute(
		input: &[u8],
		target_gas: Option<usize>,
	) -> core::result::Result<(ExitSucceed, Vec<u8>, usize), ExitError> {
		if input.len() != 0x80 {
			Err(ExitError::Other("InputDataLenErr"))
		} else {
			let cost = ensure_linear_cost(target_gas, input.len(), 60, 12)?;

			let from =
				<T as Trait>::AddressMapping::into_account_id(H160::from_slice(&input[44..64]));

			let mut to_data = [0u8; 32];
			to_data.copy_from_slice(&input[64..96]);
			let to = AccountId32::from(to_data);
			let to = <T as frame_system::Trait>::AccountId::decode(&mut to.as_ref()).unwrap();

			let mut value_data = [0u8; 16];
			value_data.copy_from_slice(&input[112..128]);
			let value = u128::from_be_bytes(value_data);
			let value  = <<T as Trait>::RingCurrency as Currency<<T as frame_system::Trait>::AccountId>>::Balance::unique_saturated_from(value);

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
