// --- paritytech ---
use frame_support::weights::Weight;
use pallet_scheduler::Config;
use sp_runtime::Perbill;
// --- darwinia-network ---
use crate::*;

frame_support::parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80)
		* RuntimeBlockWeights::get().max_block;
	pub const MaxScheduledPerBlock: u32 = 50;
}

impl Config for Runtime {
	type Event = Event;
	type Origin = Origin;
	type PalletsOrigin = OriginCaller;
	type Call = Call;
	type MaximumWeight = MaximumSchedulerWeight;
	type ScheduleOrigin = RootOrigin;
	type MaxScheduledPerBlock = MaxScheduledPerBlock;
	type WeightInfo = ();
}
