pub use pallet_bridge_grandpa::Instance2 as WithRococoGrandpa;

// --- paritytech ---
use drml_bridge_primitives::Pangoro;
use pallet_bridge_grandpa::Config;
// --- darwinia-network ---
use crate::*;
use pangoro_primitives::DAYS;

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
	pub const HeadersToKeep: u32 = 7 * DAYS as u32;
}

impl Config<WithRococoGrandpa> for Runtime {
	type BridgedChain = Pangoro;
	type MaxRequests = MaxRequests;
	type HeadersToKeep = HeadersToKeep;
	type WeightInfo = ();
}
