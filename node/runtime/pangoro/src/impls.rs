//! Some configurable implementations as associated type for the substrate runtime.

// --- paritytech ---
use sp_runtime::RuntimeDebug;
// --- darwinia-network ---
use crate::*;

darwinia_support::impl_account_data! {
	struct AccountData<Balance>
	for
		RingInstance,
		KtonInstance
	where
		Balance = Balance
	{
		// other data
	}
}
