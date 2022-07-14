// --- paritytech ---
use pallet_utility::Config;
// --- darwinia-network ---
use crate::{weights::pallet_utility::WeightInfo, *};

impl Config for Runtime {
	type Call = Call;
	type Event = Event;
	type WeightInfo = WeightInfo<Runtime>;
}
