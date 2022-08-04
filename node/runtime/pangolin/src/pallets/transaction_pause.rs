// --- darwinia-network ---
use crate::*;
use module_transaction_pause::Config;

impl Config for Runtime {
	type Event = Event;
	type UpdateOrigin = RootOrAtLeastThreeFifth<CouncilCollective>;
	type WeightInfo = ();
}
