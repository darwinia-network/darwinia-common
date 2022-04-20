pub use pallet_bridge_grandpa::{Instance1 as WithPangoroGrandpa, Instance2 as WithRococoGrandpa};

// --- paritytech ---
use pallet_bridge_grandpa::Config;
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
	pub const HeadersToKeep: u32 = 7 * bp_pangoro::DAYS as u32;
}

impl Config<WithPangoroGrandpa> for Runtime {
	type BridgedChain = bp_pangoro::Pangoro;
	type MaxRequests = MaxRequests;
	type HeadersToKeep = HeadersToKeep;
	type WeightInfo = ();
}
impl Config<WithRococoGrandpa> for Runtime {
	type BridgedChain = bp_rococo::Rococo;
	type MaxRequests = MaxRequests;
	type HeadersToKeep = HeadersToKeep;
	type WeightInfo = ();
}
