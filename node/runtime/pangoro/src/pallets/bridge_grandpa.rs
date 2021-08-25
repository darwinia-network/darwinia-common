use crate::*;

parameter_types! {
	// This is a pretty unscientific cap.
	//
	// Note that once this is hit the pallet will essentially throttle incoming requests down to one
	// call per block.
	pub const MaxRequests: u32 = 50;
	// Number of headers to keep.
	//
	// Assuming the worst case of every header being finalized, we will keep headers for at least a
	// week.
	pub const HeadersToKeep: u32 = 7 * pangoro_constants::DAYS as u32;
}
pub type WithPangolinGrandpa = pallet_bridge_grandpa::Instance1;
impl pallet_bridge_grandpa::Config<WithPangolinGrandpa> for Runtime {
	type BridgedChain = bridge_primitives::Pangolin;
	type MaxRequests = MaxRequests;
	type HeadersToKeep = HeadersToKeep;
	// FIXME
	type WeightInfo = pallet_bridge_grandpa::weights::RialtoWeight<Runtime>;
}
