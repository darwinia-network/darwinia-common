use crate::*;
use pallet_support::{
	balance::{lock::*, *},
	impl_account_data,
};

impl_account_data! {
	pub struct AccountData<Balance>
	for
		RingInstance,
		KtonInstance
	where
		Balance = u128
	{
		// other data
	}
}
