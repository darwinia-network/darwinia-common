// --- paritytech ---
use pallet_vesting::Config;
use sp_runtime::traits::ConvertInto;
// --- darwinia-network ---
use crate::*;

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
