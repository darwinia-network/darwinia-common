// --- paritytech ---
use pallet_vesting::Config;
use sp_runtime::traits::ConvertInto;
// --- darwinia-network ---
use crate::{weights::pallet_vesting::WeightInfo, *};

frame_support::parameter_types! {
	pub const MinVestedTransfer: Balance = COIN;
}

impl Config for Runtime {
	type BlockNumberToBalance = ConvertInto;
	type Currency = Ring;
	type Event = Event;
	type MinVestedTransfer = MinVestedTransfer;
	type WeightInfo = WeightInfo<Runtime>;

	const MAX_VESTING_SCHEDULES: u32 = 28;
}
