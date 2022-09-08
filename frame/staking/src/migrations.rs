// --- paritytech ---
use frame_support::{traits::Get, weights::Weight};
use sp_runtime::traits::Zero;
// --- darwinia-network ---
use crate::*;

pub fn pre_migrate<T: Config>() -> Result<(), &'static str> {
	Ok(())
}

pub fn migrate<T: Config>() -> Weight {
	0
}
