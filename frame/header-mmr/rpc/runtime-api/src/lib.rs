//! Runtime API definition required by header-mmr RPC extensions.
//!
//! This API should be imported and implemented by the runtime,
//! of a node that wants to use the custom RPC extension
//! adding header-mmr access methods.

#![cfg_attr(not(feature = "std"), no_std)]

// -- std ---
use core::fmt::Debug;
// --- crates ---
use codec::{Codec, Decode, Encode};
// --- substrate ---
use sp_api::decl_runtime_apis;
use sp_runtime::traits::{MaybeDisplay, MaybeFromStr};
use sp_std::prelude::*;
// --- darwinia ---
use darwinia_support::impl_runtime_dispatch_info;

impl_runtime_dispatch_info! {
	struct RuntimeDispatchInfo<Hash> {
		mmr_size: u64,
		proof: Proof<Hash>
	}

	fn custom_serializer() -> closure {
		|t| {
			let s = format!("{:?}", t);
			if s.len() > 6 {
				(&s[6..s.len() - 1]).to_owned()
			} else {
				s
			}
		}
	}
}

decl_runtime_apis! {
	pub trait HeaderMMRApi<Hash>
	where
		Hash: Debug + Codec + MaybeDisplay + MaybeFromStr,
	{
		fn gen_proof(
			block_number_of_member_leaf: u64,
			block_number_of_last_leaf: u64,
		) -> RuntimeDispatchInfo<Hash>;
	}
}

#[derive(Debug, Default, Eq, PartialEq, Encode, Decode)]
pub struct Proof<Hash>(pub Vec<Hash>)
where
	Hash: Debug;
