// --- paritytech ---
use pallet_preimage::Config;
// --- darwinia-network ---
use crate::*;
use drml_primitives::*;

frame_support::parameter_types! {
	pub const PreimageMaxSize: u32 = 4096 * 1024;
	pub const PreimageBaseDeposit: Balance = COIN;
	// One cent: $10,000 / MB
	pub const PreimageByteDeposit: Balance = 10 * MILLI;
}

impl Config for Runtime {
	type BaseDeposit = PreimageBaseDeposit;
	type ByteDeposit = PreimageByteDeposit;
	type Currency = Balances;
	type Event = Event;
	type ManagerOrigin = Root;
	type MaxSize = PreimageMaxSize;
	type WeightInfo = ();
}
