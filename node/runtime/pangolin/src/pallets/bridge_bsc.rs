// --- paritytech ---
use sp_core::U256;
// --- darwinia-network ---
use crate::*;
use darwinia_bridge_bsc::{BSCConfiguration, Config};

frame_support::parameter_types! {
	pub Configuration: BSCConfiguration = BSCConfiguration {
		chain_id: 56,
		min_gas_limit: 0x1388.into(),
		max_gas_limit: U256::max_value(),
		period: 0x03,
		epoch_length: 0xc8,
	};
}
impl Config for Runtime {
	type Event = Event;
	type BSCConfiguration = Configuration;
	type UnixTime = Timestamp;
	type OnHeadersSubmitted = ();
	type WeightInfo = ();
}
