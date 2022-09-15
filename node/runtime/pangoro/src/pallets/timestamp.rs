// --- paritytech ---
use pallet_timestamp::Config;
// --- darwinia-network ---
use crate::{weights::pallet_timestamp::WeightInfo, *};

frame_support::parameter_types! {
	pub const MinimumPeriod: Moment = SLOT_DURATION / 2;
}

impl Config for Runtime {
	type MinimumPeriod = MinimumPeriod;
	type Moment = Moment;
	type OnTimestampSet = Babe;
	type WeightInfo = WeightInfo<Self>;
}
