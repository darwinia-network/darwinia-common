
use pallet_evm::{Precompile, ExitError, ExitSucceed,AddressMapping};
use sp_core::{H160, Hasher};
use sp_runtime::{
    traits::{BlakeTwo256, IdentifyAccount, Verify},
    MultiSignature,
};
use frame_support::traits::ExistenceRequirement;


pub type Ring = darwinia_balances::Module<_, darwinia_balances::Instance0>;
pub type Signature = MultiSignature;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

pub struct ConcatAddressMapping;

impl AddressMapping<AccountId> for ConcatAddressMapping {
	fn into_account_id(address: H160) -> AccountId {
		let mut data = [0u8; 32];
		data[0..4].copy_from_slice(b"dvm:");
		data[11..31].copy_from_slice(&address[..]);
		let checksum: u8 = data[1..31].iter().fold(data[0], |mut sum, &byte| {
			sum = sum ^ byte;
			sum
		});
		data[31] = checksum;
		AccountId::from(data)
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
pub struct Withdraw;

impl Precompile for Withdraw {
	fn execute(
		input: &[u8],
		target_gas: Option<usize>,
	) -> core::result::Result<(ExitSucceed, Vec<u8>, usize), ExitError> {
		if input.len() != 0x80 {
			Err(ExitError::Other("InputDataLenErr"))
		} else {
			let cost = ensure_linear_cost(target_gas, input.len(), 60, 12)?;

			let from = ConcatAddressMapping::into_account_id(H160::from_slice(&input[44..64]));

			let mut to_data = [0u8; 32];
			to_data.copy_from_slice(&input[64..96]);
			let to = AccountId::from(to_data);

			let mut value_data = [0u8; 16];
			value_data.copy_from_slice(&input[112..128]);
			let value = u128::from_be_bytes(value_data);

			let result = <Ring as frame_support::traits::Currency<_>>::transfer(
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