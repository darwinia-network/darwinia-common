// --- paritytech ---
use frame_support::weights::Weight;
// --- darwinia-network ---
use crate::*;

pub fn pre_migrate<T: Config>() -> Result<(), &'static str> {
	Ok(())
}

pub fn migrate<T: Config>() -> Weight {
	0
}
