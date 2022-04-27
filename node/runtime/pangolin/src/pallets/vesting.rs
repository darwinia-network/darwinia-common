// --- paritytech ---
use sp_runtime::traits::ConvertInto;
// --- darwinia-network ---
use crate::*;
use darwinia_vesting::Config;

frame_support::parameter_types! {
	pub const MinVestedTransfer: Balance = 100 * MILLI;
}

impl Config for Runtime {
	type BlockNumberToBalance = ConvertInto;
	type Currency = Ring;
	type Event = Event;
	type MinVestedTransfer = MinVestedTransfer;
	type WeightInfo = ();

	const MAX_VESTING_SCHEDULES: u32 = 28;
}
