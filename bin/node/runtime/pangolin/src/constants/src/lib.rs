#![cfg_attr(not(feature = "std"), no_std)]

// --- substrate ---
use sp_staking::SessionIndex;
// --- darwinia ---
use drml_primitives::*;

pub const NANO: Balance = 1;
pub const MICRO: Balance = 1_000 * NANO;
pub const MILLI: Balance = 1_000 * MICRO;
pub const COIN: Balance = 1_000 * MILLI;

pub const CAP: Balance = 10_000_000_000 * COIN;
pub const TOTAL_POWER: Power = 1_000_000_000;

// Time is measured by number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = 60 * MINUTES;
pub const DAYS: BlockNumber = 24 * HOURS;

pub const MILLISECS_PER_BLOCK: Moment = 6000;
// NOTE: Currently it is not possible to change the slot duration after the chain has started.
//       Attempting to do so will brick block production.
pub const SLOT_DURATION: Moment = MILLISECS_PER_BLOCK;
// NOTE: Currently it is not possible to change the epoch duration after the chain has started.
//       Attempting to do so will brick block production.
pub const BLOCKS_PER_SESSION: BlockNumber = 10 * MINUTES;
pub const SESSIONS_PER_ERA: SessionIndex = 3;

// 1 in 4 blocks (on average, not counting collisions) will be primary babe blocks.
pub const PRIMARY_PROBABILITY: (u64, u64) = (1, 4);

pub const fn deposit(items: u32, bytes: u32) -> Balance {
	items as Balance * 20 * COIN + (bytes as Balance) * 100 * MICRO
}
