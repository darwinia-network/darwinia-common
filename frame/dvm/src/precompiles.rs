pub use codec::Decode;
use darwinia_evm::{AddressMapping, ExitError, ExitSucceed, Precompile};
use frame_support::traits::ExistenceRequirement;
use sp_core::H160;
use sp_runtime::traits::{BlakeTwo256, Hash};
use sp_std::marker::PhantomData;
use sp_std::prelude::*;

use crate::*;
use sp_runtime::traits::UniqueSaturatedFrom;
use sp_runtime::AccountId32;

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
		let checksum: u8 = data[1..31].iter().fold(data[0], |sum, &byte| sum ^ byte);
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