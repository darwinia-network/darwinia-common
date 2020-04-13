#![cfg_attr(not(feature = "std"), no_std)]

pub mod macros;
pub mod structs;
pub mod traits;

pub mod balance {
	pub mod lock {
		// --- darwinia ---
		pub use structs::{BalanceLock, LockFor, LockReasons, StakingLock, Unbonding};
		pub use traits::{
			LockIdentifier, LockableCurrency, VestingSchedule, WithdrawReason, WithdrawReasons,
		};

		// --- darwinia ---
		use crate::*;
	}

	// --- darwinia ---
	pub use structs::FrozenBalance;
	pub use traits::{BalanceInfo, DustCollector};

	// --- darwinia ---
	use crate::*;
}

// --- substrate ---
use sp_std::prelude::*;

/// convert hex string to byte array
pub fn hex_bytes_unchecked(s: &str) -> Vec<u8> {
	(if s.starts_with("0x") { 2 } else { 0 }..s.len())
		.step_by(2)
		.map(|i| u8::from_str_radix(&s[i..i + 2], 16))
		.collect::<Result<Vec<u8>, _>>()
		.unwrap_or_default()
}
