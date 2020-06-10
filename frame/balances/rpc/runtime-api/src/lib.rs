//! Runtime API definition required by balances RPC extensions.
//!
//! This API should be imported and implemented by the runtime,
//! of a node that wants to use the custom RPC extension
//! adding balances access methods.

#![cfg_attr(not(feature = "std"), no_std)]

// --- crates ---
use codec::{Codec, Decode, Encode};
// --- substrate ---
use sp_api::decl_runtime_apis;
use sp_runtime::traits::{MaybeDisplay, MaybeFromStr};
// --- darwinia ---
use darwinia_support::impl_runtime_dispatch_info;

impl_runtime_dispatch_info! {
	struct RuntimeDispatchInfo<Balance> {
		usable_balance: Balance
	}
}

decl_runtime_apis! {
	pub trait BalancesApi<AccountId, Balance>
	where
		AccountId: Codec,
		Balance: Codec + MaybeDisplay + MaybeFromStr,
	{
		fn usable_balance(instance: u8, who: AccountId) -> RuntimeDispatchInfo<Balance>;
	}
}
