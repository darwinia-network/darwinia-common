//! Runtime API definition required by balances RPC extensions.
//!
//! This API should be imported and implemented by the runtime,
//! of a node that wants to use the custom RPC extension
//! adding balances access methods.

#![cfg_attr(not(feature = "std"), no_std)]

// --- crates ---
use codec::{Codec, Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};
// --- substrate ---
use sp_api::decl_runtime_apis;
use sp_runtime::traits::{MaybeDisplay, MaybeFromStr};

#[derive(Default, Eq, PartialEq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct RuntimeDispatchInfo<Balance> {
	#[cfg_attr(
		feature = "std",
		serde(bound(serialize = "Balance: std::fmt::Display"))
	)]
	#[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
	#[cfg_attr(
		feature = "std",
		serde(bound(deserialize = "Balance: std::str::FromStr"))
	)]
	#[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
	pub usable_balance: Balance,
}

#[cfg(feature = "std")]
fn serialize_as_string<S: Serializer, T: std::fmt::Display>(
	t: &T,
	serializer: S,
) -> Result<S::Ok, S::Error> {
	serializer.serialize_str(&t.to_string())
}

#[cfg(feature = "std")]
fn deserialize_from_string<'de, D: Deserializer<'de>, T: std::str::FromStr>(
	deserializer: D,
) -> Result<T, D::Error> {
	let s = String::deserialize(deserializer)?;
	s.parse::<T>()
		.map_err(|_| serde::de::Error::custom("Parse from string failed"))
}

decl_runtime_apis! {
	pub trait BalancesApi<AccountId, Balance>
	where
		AccountId: Codec,
		Balance: Codec + MaybeDisplay + MaybeFromStr,
	{
		fn usable_balance(who: AccountId) -> RuntimeDispatchInfo<Balance>;
	}
}
