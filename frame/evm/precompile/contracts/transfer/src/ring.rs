use crate::AccountId;
use codec::Decode;
use darwinia_evm::{AddressMapping, Config};
use darwinia_support::evm::POW_9;
use evm::{Context, ExitError, ExitSucceed};
use frame_support::traits::{Currency, ExistenceRequirement};
use sp_core::U256;
use sp_runtime::traits::UniqueSaturatedInto;
use sp_std::{marker::PhantomData, prelude::*, vec::Vec};

pub struct RingBack<T: Config> {
	_maker: PhantomData<T>,
}

impl<T: Config> RingBack<T> {
	/// The Withdraw process is divided into two part:
	/// 1. parse the withdrawal address from the input parameter and get the contract address and value from the context
	/// 2. transfer from the contract address to withdrawal address
	///
	/// Input data: 32-bit substrate withdrawal public key
	pub(crate) fn transfer(
		input: &[u8],
		_: Option<u64>,
		context: &Context,
	) -> core::result::Result<(ExitSucceed, Vec<u8>, u64), ExitError> {
		// Decode input data
		let input = InputData::<T>::decode(&input)?;

		let helper = U256::from(POW_9);
		let contract_address = T::AddressMapping::into_account_id(context.address);
		let context_value = context.apparent_value.div_mod(helper).0;
		let context_value = context_value.low_u128().unique_saturated_into();

		let result = T::RingCurrency::transfer(
			&contract_address,
			&input.dest,
			context_value,
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
				_ => Err(ExitError::Other("Module Error".into())),
			},
		}
	}
}

#[derive(Debug, PartialEq, Eq)]
pub struct InputData<T: frame_system::Config> {
	pub dest: AccountId<T>,
}

impl<T: frame_system::Config> InputData<T> {
	pub fn decode(data: &[u8]) -> Result<Self, ExitError> {
		if data.len() == 32 {
			let mut dest_bytes = [0u8; 32];
			dest_bytes.copy_from_slice(&data[0..32]);

			return Ok(InputData {
				dest: <T as frame_system::Config>::AccountId::decode(&mut dest_bytes.as_ref())
					.map_err(|_| ExitError::Other("Invalid destination address".into()))?,
			});
		}
		Err(ExitError::Other("Invalid input data length".into()))
	}
}
