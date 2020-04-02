// --- darwinia ---
use crate::*;

darwinia_support::impl_account_data! {
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
