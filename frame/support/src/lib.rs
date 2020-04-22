#![cfg_attr(not(feature = "std"), no_std)]

pub mod macros;
pub mod structs;
pub mod traits;

pub mod balance {
	pub mod lock {
		// --- darwinia ---
		pub use crate::structs::{BalanceLock, LockFor, LockReasons, StakingLock, Unbonding};
		pub use crate::traits::{
			LockIdentifier, LockableCurrency, VestingSchedule, WithdrawReason, WithdrawReasons,
		};
	}

	// --- darwinia ---
	pub use crate::structs::FrozenBalance;
	pub use crate::traits::{BalanceInfo, DustCollector};
}

pub mod bytes_thing {
	// --- darwinia ---
	pub use crate::{array_unchecked, fixed_hex_bytes_unchecked};

	// --- substrate ---
	use sp_std::prelude::*;

	/// convert number to bytes base on radix `n`
	pub fn base_n_bytes_unchecked(mut x: u64, radix: u64) -> Vec<u8> {
		if x == 0 {
			return vec![b'0'];
		}

		if radix > 36 {
			return vec![];
		}

		let mut buf = vec![];
		while x != 0 {
			let rem = (x % radix) as u8;
			if rem < 10 {
				buf.push(b'0' + rem);
			} else {
				buf.push(b'a' + rem - 10);
			}
			x /= radix;
		}

		buf.reverse();
		buf
	}

	/// convert bytes to hex string
	pub fn hex_string_unchecked<B: AsRef<[u8]>>(b: B, prefix: &str) -> Vec<char> {
		let b = b.as_ref();
		let mut v = Vec::with_capacity(prefix.len() + b.len() * 2);

		for x in prefix.chars() {
			v.push(x);
		}

		for x in b.iter() {
			v.push(core::char::from_digit((x >> 4) as _, 16).unwrap_or_default());
			v.push(core::char::from_digit((x & 0xf) as _, 16).unwrap_or_default());
		}

		v
	}

	/// convert hex string to bytes
	pub fn hex_bytes_unchecked<S: AsRef<str>>(s: S) -> Vec<u8> {
		let s = s.as_ref();
		(if s.starts_with("0x") { 2 } else { 0 }..s.len())
			.step_by(2)
			.map(|i| u8::from_str_radix(&s[i..i + 2], 16))
			.collect::<Result<Vec<u8>, _>>()
			.unwrap_or_default()
	}
}
