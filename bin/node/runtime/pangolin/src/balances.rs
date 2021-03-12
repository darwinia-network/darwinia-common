// --- darwinia ---
pub use darwinia_balances::{Instance0 as RingInstance, Instance1 as KtonInstance};

// --- substrate ---
use frame_support::traits::Currency;
use frame_system::Config as SystemConfig;
// --- darwinia ---
use crate::*;
use darwinia_balances::{weights::SubstrateWeight, Config, Module};

pub type NegativeImbalance = <Module<Runtime, RingInstance> as Currency<
	<Runtime as SystemConfig>::AccountId,
>>::NegativeImbalance;

frame_support::parameter_types! {
	pub const ExistentialDeposit: Balance = 1 * COIN;
	pub const MaxLocks: u32 = 50;
}
impl Config<RingInstance> for Runtime {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type BalanceInfo = AccountData<Balance>;
	type AccountStore = System;
	type MaxLocks = MaxLocks;
	type OtherCurrencies = (Kton,);
	type WeightInfo = SubstrateWeight<Runtime>;
}
impl Config<KtonInstance> for Runtime {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type BalanceInfo = AccountData<Balance>;
	type AccountStore = System;
	type MaxLocks = MaxLocks;
	type OtherCurrencies = (Ring,);
	type WeightInfo = SubstrateWeight<Runtime>;
}
