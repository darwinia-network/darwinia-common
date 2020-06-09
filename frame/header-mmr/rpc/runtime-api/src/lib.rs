//! Runtime API definition required by header-mmr RPC extensions.
//!
//! This API should be imported and implemented by the runtime,
//! of a node that wants to use the custom RPC extension
//! adding header-mmr access methods.

#![cfg_attr(not(feature = "std"), no_std)]

// --- crates ---
use codec::{Codec, Decode, Encode};
// --- substrate ---
use sp_api::decl_runtime_apis;
use sp_runtime::{
	traits::{MaybeDisplay, MaybeFromStr},
	RuntimeDebug,
};
use sp_std::prelude::*;

darwinia_support::impl_runtime_dispatch_info! {
	struct RuntimeDispatchInfo<Hash> {
		mmr_size: u64,
		proof: Proof<Hash>
	}
}

decl_runtime_apis! {
	pub trait HeaderMMRApi<BlockNumber, Hash>
	where
		BlockNumber: Codec,
		Hash: Codec + MaybeDisplay + MaybeFromStr,
	{
		fn gen_proof(
			block_number: BlockNumber,
			mmr_block_number: BlockNumber,
		) -> RuntimeDispatchInfo<Hash>;
	}
}

#[derive(Default, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct Proof<Hash>(pub Vec<Hash>);
#[cfg(feature = "std")]
impl<Hash> std::fmt::Display for Proof<Hash>
where
	Hash: std::fmt::Display,
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"{}",
			self.0
				.iter()
				.map(|hash| hash.to_string())
				.collect::<Vec<String>>()
				.join(",")
		)
	}
}
#[cfg(feature = "std")]
impl<Hash> std::str::FromStr for Proof<Hash>
where
	Hash: std::str::FromStr,
{
	type Err = <Hash as std::str::FromStr>::Err;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(Self(
			s.split(',')
				.map(|s| Hash::from_str(s.trim()))
				.collect::<Result<Vec<Hash>, Self::Err>>()?,
		))
	}
}
