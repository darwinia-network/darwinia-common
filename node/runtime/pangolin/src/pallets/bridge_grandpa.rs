pub use pallet_bridge_grandpa::{Instance1 as WithPangoroGrandpa, Instance2 as WithRococoGrandpa, Intance3 as WithMoonbaseRelayGrandpa};

// --- paritytech ---
use pallet_bridge_grandpa::Config;
use weights::{
	pallet_bridge_grandpa_bridge_pangoro_grandpa::WeightInfo as PangoroGrandpaWeightInfo,
	pallet_bridge_grandpa_bridge_rococo_grandpa::WeightInfo as RococoGrandpaWeightInfo,
};
// --- darwinia-network ---
use crate::*;

frame_support::parameter_types! {
	// This is a pretty unscientific cap.
	//
	// Note that once this is hit the pallet will essentially throttle incoming requests down to one
	// call per block.
	pub const MaxRequests: u32 = 50;
	// Number of headers to keep.
	//
	// Assuming the worst case of every header being finalized, we will keep headers for at least a
	// week.
	pub const PangoroHeadersToKeep: u32 = 7 * bp_pangoro::DAYS as u32;
	pub const RococoHeadersToKeep: u32 = 7 * bp_rococo::DAYS as u32;
}

impl Config<WithPangoroGrandpa> for Runtime {
	type BridgedChain = bp_pangoro::Pangoro;
	type HeadersToKeep = PangoroHeadersToKeep;
	type MaxRequests = MaxRequests;
	type WeightInfo = PangoroGrandpaWeightInfo<Self>;
}
impl Config<WithRococoGrandpa> for Runtime {
	type BridgedChain = bp_rococo::Rococo;
	type HeadersToKeep = RococoHeadersToKeep;
	type MaxRequests = MaxRequests;
	type WeightInfo = RococoGrandpaWeightInfo<Self>;
}
impl Config<WithMoonbaseRelayGrandpa> for Runtime {
	type BridgedChain = bp_rococo::Rococo;
	type HeadersToKeep = RococoHeadersToKeep;
	type MaxRequests = MaxRequests;
	type WeightInfo = RococoGrandpaWeightInfo<Self>;
}
