// --- substrate ---
use pallet_utility::Config;
// --- darwinia ---
use crate::*;

impl Config for Runtime {
	type Event = Event;
	type Call = Call;
	type WeightInfo = ();
}
