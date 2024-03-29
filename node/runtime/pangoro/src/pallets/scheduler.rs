// --- paritytech ---
use frame_support::{traits::EqualPrivilegeOnly, weights::Weight};
use pallet_scheduler::Config;
use sp_runtime::Perbill;
// --- darwinia-network ---
use crate::*;

frame_support::parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80)
		* RuntimeBlockWeights::get().max_block;
	pub const MaxScheduledPerBlock: u32 = 50;
	// Retry a scheduled item every 10 blocks (1 minute) until the preimage exists.
	pub const NoPreimagePostponement: Option<u32> = Some(10);
}

impl Config for Runtime {
	type Call = Call;
	type Event = Event;
	type MaxScheduledPerBlock = MaxScheduledPerBlock;
	type MaximumWeight = MaximumSchedulerWeight;
	type NoPreimagePostponement = NoPreimagePostponement;
	type Origin = Origin;
	type OriginPrivilegeCmp = EqualPrivilegeOnly;
	type PalletsOrigin = OriginCaller;
	type PreimageProvider = Preimage;
	type ScheduleOrigin = Root;
	type WeightInfo = ();
}
