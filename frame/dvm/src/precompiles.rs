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

/// Precompile for withdrawing from evm address
// pub struct NativeTransfer<T>(PhantomData<T>);

// impl<T: Trait> Precompile for NativeTransfer<T> {
// 	fn execute(
// 		input: &[u8],
// 		target_gas: Option<usize>,
// 	) -> core::result::Result<(ExitSucceed, Vec<u8>, usize), ExitError> {
// 		if input.len() != 177 {
// 			Err(ExitError::Other("InputDataLenErr".into()))
// 		} else {
// 			let cost = ensure_linear_cost(target_gas, input.len(), 60, 12)?;

// 			let params = decode_params(input);
// 			// from account id
// 			let from =
// 				<T as Trait>::AddressMapping::into_account_id(H160::from_slice(&params.from));
// 			// to account id
// 			let to = AccountId32::from(params.to);
// 			let to = <T as frame_system::Trait>::AccountId::decode(&mut to.as_ref()).unwrap();
// 			// value
// 			let value = u128::from_be_bytes(params.value);
// 			let value  = <<T as Trait>::RingCurrency as Currency<T::AccountId>>::Balance::unique_saturated_from(value);

// 			if params.is_valid_signature() {
// 				let result = <T as Trait>::RingCurrency::transfer(
// 					&from,
// 					&to,
// 					value,
// 					ExistenceRequirement::KeepAlive,
// 				);
// 				match result {
// 					Ok(()) => Ok((ExitSucceed::Returned, vec![], cost)),
// 					Err(error) => match error {
// 						sp_runtime::DispatchError::BadOrigin => {
// 							Err(ExitError::Other("BadOrigin".into()))
// 						}
// 						sp_runtime::DispatchError::CannotLookup => {
// 							Err(ExitError::Other("CannotLookup".into()))
// 						}
// 						sp_runtime::DispatchError::Other(message) => {
// 							Err(ExitError::Other(message.into()))
// 						}
// 						sp_runtime::DispatchError::Module { message, .. } => {
// 							Err(ExitError::Other(message.unwrap_or("Module Error").into()))
// 						}
// 					},
// 				}
// 			} else {
// 				Err(ExitError::Other("BadSignature".into()))
// 			}
// 		}
// 	}
// }

struct Params {
	from: [u8; 20],
	to: [u8; 32],
	value: [u8; 16],
	signature: [u8; 65],
}

impl Params {
	pub fn is_valid_signature(&self) -> bool {
		let mut data = [0u8; 68];
		data[0..20].copy_from_slice(&self.from);
		data[20..52].copy_from_slice(&self.to);
		data[52..68].copy_from_slice(&self.value);
		let hash = BlakeTwo256::hash_of(&data);

		match recover_signer(self.signature, hash.to_fixed_bytes()) {
			Some(signer) => signer[..] == self.from,
			None => false,
		}
	}
}

fn decode_params(input: &[u8]) -> Params {
	let offset = 44;

	let mut from = [0u8; 20];
	from.copy_from_slice(&input[offset..offset + 20]);

	let mut to = [0u8; 32];
	to.copy_from_slice(&input[offset + 20..offset + 52]);

	let mut value = [0u8; 16];
	value.copy_from_slice(&input[offset + 52..offset + 68]);

	let mut signature = [0u8; 65];
	signature.copy_from_slice(&input[offset + 68..offset + 133]);

	Params {
		from,
		to,
		value,
		signature,
	}
}

fn recover_signer(sig: [u8; 65], msg: [u8; 32]) -> Option<H160> {
	match sp_io::crypto::secp256k1_ecdsa_recover(&sig, &msg) {
		Ok(pubkey) => Some(H160::from(H256::from_slice(
			Keccak256::digest(&pubkey).as_slice(),
		))),
		Err(_) => None,
	}
}

#[cfg(test)]
mod tests {
	use crate::*;
	use ethereum::TransactionSignature;

	#[cfg(test)]
	fn sign(hash: H256, key: H256) -> [u8; 65] {
		let msg = secp256k1::Message::parse(hash.as_fixed_bytes());
		let s = secp256k1::sign(&msg, &secp256k1::SecretKey::parse_slice(&key[..]).unwrap());
		let sig = s.0.serialize();
		let tran_sig = TransactionSignature::new(
			s.1.serialize() as u64 % 2 + 42 * 2 + 35,
			H256::from_slice(&sig[0..32]),
			H256::from_slice(&sig[32..64]),
		)
		.unwrap();

		let mut res: [u8; 65] = [0u8; 65];
		res[0..32].copy_from_slice(&tran_sig.r()[..]);
		res[32..64].copy_from_slice(&tran_sig.s()[..]);
		res[64] = tran_sig.standard_v();
		res
	}

	#[test]
	fn test_concat_address_mapping() {
		use crate::precompiles::ConcatAddressMapping;
		// 0x182c00A789A7cC6BeA8fbc627121022D6029a416
		let evm_address = [
			24u8, 44, 0, 167, 137, 167, 204, 107, 234, 143, 188, 98, 113, 33, 2, 45, 96, 41, 164,
			22,
		];

		// same evm address's result should be equal
		let account_id = ConcatAddressMapping::into_account_id(H160::from_slice(&evm_address));
		let account_id_2 = ConcatAddressMapping::into_account_id(H160::from_slice(&evm_address));
		assert_eq!(account_id, account_id_2);

		// the prefix should equal to the original evm address
		let account_id: &[u8] = &account_id.as_ref();
		let account_id_part_1 = &account_id[11..31];
		assert_eq!(account_id_part_1, &evm_address);
	}

	#[test]
	fn test_signature() {
		use super::{AccountId32, BlakeTwo256};
		use sp_runtime::traits::Hash;

		// from
		let private_key = H256::from_slice(&[(8 + 1) as u8; 32]);
		let secret_key = secp256k1::SecretKey::parse_slice(&private_key[..]).unwrap();
		let public_key = &secp256k1::PublicKey::from_secret_key(&secret_key).serialize()[1..65];
		let from = H160::from(H256::from_slice(&Keccak256::digest(public_key)[..])).0;

		// to
		let account: AccountId32 =
			hex_literal::hex!["d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"]
				.into();
		let to: &[u8] = &account.as_ref();

		// value = 10
		let value: [u8; 16] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10];

		// signature
		let mut data = [0u8; 68];
		data[0..20].copy_from_slice(&from);
		data[20..52].copy_from_slice(to);
		data[52..68].copy_from_slice(&value);
		let hash = BlakeTwo256::hash_of(&data);
		let signature = sign(hash, private_key);

		let mut input = [0u8; 177];
		input[44..64].copy_from_slice(&from);
		input[64..96].copy_from_slice(to);
		input[96..112].copy_from_slice(&value);
		input[112..177].copy_from_slice(&signature);

		let param = super::decode_params(&input);
		assert_eq!(param.from, from);
		assert_eq!(param.to, to);
		assert_eq!(param.value, value);
		assert_eq!(param.is_valid_signature(), true);
	}
}
