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

darwinia_support::impl_runtime_dispatch_info! {
	struct RuntimeDispatchInfo<Power> {
		power: Power
	}
}

decl_runtime_apis! {
	pub trait StakingApi<AccountId, Power>
	where
		AccountId: Codec,
		Power: Codec + MaybeDisplay + MaybeFromStr,
	{
		fn power_of(who: AccountId) -> RuntimeDispatchInfo<Power>;
	}
}
