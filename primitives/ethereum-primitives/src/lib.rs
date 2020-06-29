#![recursion_limit = "128"]
#![cfg_attr(not(feature = "std"), no_std)]

#[macro_use]
pub extern crate rlp_derive;

pub mod encoded;
pub mod error;
pub mod header;
pub mod pow;
pub mod receipt;

pub use ethbloom::{Bloom, Input as BloomInput};
pub use ethereum_types::H64;
pub use primitive_types::{H160, H256, U128, U256, U512};

use sp_std::prelude::*;

pub type Bytes = Vec<u8>;
pub type EthAddress = H160;
pub type EthBlockNumber = u64;
