// --- darwinia-network ---
pub use darwinia_balances::{Instance1 as RingInstance, Instance2 as KtonInstance};

// --- darwinia-network ---
use crate::*;
use darwinia_balances::Config;

// TODO: https://github.com/paritytech/substrate/blob/master/frame/balances/src/benchmarking.rs#L43
#[cfg(feature = "runtime-benchmarks")]
frame_support::parameter_types! {
	pub const ExistentialDeposit: Balance = 1;
}
#[cfg(not(feature = "runtime-benchmarks"))]
frame_support::parameter_types! {
	pub const ExistentialDeposit: Balance = 0;
}
frame_support::parameter_types! {
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
}

impl Config<RingInstance> for Runtime {
	type AccountStore = System;
	type Balance = Balance;
	type BalanceInfo = AccountData<Balance>;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type MaxLocks = MaxLocks;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
}
impl Config<KtonInstance> for Runtime {
	type AccountStore = System;
	type Balance = Balance;
	type BalanceInfo = AccountData<Balance>;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type MaxLocks = MaxLocks;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
}
