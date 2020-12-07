#![cfg_attr(not(feature = "std"), no_std)]

pub mod macros;
pub mod structs;
pub mod testing;
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
	pub use crate::traits::{BalanceInfo, DustCollector, OnUnbalancedKton};
}

pub mod literal_procesor {
	/// Extract the inner value from json str with specific field
	pub fn extract_from_json_str<'a>(
		json_str: &'a [u8],
		field_name: &'static [u8],
	) -> Option<&'a [u8]> {
		let mut start = 0;
		let mut open_part_count = 0;
		let mut open_part = b'\0';
		let mut close_part = b'\0';
		let field_length = field_name.len();
		let mut match_pos = 0;
		let mut has_colon = false;
		for i in 0..json_str.len() {
			if open_part_count > 0 {
				if json_str[i] == close_part {
					open_part_count -= 1;
					if 0 == open_part_count {
						return Some(&json_str[start + 1..i]);
					}
				} else if json_str[i] == open_part {
					open_part_count += 1;
				}
			} else if has_colon {
				if json_str[i] == b'"' || json_str[i] == b'[' || json_str[i] == b'{' {
					start = i;
					open_part_count += 1;
					open_part = json_str[i];
					close_part = match json_str[i] {
						b'"' => b'"',
						b'[' => b']',
						b'{' => b'}',
						_ => panic!("never here"),
					}
				}
			} else if match_pos > 0 && i > match_pos {
				if json_str[i] == b':' {
					has_colon = true;
				}
			} else if json_str[i] == field_name[0]
				&& (json_str.len() - i) >= field_length
				&& json_str[i..i + field_length] == *field_name
			{
				match_pos = i + field_length;
			}
		}
		None
	}
}

#[cfg(test)]
mod tests;
